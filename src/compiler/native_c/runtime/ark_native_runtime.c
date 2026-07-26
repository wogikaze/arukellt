#define _POSIX_C_SOURCE 200809L
#include "ark_native_runtime.h"

#include <ctype.h>
#include <errno.h>
#include <inttypes.h>
#include <limits.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <malloc.h>
#include <string.h>
#include <time.h>

/* ark_rt_trap is defined later; table helpers need it early. */
void ark_rt_trap(void);
static void ark_gc_page_inc(const void *pointer);
static void ark_gc_page_dec(const void *pointer);

typedef struct ark_arena_chunk {
    struct ark_arena_chunk *next;
    void *allocation;
    uint8_t *data;
    size_t used;
    size_t capacity;
} ark_arena_chunk;

typedef struct ark_gc_frame {
    struct ark_gc_frame *parent;
    size_t slot_count;
    size_t slot_capacity;
    ark_object_header ***slots;
} ark_gc_frame;

static int ark_gc_mode;
static ark_arena_chunk *ark_arena_head;
static ark_gc_allocation *ark_gc_heap_head;
static ark_gc_allocation *ark_gc_free_list;
#define ARK_GC_SIZE_CLASS_COUNT 20
static ark_gc_allocation *ark_gc_size_free[ARK_GC_SIZE_CLASS_COUNT];
static uint64_t ark_gc_size_free_bytes;
static uint64_t ark_gc_size_free_limit_bytes;
static uint64_t ark_gc_rss_soft_limit_bytes;
static uint64_t ark_gc_committed_soft_limit_bytes;
static ark_gc_frame *ark_gc_frame_top;
static ark_gc_frame *ark_gc_frame_free;
static ark_gc_allocation **ark_gc_object_table;
static size_t ark_gc_object_table_cap;
static size_t ark_gc_object_table_len;
typedef struct {
    uintptr_t key; /* 0 empty, UINTPTR_MAX tomb, else addr>>12 */
    uint32_t count;
} ark_gc_page_slot;
#define ARK_GC_PAGE_EMPTY ((uintptr_t)0)
#define ARK_GC_PAGE_TOMB ((uintptr_t)UINTPTR_MAX)
static ark_gc_page_slot *ark_gc_page_map;
static size_t ark_gc_page_map_cap;
static size_t ark_gc_page_map_len;
static size_t ark_gc_page_map_tombs;
static uint64_t ark_gc_page_map_misses;
/* Non-zero epoch; objects with mark==epoch are black. Avoids O(heap) clear. */
static uint8_t ark_gc_mark_epoch;
static size_t ark_gc_heap_len;
static uint64_t ark_requested_bytes;
static uint64_t ark_committed_bytes;
static uint64_t ark_live_bytes;
static uint64_t ark_collection_count;
static uint64_t ark_reclaimed_bytes;
static uint64_t ark_reclaimed_object_bytes;
static uint64_t ark_reclaimed_side_buffer_bytes;
static uint64_t ark_gc_threshold_bytes;
static uint64_t ark_gc_threshold_override;
static uint64_t ark_gc_bytes_since_collection;
static uint64_t ark_gc_object_bytes;
static uint64_t ark_gc_string_buffer_bytes;
static uint64_t ark_gc_vec_buffer_bytes;
static uint64_t ark_gc_root_frame_bytes;
static uint64_t ark_gc_total_mark_time_ns;
static uint64_t ark_gc_total_sweep_time_ns;
static uint64_t ark_gc_total_table_rebuild_time_ns;
static uint64_t ark_gc_total_malloc_trim_time_ns;
static uint64_t ark_gc_total_root_scan_time_ns;
static uint64_t ark_gc_total_marked_objects;
static uint64_t ark_gc_max_marked_objects_per_collection;
static uint64_t ark_gc_max_root_slots_scanned;
static uint64_t ark_gc_max_heap_objects_before_collection;
static uint64_t ark_gc_max_heap_objects_after_collection;
static uint64_t ark_gc_mark_stack_peak;
static ark_object_header **ark_gc_mark_stack;
static size_t ark_gc_mark_stack_cap;
static uint32_t ark_chunk_count;
static ark_vec *ark_process_args;
static int ark_gc_collecting;
static const char *ark_gc_current_function;

static int ark_env_gc_enabled(void) {
    const char *enable = getenv("ARUKELLT_NATIVE_GC");
    /* Public run defaults to GC on (ADR-050). The selfhost native executor
       always sets ARUKELLT_NATIVE_GC explicitly (0 or 1). */
    if (enable == NULL) return 1;
    return enable[0] == '1';
}

static int ark_env_debug_gc_dump(void) {
    const char *flag = getenv("ARUKELLT_NATIVE_GC_DEBUG_DUMP");
    return flag != NULL && flag[0] == '1';
}

static uint64_t ark_env_u64(const char *name, uint64_t fallback) {
    const char *raw = getenv(name);
    if (raw == NULL || raw[0] == '\0') return fallback;
    char *end = NULL;
    errno = 0;
    unsigned long long value = strtoull(raw, &end, 10);
    if (errno != 0 || end == raw) return fallback;
    return (uint64_t)value;
}

void ark_gc_set_current_function(const char *name) {
    ark_gc_current_function = name;
}

static void ark_gc_dump_crash_state(const char *reason) {
    fprintf(
        stderr,
        "native-cpp GC diagnostic: %s\n"
        "  collection=%" PRIu64 " function=%s live_objects=%zu table_cap=%zu\n"
        "  object_bytes=%" PRIu64 " string_buf=%" PRIu64 " vec_buf=%" PRIu64 "\n"
        "  root_frame_bytes=%" PRIu64 " reclaimed_object=%" PRIu64 " reclaimed_side=%" PRIu64 "\n",
        reason,
        ark_collection_count,
        ark_gc_current_function != NULL ? ark_gc_current_function : "(unknown)",
        ark_gc_object_table_len,
        ark_gc_object_table_cap,
        ark_gc_object_bytes,
        ark_gc_string_buffer_bytes,
        ark_gc_vec_buffer_bytes,
        ark_gc_root_frame_bytes,
        ark_reclaimed_object_bytes,
        ark_reclaimed_side_buffer_bytes
    );
}

static uint64_t ark_gc_measure_root_frame_bytes(void);

static uint64_t ark_gc_now_ns(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) return 0;
    return (uint64_t)ts.tv_sec * 1000000000ull + (uint64_t)ts.tv_nsec;
}

static void ark_gc_write_stats_file(void) {
    const char *path = getenv("ARUKELLT_NATIVE_GC_STATS_PATH");
    if (path == NULL || path[0] == '\0') return;
    FILE *out = fopen(path, "w");
    if (out == NULL) return;
    uint64_t table_bytes =
        (uint64_t)ark_gc_object_table_cap * (uint64_t)sizeof(ark_gc_allocation *) +
        (uint64_t)ark_gc_page_map_cap * (uint64_t)sizeof(ark_gc_page_slot);
    ark_gc_root_frame_bytes = ark_gc_measure_root_frame_bytes();
    fprintf(
        out,
        "{\n"
        "  \"gc_object_bytes\": %" PRIu64 ",\n"
        "  \"gc_string_buffer_bytes\": %" PRIu64 ",\n"
        "  \"gc_vec_buffer_bytes\": %" PRIu64 ",\n"
        "  \"gc_object_table_bytes\": %" PRIu64 ",\n"
        "  \"gc_root_frame_bytes\": %" PRIu64 ",\n"
        "  \"gc_live_object_count\": %zu,\n"
        "  \"gc_object_table_capacity\": %zu,\n"
        "  \"gc_collection_count\": %" PRIu64 ",\n"
        "  \"gc_reclaimed_object_bytes\": %" PRIu64 ",\n"
        "  \"gc_reclaimed_side_buffer_bytes\": %" PRIu64 ",\n"
        "  \"gc_total_mark_time_ms\": %" PRIu64 ",\n"
        "  \"gc_total_sweep_time_ms\": %" PRIu64 ",\n"
        "  \"gc_total_table_rebuild_time_ms\": %" PRIu64 ",\n"
        "  \"gc_total_malloc_trim_time_ms\": %" PRIu64 ",\n"
        "  \"gc_total_root_scan_time_ms\": %" PRIu64 ",\n"
        "  \"gc_total_marked_objects\": %" PRIu64 ",\n"
        "  \"gc_max_marked_objects_per_collection\": %" PRIu64 ",\n"
        "  \"gc_max_root_slots_scanned\": %" PRIu64 ",\n"
        "  \"gc_max_heap_objects_before_collection\": %" PRIu64 ",\n"
        "  \"gc_max_heap_objects_after_collection\": %" PRIu64 ",\n"
        "  \"gc_threshold_bytes\": %" PRIu64 ",\n"
        "  \"gc_mark_stack_peak\": %" PRIu64 ",\n"
        "  \"runtime_requested_bytes\": %" PRIu64 ",\n"
        "  \"runtime_committed_bytes\": %" PRIu64 ",\n"
        "  \"runtime_live_bytes\": %" PRIu64 ",\n"
        "  \"runtime_collection_count\": %" PRIu64 ",\n"
        "  \"runtime_reclaimed_bytes\": %" PRIu64 "\n"
        "}\n",
        ark_gc_object_bytes,
        ark_gc_string_buffer_bytes,
        ark_gc_vec_buffer_bytes,
        table_bytes,
        ark_gc_root_frame_bytes,
        ark_gc_object_table_len,
        ark_gc_object_table_cap,
        ark_collection_count,
        ark_reclaimed_object_bytes,
        ark_reclaimed_side_buffer_bytes,
        (uint64_t)(ark_gc_total_mark_time_ns / 1000000ull),
        (uint64_t)(ark_gc_total_sweep_time_ns / 1000000ull),
        (uint64_t)(ark_gc_total_table_rebuild_time_ns / 1000000ull),
        (uint64_t)(ark_gc_total_malloc_trim_time_ns / 1000000ull),
        (uint64_t)(ark_gc_total_root_scan_time_ns / 1000000ull),
        ark_gc_total_marked_objects,
        ark_gc_max_marked_objects_per_collection,
        ark_gc_max_root_slots_scanned,
        ark_gc_max_heap_objects_before_collection,
        ark_gc_max_heap_objects_after_collection,
        ark_gc_threshold_bytes,
        ark_gc_mark_stack_peak,
        ark_requested_bytes,
        ark_committed_bytes,
        ark_live_bytes,
        ark_collection_count,
        ark_reclaimed_bytes
    );
    fclose(out);
}

#define ARK_GC_INITIAL_THRESHOLD (128ull * 1024ull * 1024ull)
#define ARK_GC_KIND_RAW 0u
#define ARK_GC_KIND_STRING 1u
#define ARK_GC_KIND_VEC 2u
#define ARK_GC_KIND_STRUCT 3u
#define ARK_GC_SIZE_FREE_LIMIT_DEFAULT (8ull * 1024ull * 1024ull)
/* Soft budget for live heap buffers (leaves headroom under 2.4 GiB RSS).
 * Freelist bytes are capped separately and excluded from the soft trigger so
 * recycling does not force extra full-heap collections. */
#define ARK_GC_RSS_SOFT_LIMIT_DEFAULT (1160ull * 1024ull * 1024ull)
#define ARK_GC_COMMITTED_SOFT_LIMIT_DEFAULT (1520ull * 1024ull * 1024ull)
/* reserved[1..4] hold a little-endian magic used for membership without a hash table. */
#define ARK_GC_HEADER_MAGIC 0xA4C9D17Bu

static int ark_gc_size_class_for(size_t size) {
    size_t bucket = 16u;
    int cls = 0;
    while (bucket < size && cls + 1 < ARK_GC_SIZE_CLASS_COUNT) {
        bucket *= 2u;
        cls += 1;
    }
    return cls;
}

static size_t ark_gc_size_class_bytes(int cls) {
    size_t bucket = 16u;
    int i = 0;
    while (i < cls) {
        bucket *= 2u;
        i += 1;
    }
    return bucket;
}

static void ark_gc_size_free_release_excess(void) {
    while (ark_gc_size_free_bytes > ark_gc_size_free_limit_bytes) {
        int freed_any = 0;
        for (int cls = ARK_GC_SIZE_CLASS_COUNT - 1; cls >= 0; cls -= 1) {
            ark_gc_allocation *node = ark_gc_size_free[cls];
            if (node == NULL) continue;
            ark_gc_size_free[cls] = node->next;
            size_t total = sizeof(ark_gc_allocation) + node->allocation_size;
            if (ark_gc_size_free_bytes >= total) ark_gc_size_free_bytes -= total;
            else ark_gc_size_free_bytes = 0;
            if (ark_committed_bytes >= total) ark_committed_bytes -= total;
            ark_gc_page_dec(node);
            free(node);
            freed_any = 1;
            break;
        }
        if (!freed_any) break;
    }
}

static void ark_gc_size_free_push(ark_gc_allocation *node) {
    size_t total = sizeof(ark_gc_allocation) + node->allocation_size;
    /* Prefer recycling through the size-class freelist for mutator speed.
     * When the cap is already saturated, free immediately (O(1)) instead of
     * scanning release_excess() once per reclaimed object. */
    if (ark_gc_size_free_bytes + total > ark_gc_size_free_limit_bytes) {
        if (ark_committed_bytes >= total) ark_committed_bytes -= total;
        ark_gc_page_dec(node);
        free(node);
        return;
    }
    int cls = ark_gc_size_class_for(node->allocation_size);
    node->mark = 0;
    memset(node->reserved, 0, sizeof(node->reserved));
    /* Magic cleared with reserved[] so freelist nodes are not mistaken for live objects. */
    node->next = ark_gc_size_free[cls];
    ark_gc_size_free[cls] = node;
    ark_gc_size_free_bytes += total;
}

static size_t ark_gc_round_alloc_size(size_t size) {
    int cls = ark_gc_size_class_for(size);
    size_t rounded = ark_gc_size_class_bytes(cls);
    if (rounded < size) return size;
    return rounded;
}

static ark_gc_allocation *ark_gc_size_free_take(size_t size) {
    size_t rounded = ark_gc_round_alloc_size(size);
    int cls = ark_gc_size_class_for(rounded);
    if (ark_gc_size_class_bytes(cls) < rounded) return NULL;
    ark_gc_allocation *node = ark_gc_size_free[cls];
    if (node == NULL) return NULL;
    ark_gc_size_free[cls] = node->next;
    size_t total = sizeof(ark_gc_allocation) + node->allocation_size;
    if (ark_gc_size_free_bytes >= total) ark_gc_size_free_bytes -= total;
    else ark_gc_size_free_bytes = 0;
    node->allocation_size = rounded;
    node->next = NULL;
    return node;
}

static size_t ark_checked_add(size_t left, size_t right) {
    if (left > SIZE_MAX - right) ark_rt_trap_kind(ARK_TRAP_ALLOC);
    return left + right;
}

static size_t ark_checked_mul(size_t left, size_t right) {
    if (left != 0 && right > SIZE_MAX / left) ark_rt_trap_kind(ARK_TRAP_ALLOC);
    return left * right;
}

static ark_gc_allocation *ark_gc_header_from_object(void *object) {
    return ((ark_gc_allocation *)object) - 1;
}

static void *ark_gc_object_from_header(ark_gc_allocation *header) {
    return (void *)(header + 1);
}

static void ark_gc_header_set_magic(ark_gc_allocation *header) {
    uint32_t magic = ARK_GC_HEADER_MAGIC;
    memcpy(&header->reserved[1], &magic, sizeof(magic));
}

static int ark_gc_header_has_magic(const ark_gc_allocation *header) {
    uint32_t magic = 0;
    memcpy(&magic, &header->reserved[1], sizeof(magic));
    return magic == ARK_GC_HEADER_MAGIC;
}

static size_t ark_gc_hash_uptr(uintptr_t value) {
    value ^= value >> 30;
    value *= (uintptr_t)0xbf58476d1ce4e5b9ULL;
    value ^= value >> 27;
    return (size_t)value;
}

static uintptr_t ark_gc_page_key(const void *pointer) {
    return ((uintptr_t)pointer) >> 12;
}

static void ark_gc_page_rehash(size_t new_cap) {
    ark_gc_page_slot *fresh = calloc(new_cap, sizeof(*fresh));
    if (fresh == NULL) ark_rt_trap_kind(ARK_TRAP_OOM);
    size_t mask = new_cap - 1u;
    size_t len = 0;
    if (ark_gc_page_map != NULL) {
        for (size_t i = 0; i < ark_gc_page_map_cap; i += 1) {
            uintptr_t key = ark_gc_page_map[i].key;
            if (key == ARK_GC_PAGE_EMPTY || key == ARK_GC_PAGE_TOMB) continue;
            size_t slot = ark_gc_hash_uptr(key) & mask;
            while (fresh[slot].key != ARK_GC_PAGE_EMPTY) slot = (slot + 1u) & mask;
            fresh[slot].key = key;
            fresh[slot].count = ark_gc_page_map[i].count;
            len += 1;
        }
        free(ark_gc_page_map);
    }
    ark_gc_page_map = fresh;
    ark_gc_page_map_cap = new_cap;
    ark_gc_page_map_len = len;
    ark_gc_page_map_tombs = 0;
}

static void ark_gc_page_ensure_load(void) {
    size_t occupied = ark_gc_page_map_len + ark_gc_page_map_tombs;
    if (ark_gc_page_map_cap == 0) {
        ark_gc_page_rehash(1024u);
        return;
    }
    if ((occupied + 1u) * 2u <= ark_gc_page_map_cap) return;
    size_t next = ark_gc_page_map_cap * 2u;
    ark_gc_page_rehash(next);
}

static void ark_gc_page_inc(const void *pointer) {
    if (pointer == NULL) return;
    uintptr_t key = ark_gc_page_key(pointer);
    if (key == ARK_GC_PAGE_EMPTY) return;
    ark_gc_page_ensure_load();
    size_t mask = ark_gc_page_map_cap - 1u;
    size_t slot = ark_gc_hash_uptr(key) & mask;
    size_t tomb = (size_t)-1;
    for (;;) {
        uintptr_t present = ark_gc_page_map[slot].key;
        if (present == key) {
            ark_gc_page_map[slot].count += 1u;
            return;
        }
        if (present == ARK_GC_PAGE_EMPTY) {
            size_t dest = tomb != (size_t)-1 ? tomb : slot;
            if (ark_gc_page_map[dest].key == ARK_GC_PAGE_TOMB) ark_gc_page_map_tombs -= 1u;
            else ark_gc_page_map_len += 1u;
            ark_gc_page_map[dest].key = key;
            ark_gc_page_map[dest].count = 1u;
            return;
        }
        if (present == ARK_GC_PAGE_TOMB && tomb == (size_t)-1) tomb = slot;
        slot = (slot + 1u) & mask;
    }
}

static void ark_gc_page_dec(const void *pointer) {
    if (pointer == NULL || ark_gc_page_map_cap == 0) return;
    uintptr_t key = ark_gc_page_key(pointer);
    size_t mask = ark_gc_page_map_cap - 1u;
    size_t slot = ark_gc_hash_uptr(key) & mask;
    for (;;) {
        uintptr_t present = ark_gc_page_map[slot].key;
        if (present == ARK_GC_PAGE_EMPTY) return;
        if (present == key) {
            if (ark_gc_page_map[slot].count > 1u) {
                ark_gc_page_map[slot].count -= 1u;
                return;
            }
            ark_gc_page_map[slot].key = ARK_GC_PAGE_TOMB;
            ark_gc_page_map[slot].count = 0;
            ark_gc_page_map_len -= 1u;
            ark_gc_page_map_tombs += 1u;
            return;
        }
        slot = (slot + 1u) & mask;
    }
}

static int ark_gc_page_has(const void *pointer) {
    if (pointer == NULL || ark_gc_page_map_cap == 0) return 0;
    uintptr_t key = ark_gc_page_key(pointer);
    size_t mask = ark_gc_page_map_cap - 1u;
    size_t slot = ark_gc_hash_uptr(key) & mask;
    for (;;) {
        uintptr_t present = ark_gc_page_map[slot].key;
        if (present == ARK_GC_PAGE_EMPTY) return 0;
        if (present == key) return ark_gc_page_map[slot].count > 0u;
        slot = (slot + 1u) & mask;
    }
}

static size_t ark_gc_page_verify_heap(void) {
    size_t misses = 0;
    for (ark_gc_allocation *node = ark_gc_heap_head; node != NULL; node = node->next) {
        if (!ark_gc_page_has(node)) misses += 1u;
    }
    return misses;
}

static ark_gc_allocation *ark_gc_header_if_heap_object(void *object) {
    if (object == NULL) return NULL;
    uintptr_t address = (uintptr_t)object;
    /* Fresh allocations come from 16-byte-aligned blocks; reject obvious scalars. */
    if ((address & 7u) != 0u) return NULL;
    if (address < 4096u) return NULL;
    ark_gc_allocation *header = ark_gc_header_from_object(object);
    if (!ark_gc_header_has_magic(header)) return NULL;
    return header;
}

static size_t ark_gc_hash_ptr(const void *pointer) {
    uintptr_t value = (uintptr_t)pointer;
    value ^= value >> 30;
    value *= (uintptr_t)0xbf58476d1ce4e5b9ULL;
    value ^= value >> 27;
    return (size_t)value;
}

static void ark_gc_table_clear(void) {
    if (ark_gc_object_table != NULL && ark_gc_object_table_cap != 0) {
        memset(ark_gc_object_table, 0, ark_gc_object_table_cap * sizeof(*ark_gc_object_table));
    }
    ark_gc_object_table_len = 0;
}

static void ark_gc_table_ensure(size_t minimum_cap) {
    size_t cap = ark_gc_object_table_cap;
    if (cap == 0) cap = 1024u;
    while (cap < minimum_cap) {
        if (cap > (SIZE_MAX / 2u)) ark_rt_trap();
        cap *= 2u;
    }
    if (cap == ark_gc_object_table_cap) return;
    ark_gc_allocation **old = ark_gc_object_table;
    size_t old_cap = ark_gc_object_table_cap;
    ark_gc_allocation **fresh = calloc(cap, sizeof(*fresh));
    if (fresh == NULL) ark_rt_trap_kind(ARK_TRAP_OOM);
    ark_gc_object_table = fresh;
    ark_gc_object_table_cap = cap;
    ark_gc_object_table_len = 0;
    if (old != NULL) {
        for (size_t i = 0; i < old_cap; i += 1) {
            ark_gc_allocation *header = old[i];
            if (header == NULL) continue;
            void *object = ark_gc_object_from_header(header);
            size_t slot = ark_gc_hash_ptr(object) & (cap - 1u);
            while (ark_gc_object_table[slot] != NULL) {
                slot = (slot + 1u) & (cap - 1u);
            }
            ark_gc_object_table[slot] = header;
            ark_gc_object_table_len += 1;
        }
        free(old);
    }
}

static void ark_gc_table_insert(ark_gc_allocation *header) {
    if (header == NULL) return;
    if (ark_gc_object_table_cap == 0 ||
        (ark_gc_object_table_len + 1u) * 2u > ark_gc_object_table_cap) {
        size_t need = ark_gc_object_table_cap == 0 ? 1024u : ark_gc_object_table_cap * 2u;
        ark_gc_table_ensure(need);
    }
    void *object = ark_gc_object_from_header(header);
    size_t slot = ark_gc_hash_ptr(object) & (ark_gc_object_table_cap - 1u);
    while (ark_gc_object_table[slot] != NULL) {
        if (ark_gc_object_from_header(ark_gc_object_table[slot]) == object) return;
        slot = (slot + 1u) & (ark_gc_object_table_cap - 1u);
    }
    ark_gc_object_table[slot] = header;
    ark_gc_object_table_len += 1;
}

static ark_gc_allocation *ark_gc_table_find(void *object) {
    if (object == NULL) return NULL;
    uintptr_t address = (uintptr_t)object;
    if ((address & 7u) != 0u || address < 4096u) return NULL;
    ark_gc_allocation *header = ark_gc_header_from_object(object);
    if (ark_gc_page_has(header)) {
        if (ark_gc_header_has_magic(header)) return header;
        return NULL;
    }
    /* Fallback path when page-map is incomplete (should be rare). */
    if (ark_gc_object_table_cap == 0) return NULL;
    size_t mask = ark_gc_object_table_cap - 1u;
    size_t slot = ark_gc_hash_ptr(object) & mask;
    for (;;) {
        ark_gc_allocation *present = ark_gc_object_table[slot];
        if (present == NULL) return NULL;
        if (ark_gc_object_from_header(present) == object) return present;
        slot = (slot + 1u) & mask;
    }
}

static size_t ark_next_pow2_size(size_t value) {
    size_t cap = 1u;
    while (cap < value) {
        if (cap > (SIZE_MAX / 2u)) return SIZE_MAX;
        cap *= 2u;
    }
    return cap;
}

static void ark_gc_table_rebuild_from_heap_count(size_t count) {
    /* 5/4 load target (vs 2×) shrinks table rebuild/copy traffic. */
    size_t desired = count + (count / 4u);
    if (desired < 1024u) desired = 1024u;
    desired = ark_next_pow2_size(desired);
    if (ark_gc_object_table != NULL && ark_gc_object_table_cap == desired) {
        ark_gc_table_clear();
    } else {
        free(ark_gc_object_table);
        ark_gc_object_table = calloc(desired, sizeof(*ark_gc_object_table));
        if (ark_gc_object_table == NULL) ark_rt_trap_kind(ARK_TRAP_OOM);
        ark_gc_object_table_cap = desired;
    }
    ark_gc_object_table_len = 0;
    size_t mask = desired - 1u;
    for (ark_gc_allocation *node = ark_gc_heap_head; node != NULL; node = node->next) {
        void *object = ark_gc_object_from_header(node);
        size_t slot = ark_gc_hash_ptr(object) & mask;
        while (ark_gc_object_table[slot] != NULL) {
            slot = (slot + 1u) & mask;
        }
        ark_gc_object_table[slot] = node;
        ark_gc_object_table_len += 1;
    }
}

static void ark_gc_table_rebuild_from_heap(void) {
    size_t count = 0;
    for (ark_gc_allocation *node = ark_gc_heap_head; node != NULL; node = node->next) {
        count += 1;
    }
    ark_gc_table_rebuild_from_heap_count(count);
}

static void ark_gc_set_kind(void *object, uint8_t kind) {
    if (!ark_gc_mode || object == NULL) return;
    /* Kind lives on the allocation header; no hash-table lookup in the mutator. */
    ark_gc_header_from_object(object)->reserved[0] = kind;
}

static uint8_t ark_gc_kind(void *object) {
    if (!ark_gc_mode || object == NULL) return ARK_GC_KIND_RAW;
    return ark_gc_header_from_object(object)->reserved[0];
}

static void *ark_side_bytes_accounted(size_t size, uint64_t *counter) {
    if (ark_gc_mode) {
        void *bytes = malloc(size);
        if (bytes == NULL) {
            ark_gc_dump_crash_state("side-buffer malloc failed");
            ark_rt_trap();
        }
        *counter += size;
        return bytes;
    }
    return ark_rt_alloc_aligned(size, 16u);
}

static void *ark_side_bytes_string(size_t size) {
    return ark_side_bytes_accounted(size, &ark_gc_string_buffer_bytes);
}

static void *ark_side_bytes_vec(size_t size) {
    return ark_side_bytes_accounted(size, &ark_gc_vec_buffer_bytes);
}

static void ark_gc_mark_object(ark_object_header *object);

static void ark_gc_mark_value(ark_value value) {
    if (value.ref != NULL) ark_gc_mark_object(value.ref);
}

static void ark_gc_free_side_buffers(ark_object_header *object) {
    uint8_t kind = ark_gc_kind(object);
    if (kind == ARK_GC_KIND_STRING) {
        ark_string *string = (ark_string *)object;
        if (string->bytes != NULL) {
            uint64_t bytes = string->capacity;
            if (ark_gc_string_buffer_bytes >= bytes) ark_gc_string_buffer_bytes -= bytes;
            else ark_gc_string_buffer_bytes = 0;
            ark_reclaimed_side_buffer_bytes += bytes;
            free(string->bytes);
            string->bytes = NULL;
            string->capacity = 0;
        }
        return;
    }
    if (kind == ARK_GC_KIND_VEC) {
        ark_vec *vector = (ark_vec *)object;
        if (vector->data != NULL) {
            uint64_t bytes = (uint64_t)vector->capacity * (uint64_t)sizeof(ark_value);
            if (ark_gc_vec_buffer_bytes >= bytes) ark_gc_vec_buffer_bytes -= bytes;
            else ark_gc_vec_buffer_bytes = 0;
            ark_reclaimed_side_buffer_bytes += bytes;
            free(vector->data);
            vector->data = NULL;
            vector->capacity = 0;
        }
    }
}

static void ark_gc_mark_stack_ensure(size_t need) {
    if (need <= ark_gc_mark_stack_cap) return;
    size_t next = ark_gc_mark_stack_cap == 0 ? 4096u : ark_gc_mark_stack_cap;
    while (next < need) {
        if (next > (SIZE_MAX / 2u)) ark_rt_trap();
        next *= 2u;
    }
    ark_object_header **fresh = realloc(ark_gc_mark_stack, next * sizeof(*fresh));
    if (fresh == NULL) ark_rt_trap_kind(ARK_TRAP_OOM);
    ark_gc_mark_stack = fresh;
    ark_gc_mark_stack_cap = next;
}

static void ark_gc_mark_push(size_t *len, ark_object_header *object) {
    if (object == NULL) return;
    ark_gc_allocation *header = ark_gc_table_find(object);
    if (header == NULL || header->mark == ark_gc_mark_epoch) return;
    header->mark = ark_gc_mark_epoch;
    ark_gc_mark_stack_ensure(*len + 1u);
    ark_gc_mark_stack[*len] = object;
    *len += 1u;
    if ((uint64_t)(*len) > ark_gc_mark_stack_peak) {
        ark_gc_mark_stack_peak = (uint64_t)(*len);
    }
}

static void ark_gc_mark_drain(size_t *len) {
    while (*len > 0) {
        ark_object_header *current = ark_gc_mark_stack[*len - 1u];
        *len -= 1u;
        /* Object was marked at push time; kind lives on the allocation header. */
        ark_gc_allocation *header = ark_gc_header_from_object(current);
        uint8_t kind = header->reserved[0];
        if (kind == ARK_GC_KIND_STRING || kind == ARK_GC_KIND_RAW) {
            continue;
        }
        if (kind == ARK_GC_KIND_VEC) {
            ark_vec *vector = (ark_vec *)current;
            if (vector->data == NULL) continue;
            for (uint32_t i = 0; i < vector->length; i += 1) {
                ark_gc_mark_push(len, vector->data[i].ref);
            }
            continue;
        }
        if (kind == ARK_GC_KIND_STRUCT) {
            ark_struct_object *structure = (ark_struct_object *)current;
            for (uint32_t i = 0; i < structure->field_count; i += 1) {
                ark_gc_mark_push(len, structure->fields[i].ref);
            }
        }
    }
}

static void ark_gc_mark_object(ark_object_header *object) {
    size_t len = 0;
    ark_gc_mark_push(&len, object);
    ark_gc_mark_drain(&len);
}

static uint64_t ark_gc_mark_roots(void) {
    uint64_t slots_scanned = 0;
    size_t len = 0;
    for (ark_gc_frame *frame = ark_gc_frame_top; frame != NULL; frame = frame->parent) {
        for (size_t i = 0; i < frame->slot_count; i += 1) {
            slots_scanned += 1u;
            ark_object_header **slot = frame->slots[i];
            if (slot != NULL && *slot != NULL) {
                ark_gc_mark_push(&len, *slot);
            }
        }
    }
    if (ark_process_args != NULL) {
        slots_scanned += 1u;
        ark_gc_mark_push(&len, (ark_object_header *)ark_process_args);
    }
    ark_gc_mark_drain(&len);
    return slots_scanned;
}

static void ark_gc_table_maybe_shrink(size_t live_count) {
    /* Shrink policy is applied inside ark_gc_table_rebuild_from_heap. */
    (void)live_count;
}

void ark_gc_collect(void) {
    if (!ark_gc_mode || ark_gc_collecting) return;
    ark_gc_collecting = 1;
    ark_collection_count += 1;

    /* Advance mark epoch instead of clearing every object (was O(heap)). */
    ark_gc_mark_epoch = (uint8_t)(ark_gc_mark_epoch + 1u);
    if (ark_gc_mark_epoch == 0) {
        for (ark_gc_allocation *clear_node = ark_gc_heap_head;
             clear_node != NULL;
             clear_node = clear_node->next) {
            clear_node->mark = 0;
        }
        ark_gc_mark_epoch = 1;
    }
    size_t heap_before = ark_gc_heap_len;
    if ((uint64_t)heap_before > ark_gc_max_heap_objects_before_collection) {
        ark_gc_max_heap_objects_before_collection = (uint64_t)heap_before;
    }
    {
        const char *verify = getenv("ARUKELLT_NATIVE_GC_VERIFY_PAGE_MAP");
        size_t misses = 0;
        if (verify != NULL && verify[0] == '1') {
            misses = ark_gc_page_verify_heap();
            ark_gc_page_map_misses += (uint64_t)misses;
        }
        if (misses != 0) {
            uint64_t rebuild_started = ark_gc_now_ns();
            ark_gc_table_rebuild_from_heap_count(heap_before);
            ark_gc_total_table_rebuild_time_ns += ark_gc_now_ns() - rebuild_started;
            fprintf(
                stderr,
                "native-cpp GC: page-map miss=%zu heap=%zu; fell back to table rebuild\n",
                misses,
                heap_before
            );
        } else {
            /* Mark uses page-map+magic; object-table stays unused. */
            ark_gc_object_table_len = heap_before;
        }
    }

    uint64_t root_started = ark_gc_now_ns();
    uint64_t slots_scanned = ark_gc_mark_roots();
    uint64_t root_elapsed = ark_gc_now_ns() - root_started;
    ark_gc_total_root_scan_time_ns += root_elapsed;
    ark_gc_total_mark_time_ns += root_elapsed;
    if (slots_scanned > ark_gc_max_root_slots_scanned) {
        ark_gc_max_root_slots_scanned = slots_scanned;
    }

    uint64_t sweep_started = ark_gc_now_ns();
    ark_gc_allocation *live_head = NULL;
    uint64_t live = 0;
    uint64_t reclaimed = 0;
    size_t live_count = 0;
    ark_gc_allocation *node = ark_gc_heap_head;
    while (node != NULL) {
        ark_gc_allocation *next = node->next;
        if (node->mark == ark_gc_mark_epoch) {
            node->next = live_head;
            live_head = node;
            live += node->allocation_size;
            live_count += 1u;
        } else {
            ark_gc_free_side_buffers(
                (ark_object_header *)ark_gc_object_from_header(node)
            );
            reclaimed += node->allocation_size;
            ark_reclaimed_object_bytes += node->allocation_size;
            if (ark_gc_object_bytes >= node->allocation_size) {
                ark_gc_object_bytes -= node->allocation_size;
            } else {
                ark_gc_object_bytes = 0;
            }
            if (ark_chunk_count > 0) ark_chunk_count -= 1;
            /* Recycle into size-class freelist (Branch E) instead of free()+memalign. */
            ark_gc_size_free_push(node);
        }
        node = next;
    }
    ark_gc_total_sweep_time_ns += ark_gc_now_ns() - sweep_started;
    ark_gc_heap_head = live_head;
    ark_gc_heap_len = live_count;
    ark_gc_object_table_len = live_count;
    ark_gc_total_marked_objects += (uint64_t)live_count;
    if ((uint64_t)live_count > ark_gc_max_marked_objects_per_collection) {
        ark_gc_max_marked_objects_per_collection = (uint64_t)live_count;
    }
    if ((uint64_t)live_count > ark_gc_max_heap_objects_after_collection) {
        ark_gc_max_heap_objects_after_collection = (uint64_t)live_count;
    }

    /* Page-map is SSOT for membership; drop any emergency object-table. */
    if (ark_gc_object_table != NULL) {
        free(ark_gc_object_table);
        ark_gc_object_table = NULL;
        ark_gc_object_table_cap = 0;
    }
    ark_gc_object_table_len = live_count;

    ark_live_bytes = live;
    ark_reclaimed_bytes += reclaimed;
    ark_gc_bytes_since_collection = 0;
    if (ark_gc_threshold_override != 0) {
        ark_gc_threshold_bytes = ark_gc_threshold_override;
    } else {
        /* live*2 under soft RSS/committed caps: fewer full-heap sweeps. */
        ark_gc_threshold_bytes = live * 2ull;
        if (ark_gc_threshold_bytes < ARK_GC_INITIAL_THRESHOLD) {
            ark_gc_threshold_bytes = ARK_GC_INITIAL_THRESHOLD;
        }
    }
    /* Branch E: do not malloc_trim per collection (~20–35s). Cap freelist instead. */
    (void)reclaimed;
    ark_gc_size_free_release_excess();
    ark_gc_collecting = 0;
}

void ark_gc_push_frame(size_t slot_count) {
    if (!ark_gc_mode) return;
    ark_gc_frame *frame = ark_gc_frame_free;
    if (frame != NULL) {
        ark_gc_frame_free = frame->parent;
    } else {
        frame = malloc(sizeof(*frame));
        if (frame == NULL) ark_rt_trap_kind(ARK_TRAP_OOM);
        frame->slots = NULL;
        frame->slot_capacity = 0;
    }
    if (slot_count > frame->slot_capacity) {
        ark_object_header ***slots = realloc(frame->slots, slot_count * sizeof(*slots));
        if (slots == NULL) ark_rt_trap_kind(ARK_TRAP_OOM);
        frame->slots = slots;
        frame->slot_capacity = slot_count;
    }
    if (slot_count > 0) {
        memset(frame->slots, 0, slot_count * sizeof(*frame->slots));
    }
    frame->parent = ark_gc_frame_top;
    frame->slot_count = slot_count;
    ark_gc_frame_top = frame;
}

void ark_gc_pop_frame(void) {
    if (!ark_gc_mode) return;
    ark_gc_frame *frame = ark_gc_frame_top;
    if (frame == NULL) return;
    ark_gc_frame_top = frame->parent;
    frame->parent = ark_gc_frame_free;
    ark_gc_frame_free = frame;
}

static uint64_t ark_gc_measure_root_frame_bytes(void) {
    uint64_t total = 0;
    for (ark_gc_frame *frame = ark_gc_frame_top; frame != NULL; frame = frame->parent) {
        total += sizeof(*frame) + frame->slot_capacity * sizeof(ark_object_header **);
    }
    for (ark_gc_frame *frame = ark_gc_frame_free; frame != NULL; frame = frame->parent) {
        total += sizeof(*frame) + frame->slot_capacity * sizeof(ark_object_header **);
    }
    return total;
}

void ark_gc_set_root(size_t slot, ark_object_header **slot_ptr) {
    if (!ark_gc_mode) return;
    ark_gc_frame *frame = ark_gc_frame_top;
    if (frame == NULL || slot >= frame->slot_count) ark_rt_trap();
    frame->slots[slot] = slot_ptr;
}

void ark_gc_clear_root_slots(size_t count, const size_t *slots) {
    if (!ark_gc_mode || count == 0) return;
    {
        const char *skip = getenv("ARUKELLT_NATIVE_GC_SKIP_CLEARS");
        if (skip != NULL && skip[0] == '1') return;
    }
    ark_gc_frame *frame = ark_gc_frame_top;
    if (frame == NULL || slots == NULL) ark_rt_trap();
    for (size_t index = 0; index < count; index += 1) {
        size_t slot = slots[index];
        if (slot >= frame->slot_count) ark_rt_trap();
        ark_object_header **slot_ptr = frame->slots[slot];
        if (slot_ptr != NULL) {
            *slot_ptr = NULL;
        }
    }
}

uint64_t ark_rt_stats_requested_bytes(void) { return ark_requested_bytes; }
uint64_t ark_rt_stats_committed_bytes(void) { return ark_committed_bytes; }
uint64_t ark_rt_stats_live_bytes(void) { return ark_live_bytes; }
uint64_t ark_rt_stats_collection_count(void) { return ark_collection_count; }
uint64_t ark_rt_stats_reclaimed_bytes(void) { return ark_reclaimed_bytes; }
uint32_t ark_rt_stats_chunk_count(void) { return ark_chunk_count; }

void *ark_rt_alloc_aligned(size_t size, size_t alignment) {
    if (alignment < 16u) alignment = 16u;
    if ((alignment & (alignment - 1u)) != 0u) ark_rt_trap();
    if (!ark_gc_mode) {
        size_t required = ark_checked_add(size, alignment - 1u);
        ark_arena_chunk *chunk = ark_arena_head;
        if (chunk == NULL) {
            size_t capacity = 1024u * 1024u;
            if (capacity < required) capacity = required;
            void *allocation = malloc(ark_checked_add(capacity, 15u));
            chunk = malloc(sizeof(*chunk));
            if (allocation == NULL || chunk == NULL) {
                free(allocation);
                free(chunk);
                ark_rt_trap();
            }
            chunk->next = ark_arena_head;
            chunk->allocation = allocation;
            chunk->data = (uint8_t *)(((uintptr_t)allocation + 15u) & ~(uintptr_t)15u);
            chunk->used = 0;
            chunk->capacity = capacity;
            ark_arena_head = chunk;
            ark_chunk_count += 1;
            ark_committed_bytes += capacity;
        }
        uintptr_t base = (uintptr_t)chunk->data;
        uintptr_t aligned = (base + chunk->used + alignment - 1u) & ~(uintptr_t)(alignment - 1u);
        size_t end = ark_checked_add((size_t)(aligned - base), size);
        if (end > chunk->capacity) {
            size_t capacity = 1024u * 1024u;
            if (capacity < required) capacity = required;
            void *allocation = malloc(ark_checked_add(capacity, 15u));
            ark_arena_chunk *fresh = malloc(sizeof(*fresh));
            if (allocation == NULL || fresh == NULL) {
                free(allocation);
                free(fresh);
                ark_rt_trap();
            }
            fresh->next = ark_arena_head;
            fresh->allocation = allocation;
            fresh->data = (uint8_t *)(((uintptr_t)allocation + 15u) & ~(uintptr_t)15u);
            fresh->used = 0;
            fresh->capacity = capacity;
            ark_arena_head = fresh;
            ark_chunk_count += 1;
            ark_committed_bytes += capacity;
            chunk = fresh;
            base = (uintptr_t)chunk->data;
            aligned = (base + alignment - 1u) & ~(uintptr_t)(alignment - 1u);
            end = ark_checked_add((size_t)(aligned - base), size);
        }
        chunk->used = end;
        ark_requested_bytes += size;
        ark_live_bytes = ark_requested_bytes;
        void *result = (void *)aligned;
        memset(result, 0, size);
        return result;
    }
    {
        /* Soft limit tracks live payload only; freelist has its own cap. */
        uint64_t accounted = ark_gc_object_bytes + ark_gc_string_buffer_bytes +
            ark_gc_vec_buffer_bytes;
        if (!ark_gc_collecting &&
            (ark_gc_bytes_since_collection >= ark_gc_threshold_bytes ||
             accounted >= ark_gc_rss_soft_limit_bytes ||
             ark_committed_bytes >= ark_gc_committed_soft_limit_bytes)) {
            ark_gc_collect();
        }
    }
    size_t rounded = ark_gc_round_alloc_size(size);
    size_t prefix = sizeof(ark_gc_allocation);
    size_t total = ark_checked_add(prefix, rounded);
    ark_gc_allocation *header = ark_gc_size_free_take(size);
    if (header == NULL) {
        /* malloc is faster than posix_memalign on this mutator path; 8-byte
         * alignment is enough for ark_gc_allocation + object payloads. */
        void *block = malloc(total);
        if (block == NULL) ark_rt_trap_kind(ARK_TRAP_OOM);
        header = (ark_gc_allocation *)block;
        ark_committed_bytes += total;
        header->allocation_size = rounded;
        ark_gc_page_inc(header);
    } else {
        /* Freelist reuse: page membership remains from the original malloc. */
    }
    header->next = ark_gc_heap_head;
    header->mark = 0;
    memset(header->reserved, 0, sizeof(header->reserved));
    ark_gc_header_set_magic(header);
    ark_gc_heap_head = header;
    ark_gc_heap_len += 1u;
    ark_requested_bytes += size;
    /* Charge the rounded slab size so soft-limit accounting matches sweep. */
    ark_gc_bytes_since_collection += rounded;
    ark_live_bytes += rounded;
    ark_gc_object_bytes += rounded;
    ark_chunk_count += 1;
    void *result = ark_gc_object_from_header(header);
    memset(result, 0, size);
    (void)alignment;
    return result;
}

void ark_rt_init(int argc, char **argv) {
    ark_gc_mode = ark_env_gc_enabled();
    ark_arena_head = NULL;
    ark_gc_heap_head = NULL;
    ark_gc_free_list = NULL;
    for (int cls = 0; cls < ARK_GC_SIZE_CLASS_COUNT; cls += 1) {
        ark_gc_size_free[cls] = NULL;
    }
    ark_gc_size_free_bytes = 0;
    ark_gc_size_free_limit_bytes = ark_env_u64(
        "ARUKELLT_NATIVE_GC_SIZE_FREE_LIMIT_BYTES",
        ARK_GC_SIZE_FREE_LIMIT_DEFAULT
    );
    ark_gc_rss_soft_limit_bytes = ark_env_u64(
        "ARUKELLT_NATIVE_GC_RSS_SOFT_LIMIT_BYTES",
        ARK_GC_RSS_SOFT_LIMIT_DEFAULT
    );
    ark_gc_committed_soft_limit_bytes = ark_env_u64(
        "ARUKELLT_NATIVE_GC_COMMITTED_SOFT_LIMIT_BYTES",
        ARK_GC_COMMITTED_SOFT_LIMIT_DEFAULT
    );
    ark_gc_frame_top = NULL;
    ark_gc_frame_free = NULL;
    ark_gc_object_table = NULL;
    ark_gc_object_table_cap = 0;
    ark_gc_object_table_len = 0;
    ark_gc_page_map = NULL;
    ark_gc_page_map_cap = 0;
    ark_gc_page_map_len = 0;
    ark_gc_page_map_tombs = 0;
    ark_gc_page_map_misses = 0;
    ark_gc_mark_epoch = 1;
    ark_gc_heap_len = 0;
    ark_requested_bytes = 0;
    ark_committed_bytes = 0;
    ark_live_bytes = 0;
    ark_collection_count = 0;
    ark_reclaimed_bytes = 0;
    ark_reclaimed_object_bytes = 0;
    ark_reclaimed_side_buffer_bytes = 0;
    ark_gc_threshold_override = ark_env_u64("ARUKELLT_NATIVE_GC_THRESHOLD_BYTES", 0);
    ark_gc_threshold_bytes = ark_gc_threshold_override != 0
        ? ark_gc_threshold_override
        : ARK_GC_INITIAL_THRESHOLD;
    ark_gc_bytes_since_collection = 0;
    ark_gc_object_bytes = 0;
    ark_gc_string_buffer_bytes = 0;
    ark_gc_vec_buffer_bytes = 0;
    ark_gc_root_frame_bytes = 0;
    ark_chunk_count = 0;
    ark_gc_collecting = 0;
    ark_gc_current_function = NULL;
    ark_process_args = ark_rt_vec_new(0);
    /* Language args() matches Wasm/WASI user programs: exclude argv[0] (RFC-008).
       The selfhost native executor CLI still expects C-style argv where index 0 is
       the program name and the subcommand is at index 1; that lane sets
       ARUKELLT_NATIVE_ARGS_INCLUDE_ARGV0=1 until parse_args is migrated (#649). */
    {
        const char *include_argv0 = getenv("ARUKELLT_NATIVE_ARGS_INCLUDE_ARGV0");
        int start = (include_argv0 != NULL && include_argv0[0] == '1') ? 0 : 1;
        for (int index = start; index < argc; index += 1) {
            size_t length = strlen(argv[index]);
            if (length > UINT32_MAX) ark_rt_trap();
            ark_value value;
            value.ref = (ark_object_header *)ark_rt_string_from_bytes(
                (const uint8_t *)argv[index], (uint32_t)length
            );
            ark_rt_vec_push(ark_process_args, value);
        }
    }
}

void ark_rt_shutdown(void) {
    ark_gc_write_stats_file();
    while (ark_gc_frame_top != NULL) ark_gc_pop_frame();
    while (ark_gc_frame_free != NULL) {
        ark_gc_frame *frame = ark_gc_frame_free;
        ark_gc_frame_free = frame->parent;
        free(frame->slots);
        free(frame);
    }
    if (!ark_gc_mode) {
        ark_arena_chunk *chunk = ark_arena_head;
        while (chunk != NULL) {
            ark_arena_chunk *next = chunk->next;
            free(chunk->allocation);
            free(chunk);
            chunk = next;
        }
        ark_arena_head = NULL;
        return;
    }
    ark_gc_allocation *node = ark_gc_heap_head;
    while (node != NULL) {
        ark_gc_allocation *next = node->next;
        ark_gc_free_side_buffers((ark_object_header *)ark_gc_object_from_header(node));
        ark_gc_page_dec(node);
        free(node);
        node = next;
    }
    node = ark_gc_free_list;
    while (node != NULL) {
        ark_gc_allocation *next = node->next;
        free(node);
        node = next;
    }
    for (int cls = 0; cls < ARK_GC_SIZE_CLASS_COUNT; cls += 1) {
        ark_gc_allocation *free_node = ark_gc_size_free[cls];
        while (free_node != NULL) {
            ark_gc_allocation *next = free_node->next;
            ark_gc_page_dec(free_node);
            free(free_node);
            free_node = next;
        }
        ark_gc_size_free[cls] = NULL;
    }
    ark_gc_size_free_bytes = 0;
    ark_gc_heap_head = NULL;
    ark_gc_free_list = NULL;
    free(ark_gc_mark_stack);
    ark_gc_mark_stack = NULL;
    ark_gc_mark_stack_cap = 0;
    malloc_trim(0);
    free(ark_gc_object_table);
    ark_gc_object_table = NULL;
    ark_gc_object_table_cap = 0;
    ark_gc_object_table_len = 0;
    free(ark_gc_page_map);
    ark_gc_page_map = NULL;
    ark_gc_page_map_cap = 0;
    ark_gc_page_map_len = 0;
    ark_gc_page_map_tombs = 0;
}

static const char *ark_trap_kind_label(ark_trap_kind kind) {
    switch (kind) {
        case ARK_TRAP_BOUNDS: return "bounds error";
        case ARK_TRAP_DIV_BY_ZERO: return "divide by zero";
        case ARK_TRAP_NULL_REF: return "null reference";
        case ARK_TRAP_ALLOC: return "allocation overflow";
        case ARK_TRAP_OOM: return "out of memory";
        case ARK_TRAP_INVALID_CAST: return "invalid cast";
        case ARK_TRAP_GENERIC:
        default: return "runtime trap";
    }
}

void ark_rt_trap_kind(ark_trap_kind kind) {
    const char *function = ark_gc_current_function != NULL ? ark_gc_current_function : "<unknown>";
    fprintf(stderr, "arukellt: %s in `%s`\n", ark_trap_kind_label(kind), function);
    if (ark_gc_mode && ark_env_debug_gc_dump()) {
        ark_gc_dump_crash_state("ark_rt_trap");
    }
    abort();
}

void ark_rt_trap(void) {
    ark_rt_trap_kind(ARK_TRAP_GENERIC);
}

void ark_rt_panic(ark_string *message) {
    const char *function = ark_gc_current_function != NULL ? ark_gc_current_function : "<unknown>";
    fprintf(stderr, "arukellt: panic in `%s`: ", function);
    if (message != NULL && message->bytes != NULL && message->byte_length > 0) {
        fwrite(message->bytes, 1, message->byte_length, stderr);
    } else {
        fputs("(no message)", stderr);
    }
    fputc('\n', stderr);
    if (ark_gc_mode && ark_env_debug_gc_dump()) {
        ark_gc_dump_crash_state("ark_rt_panic");
    }
    exit(1);
}

ark_struct_object *ark_rt_struct_new(uint32_t type_id, uint32_t field_count) {
    size_t size = ark_checked_add(
        offsetof(ark_struct_object, fields),
        ark_checked_mul(field_count, sizeof(ark_value))
    );
    ark_struct_object *object = ark_rt_alloc_aligned(size, 16u);
    ark_gc_set_kind(object, ARK_GC_KIND_STRUCT);
    object->header.type_id = type_id;
    object->header.flags = 0;
    object->field_count = field_count;
    return object;
}

ark_value ark_rt_struct_get(ark_object_header *object, uint32_t field_index) {
    ark_struct_object *structure = (ark_struct_object *)object;
    if (structure == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (field_index >= structure->field_count) ark_rt_trap_kind(ARK_TRAP_BOUNDS);
    return structure->fields[field_index];
}

void ark_rt_struct_set(ark_object_header *object, uint32_t field_index, ark_value value) {
    ark_struct_object *structure = (ark_struct_object *)object;
    if (structure == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (field_index >= structure->field_count) ark_rt_trap_kind(ARK_TRAP_BOUNDS);
    structure->fields[field_index] = value;
}

ark_string *ark_rt_string_from_bytes(const uint8_t *bytes, uint32_t length) {
    ark_string *result = ark_rt_alloc_aligned(sizeof(*result), 16u);
    ark_gc_set_kind(result, ARK_GC_KIND_STRING);
    result->header.type_id = 0;
    result->header.flags = 0;
    result->byte_length = length;
    result->capacity = length;
    if (length != 0) {
        result->bytes = ark_side_bytes_string(length);
        memcpy(result->bytes, bytes, length);
    }
    return result;
}

ark_string *ark_rt_string_from_vec_bytes(ark_vec *bytes) {
    if (bytes == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    ark_string *result = ark_rt_string_from_bytes(NULL, 0);
    result->byte_length = bytes->length;
    result->capacity = bytes->length;
    if (bytes->length != 0) {
        result->bytes = ark_side_bytes_string(bytes->length);
        for (uint32_t index = 0; index < bytes->length; index += 1) {
            result->bytes[index] = (uint8_t)bytes->data[index].i32;
        }
    }
    return result;
}

ark_string *ark_rt_string_clone(ark_string *source) {
    if (source == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    return source;
}

ark_string *ark_rt_string_concat(ark_string *left, ark_string *right) {
    if (left == NULL || right == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    uint32_t length = left->byte_length + right->byte_length;
    if (length < left->byte_length) ark_rt_trap_kind(ARK_TRAP_ALLOC);
    ark_string *result = ark_rt_string_from_bytes(NULL, 0);
    result->byte_length = length;
    result->capacity = length;
    if (length != 0) {
        result->bytes = ark_side_bytes_string(length);
        memcpy(result->bytes, left->bytes, left->byte_length);
        memcpy(result->bytes + left->byte_length, right->bytes, right->byte_length);
    }
    return result;
}

ark_string *ark_rt_string_slice(ark_string *source, int32_t start, int32_t end) {
    if (source == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (start < 0 || end < start || (uint32_t)end > source->byte_length) {
        ark_rt_trap_kind(ARK_TRAP_BOUNDS);
    }
    return ark_rt_string_from_bytes(source->bytes + start, (uint32_t)(end - start));
}

int32_t ark_rt_string_len(ark_string *source) {
    if (source == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (source->byte_length > INT32_MAX) ark_rt_trap_kind(ARK_TRAP_BOUNDS);
    return (int32_t)source->byte_length;
}

int32_t ark_rt_string_char_at(ark_string *source, int32_t index) {
    if (source == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (index < 0 || (uint32_t)index >= source->byte_length) ark_rt_trap_kind(ARK_TRAP_BOUNDS);
    return source->bytes[index];
}

int32_t ark_rt_string_eq(ark_string *left, ark_string *right) {
    if (left == right) return 1;
    if (left == NULL || right == NULL || left->byte_length != right->byte_length) return 0;
    return memcmp(left->bytes, right->bytes, left->byte_length) == 0;
}

int32_t ark_rt_string_contains(ark_string *source, ark_string *needle) {
    return ark_rt_string_index_of(source, needle) >= 0;
}

int32_t ark_rt_string_starts_with(ark_string *source, ark_string *prefix) {
    if (source == NULL || prefix == NULL || prefix->byte_length > source->byte_length) return 0;
    return memcmp(source->bytes, prefix->bytes, prefix->byte_length) == 0;
}

int32_t ark_rt_string_ends_with(ark_string *source, ark_string *suffix) {
    if (source == NULL || suffix == NULL || suffix->byte_length > source->byte_length) return 0;
    return memcmp(
        source->bytes + source->byte_length - suffix->byte_length,
        suffix->bytes,
        suffix->byte_length
    ) == 0;
}

int32_t ark_rt_string_index_of(ark_string *source, ark_string *needle) {
    if (source == NULL || needle == NULL || needle->byte_length > source->byte_length) return -1;
    uint32_t limit = source->byte_length - needle->byte_length;
    for (uint32_t index = 0; index <= limit; index += 1) {
        if (memcmp(source->bytes + index, needle->bytes, needle->byte_length) == 0) {
            return (int32_t)index;
        }
    }
    return -1;
}

ark_string *ark_rt_char_to_string(uint32_t value) {
    uint8_t bytes[4];
    uint32_t length = 0;
    if (value <= 0x7fu) {
        bytes[0] = (uint8_t)value;
        length = 1;
    } else if (value <= 0x7ffu) {
        bytes[0] = (uint8_t)(0xc0u | (value >> 6));
        bytes[1] = (uint8_t)(0x80u | (value & 0x3fu));
        length = 2;
    } else if (value <= 0xffffu && !(value >= 0xd800u && value <= 0xdfffu)) {
        bytes[0] = (uint8_t)(0xe0u | (value >> 12));
        bytes[1] = (uint8_t)(0x80u | ((value >> 6) & 0x3fu));
        bytes[2] = (uint8_t)(0x80u | (value & 0x3fu));
        length = 3;
    } else if (value <= 0x10ffffu) {
        bytes[0] = (uint8_t)(0xf0u | (value >> 18));
        bytes[1] = (uint8_t)(0x80u | ((value >> 12) & 0x3fu));
        bytes[2] = (uint8_t)(0x80u | ((value >> 6) & 0x3fu));
        bytes[3] = (uint8_t)(0x80u | (value & 0x3fu));
        length = 4;
    } else {
        ark_rt_trap();
    }
    return ark_rt_string_from_bytes(bytes, length);
}

ark_string *ark_rt_i32_to_string(int32_t value) {
    char buffer[32];
    int length = snprintf(buffer, sizeof(buffer), "%" PRId32, value);
    return ark_rt_string_from_bytes((const uint8_t *)buffer, (uint32_t)length);
}

ark_string *ark_rt_i64_to_string(int64_t value) {
    char buffer[64];
    int length = snprintf(buffer, sizeof(buffer), "%" PRId64, value);
    return ark_rt_string_from_bytes((const uint8_t *)buffer, (uint32_t)length);
}

ark_string *ark_rt_f64_to_string(double value) {
    char buffer[64];
    int length = snprintf(buffer, sizeof(buffer), "%.17g", value);
    return ark_rt_string_from_bytes((const uint8_t *)buffer, (uint32_t)length);
}

ark_object_header *ark_rt_parse_f64(ark_string *source) {
    if (source == NULL) ark_rt_trap();
    char *buffer = malloc((size_t)source->byte_length + 1u);
    if (buffer == NULL) ark_rt_trap();
    memcpy(buffer, source->bytes, source->byte_length);
    buffer[source->byte_length] = '\0';
    char *end;
    errno = 0;
    double result = strtod(buffer, &end);
    ark_struct_object *parsed = ark_rt_struct_new(0, 2);
    if (errno != 0 || end != buffer + source->byte_length) {
        parsed->fields[0].i32 = 1;
        parsed->fields[1].ref = (ark_object_header *)ark_rt_string_clone(source);
        free(buffer);
        return &parsed->header;
    }
    parsed->fields[0].i32 = 0;
    parsed->fields[1].f64 = result;
    free(buffer);
    return &parsed->header;
}

ark_object_header *ark_rt_parse_i32(ark_string *source) {
    if (source == NULL) ark_rt_trap();
    char *buffer = malloc((size_t)source->byte_length + 1u);
    if (buffer == NULL) ark_rt_trap();
    memcpy(buffer, source->bytes, source->byte_length);
    buffer[source->byte_length] = '\0';
    char *end;
    errno = 0;
    long result = strtol(buffer, &end, 10);
    ark_struct_object *parsed = ark_rt_struct_new(0, 2);
    if (errno != 0 || end != buffer + source->byte_length || result < INT32_MIN || result > INT32_MAX) {
        parsed->fields[0].i32 = 1;
        parsed->fields[1].ref = (ark_object_header *)ark_rt_string_clone(source);
        free(buffer);
        return &parsed->header;
    }
    parsed->fields[0].i32 = 0;
    parsed->fields[1].i32 = (int32_t)result;
    free(buffer);
    return &parsed->header;
}

ark_object_header *ark_rt_parse_i64(ark_string *source) {
    if (source == NULL) ark_rt_trap();
    char *buffer = malloc((size_t)source->byte_length + 1u);
    if (buffer == NULL) ark_rt_trap();
    memcpy(buffer, source->bytes, source->byte_length);
    buffer[source->byte_length] = '\0';
    char *end;
    errno = 0;
    long long result = strtoll(buffer, &end, 10);
    ark_struct_object *parsed = ark_rt_struct_new(0, 2);
    if (errno != 0 || end != buffer + source->byte_length) {
        parsed->fields[0].i32 = 1;
        parsed->fields[1].ref = (ark_object_header *)ark_rt_string_clone(source);
        free(buffer);
        return &parsed->header;
    }
    parsed->fields[0].i32 = 0;
    parsed->fields[1].i64 = (int64_t)result;
    free(buffer);
    return &parsed->header;
}

void ark_rt_assert(int32_t condition) {
    if (!condition) {
        const char *function = ark_gc_current_function != NULL ? ark_gc_current_function : "<unknown>";
        fprintf(stderr, "arukellt: panic in `%s`: assertion failed\n", function);
        exit(1);
    }
}

void ark_rt_assert_eq_i32(int32_t left, int32_t right) {
    if (left != right) {
        const char *function = ark_gc_current_function != NULL ? ark_gc_current_function : "<unknown>";
        fprintf(stderr, "arukellt: panic in `%s`: assertion failed\n", function);
        exit(1);
    }
}

static void ark_vec_reserve(ark_vec *vector, uint32_t minimum);
ark_vec *ark_rt_vec_new(uint32_t type_id);
ark_unit ark_rt_vec_push(ark_vec *vector, ark_value value);

void ark_rt_assert_eq_i64(int64_t left, int64_t right) {
    if (left != right) {
        const char *function = ark_gc_current_function != NULL ? ark_gc_current_function : "<unknown>";
        fprintf(stderr, "arukellt: panic in `%s`: assertion failed\n", function);
        exit(1);
    }
}

void ark_rt_assert_eq_str(ark_string *left, ark_string *right) {
    if (!ark_rt_string_eq(left, right)) {
        const char *function = ark_gc_current_function != NULL ? ark_gc_current_function : "<unknown>";
        fprintf(stderr, "arukellt: panic in `%s`: assertion failed\n", function);
        exit(1);
    }
}

void ark_rt_assert_ne_i32(int32_t left, int32_t right) {
    if (left == right) {
        const char *function = ark_gc_current_function != NULL ? ark_gc_current_function : "<unknown>";
        fprintf(stderr, "arukellt: panic in `%s`: assertion failed\n", function);
        exit(1);
    }
}

ark_object_header *ark_rt_range_new(int32_t start, int32_t end) {
    /* Match std::core::Range { start, end } / Wasm half-open layout. */
    ark_object_header *range = (ark_object_header *)ark_rt_struct_new(0u, 2u);
    ark_rt_struct_set(range, 0u, (ark_value){ .i32 = start });
    ark_rt_struct_set(range, 1u, (ark_value){ .i32 = end });
    return range;
}

int32_t ark_rt_range_contains(ark_object_header *range, int32_t value) {
    if (range == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    int32_t start = ark_rt_struct_get(range, 0u).i32;
    int32_t end = ark_rt_struct_get(range, 1u).i32;
    return value >= start && value < end;
}

int32_t ark_rt_range_len(ark_object_header *range) {
    if (range == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    int32_t start = ark_rt_struct_get(range, 0u).i32;
    int32_t end = ark_rt_struct_get(range, 1u).i32;
    if (end > start) {
        return end - start;
    }
    return 0;
}

static int ark_rt_is_ascii_space(uint8_t byte) {
    return byte == ' ' || byte == '\t' || byte == '\n' || byte == '\r';
}

ark_string *ark_rt_bool_to_string(int32_t value) {
    if (value) {
        return ark_rt_string_from_bytes((const uint8_t *)"true", 4u);
    }
    return ark_rt_string_from_bytes((const uint8_t *)"false", 5u);
}

ark_string *ark_rt_string_trim_start(ark_string *source) {
    if (source == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    uint32_t start = 0;
    while (start < source->byte_length && ark_rt_is_ascii_space(source->bytes[start])) {
        start += 1u;
    }
    return ark_rt_string_from_bytes(source->bytes + start, source->byte_length - start);
}

ark_string *ark_rt_string_trim_end(ark_string *source) {
    if (source == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    uint32_t end = source->byte_length;
    while (end > 0u && ark_rt_is_ascii_space(source->bytes[end - 1u])) {
        end -= 1u;
    }
    return ark_rt_string_from_bytes(source->bytes, end);
}

ark_string *ark_rt_string_trim(ark_string *source) {
    if (source == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    uint32_t start = 0;
    while (start < source->byte_length && ark_rt_is_ascii_space(source->bytes[start])) {
        start += 1u;
    }
    uint32_t end = source->byte_length;
    while (end > start && ark_rt_is_ascii_space(source->bytes[end - 1u])) {
        end -= 1u;
    }
    return ark_rt_string_from_bytes(source->bytes + start, end - start);
}

ark_string *ark_rt_string_replace(ark_string *source, ark_string *from, ark_string *to) {
    if (source == NULL || from == NULL || to == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (from->byte_length == 0u) {
        return ark_rt_string_clone(source);
    }
    uint32_t count = 0;
    uint32_t index = 0;
    while (index + from->byte_length <= source->byte_length) {
        if (memcmp(source->bytes + index, from->bytes, from->byte_length) == 0) {
            count += 1u;
            index += from->byte_length;
        } else {
            index += 1u;
        }
    }
    uint64_t out_len64 = (uint64_t)source->byte_length
        + (uint64_t)count * ((uint64_t)to->byte_length - (uint64_t)from->byte_length);
    if (out_len64 > UINT32_MAX) ark_rt_trap_kind(ARK_TRAP_ALLOC);
    uint32_t out_len = (uint32_t)out_len64;
    ark_string *result = ark_rt_string_from_bytes(NULL, 0);
    result->byte_length = out_len;
    result->capacity = out_len;
    if (out_len != 0u) {
        result->bytes = ark_side_bytes_string(out_len);
        uint32_t write = 0;
        index = 0;
        while (index < source->byte_length) {
            if (index + from->byte_length <= source->byte_length
                && memcmp(source->bytes + index, from->bytes, from->byte_length) == 0) {
                if (to->byte_length != 0u) {
                    memcpy(result->bytes + write, to->bytes, to->byte_length);
                    write += to->byte_length;
                }
                index += from->byte_length;
            } else {
                result->bytes[write] = source->bytes[index];
                write += 1u;
                index += 1u;
            }
        }
    }
    return result;
}

ark_string *ark_rt_string_repeat(ark_string *source, int32_t count) {
    if (source == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (count < 0) ark_rt_trap_kind(ARK_TRAP_BOUNDS);
    if (count == 0 || source->byte_length == 0u) {
        return ark_rt_string_from_bytes(NULL, 0);
    }
    uint64_t out_len64 = (uint64_t)source->byte_length * (uint64_t)(uint32_t)count;
    if (out_len64 > UINT32_MAX) ark_rt_trap_kind(ARK_TRAP_ALLOC);
    uint32_t out_len = (uint32_t)out_len64;
    ark_string *result = ark_rt_string_from_bytes(NULL, 0);
    result->byte_length = out_len;
    result->capacity = out_len;
    result->bytes = ark_side_bytes_string(out_len);
    for (int32_t i = 0; i < count; i += 1) {
        memcpy(result->bytes + (uint32_t)i * source->byte_length, source->bytes, source->byte_length);
    }
    return result;
}

ark_string *ark_rt_string_to_lowercase(ark_string *source) {
    if (source == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    ark_string *result = ark_rt_string_from_bytes(source->bytes, source->byte_length);
    for (uint32_t i = 0; i < result->byte_length; i += 1u) {
        result->bytes[i] = (uint8_t)tolower((int)result->bytes[i]);
    }
    return result;
}

ark_string *ark_rt_string_to_uppercase(ark_string *source) {
    if (source == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    ark_string *result = ark_rt_string_from_bytes(source->bytes, source->byte_length);
    for (uint32_t i = 0; i < result->byte_length; i += 1u) {
        result->bytes[i] = (uint8_t)toupper((int)result->bytes[i]);
    }
    return result;
}

ark_string *ark_rt_string_reverse(ark_string *source) {
    if (source == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    ark_string *result = ark_rt_string_from_bytes(source->bytes, source->byte_length);
    for (uint32_t i = 0; i < result->byte_length / 2u; i += 1u) {
        uint8_t tmp = result->bytes[i];
        result->bytes[i] = result->bytes[result->byte_length - 1u - i];
        result->bytes[result->byte_length - 1u - i] = tmp;
    }
    return result;
}

ark_string *ark_rt_string_join(ark_vec *parts, ark_string *separator) {
    if (parts == NULL || separator == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    uint64_t total = 0;
    for (uint32_t i = 0; i < parts->length; i += 1u) {
        ark_string *part = (ark_string *)parts->data[i].ref;
        if (part == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
        total += part->byte_length;
        if (i + 1u < parts->length) {
            total += separator->byte_length;
        }
    }
    if (total > UINT32_MAX) ark_rt_trap_kind(ARK_TRAP_ALLOC);
    ark_string *result = ark_rt_string_from_bytes(NULL, 0);
    result->byte_length = (uint32_t)total;
    result->capacity = (uint32_t)total;
    if (total != 0u) {
        result->bytes = ark_side_bytes_string((uint32_t)total);
        uint32_t write = 0;
        for (uint32_t i = 0; i < parts->length; i += 1u) {
            ark_string *part = (ark_string *)parts->data[i].ref;
            if (part->byte_length != 0u) {
                memcpy(result->bytes + write, part->bytes, part->byte_length);
                write += part->byte_length;
            }
            if (i + 1u < parts->length && separator->byte_length != 0u) {
                memcpy(result->bytes + write, separator->bytes, separator->byte_length);
                write += separator->byte_length;
            }
        }
    }
    return result;
}

ark_vec *ark_rt_string_split(ark_string *source, ark_string *separator, uint32_t type_id) {
    if (source == NULL || separator == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    ark_vec *parts = ark_rt_vec_new(type_id);
    if (separator->byte_length == 0u) {
        ark_rt_vec_push(parts, (ark_value){.ref = (ark_object_header *)ark_rt_string_clone(source)});
        return parts;
    }
    uint32_t start = 0;
    uint32_t index = 0;
    while (index + separator->byte_length <= source->byte_length) {
        if (memcmp(source->bytes + index, separator->bytes, separator->byte_length) == 0) {
            ark_string *part = ark_rt_string_from_bytes(source->bytes + start, index - start);
            ark_rt_vec_push(parts, (ark_value){.ref = (ark_object_header *)part});
            index += separator->byte_length;
            start = index;
        } else {
            index += 1u;
        }
    }
    ark_string *tail = ark_rt_string_from_bytes(source->bytes + start, source->byte_length - start);
    ark_rt_vec_push(parts, (ark_value){.ref = (ark_object_header *)tail});
    return parts;
}

ark_vec *ark_rt_string_lines(ark_string *source, uint32_t type_id) {
    static uint8_t newline_bytes[1] = {'\n'};
    ark_string separator;
    separator.header.type_id = 0u;
    separator.header.flags = 0u;
    separator.bytes = newline_bytes;
    separator.byte_length = 1u;
    separator.capacity = 1u;
    return ark_rt_string_split(source, &separator, type_id);
}

ark_vec *ark_rt_string_chars(ark_string *source, uint32_t type_id) {
    if (source == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    ark_vec *chars = ark_rt_vec_new(type_id);
    uint32_t index = 0;
    while (index < source->byte_length) {
        ark_rt_vec_push(chars, (ark_value){.i32 = (int32_t)source->bytes[index]});
        index += 1u;
    }
    return chars;
}

int32_t ark_rt_string_is_empty(ark_string *source) {
    if (source == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    return source->byte_length == 0u;
}

int32_t ark_rt_is_ok(ark_object_header *value) {
    if (value == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    return ((ark_struct_object *)value)->fields[0].i32 == 0;
}

int32_t ark_rt_is_err(ark_object_header *value) {
    if (value == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    return ((ark_struct_object *)value)->fields[0].i32 != 0;
}

ark_value ark_rt_result_unwrap(ark_object_header *value) {
    if (value == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    ark_struct_object *object = (ark_struct_object *)value;
    if (object->fields[0].i32 != 0) {
        const char *function = ark_gc_current_function != NULL ? ark_gc_current_function : "<unknown>";
        fprintf(stderr, "arukellt: panic in `%s`: unwrap on Err/None\n", function);
        exit(1);
    }
    return object->fields[1];
}

ark_value ark_rt_result_unwrap_or(ark_object_header *value, ark_value fallback) {
    if (value == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    ark_struct_object *object = (ark_struct_object *)value;
    if (object->fields[0].i32 == 0) {
        return object->fields[1];
    }
    return fallback;
}

int32_t ark_rt_vec_is_empty(ark_vec *vector) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    return vector->length == 0u;
}

int32_t ark_rt_math_abs_i32(int32_t value) {
    if (value == INT32_MIN) ark_rt_trap();
    return value < 0 ? -value : value;
}

int32_t ark_rt_math_min_i32(int32_t left, int32_t right) {
    return left < right ? left : right;
}

int32_t ark_rt_math_max_i32(int32_t left, int32_t right) {
    return left > right ? left : right;
}

int32_t ark_rt_math_clamp_i32(int32_t value, int32_t low, int32_t high) {
    if (low > high) ark_rt_trap();
    if (value < low) return low;
    if (value > high) return high;
    return value;
}

int32_t ark_rt_next_power_of_two_i32(int32_t value) {
    if (value <= 1) {
        return 1;
    }
    uint32_t bits = (uint32_t)value - 1u;
    bits |= bits >> 1;
    bits |= bits >> 2;
    bits |= bits >> 4;
    bits |= bits >> 8;
    bits |= bits >> 16;
    return (int32_t)(bits + 1u);
}

int32_t ark_rt_popcount_i32(int32_t value) {
    return (int32_t)__builtin_popcount((unsigned)value);
}

int32_t ark_rt_popcount_i64(int64_t value) {
    return (int32_t)__builtin_popcountll((unsigned long long)value);
}

int32_t ark_rt_leading_zeros_i32(int32_t value) {
    if (value == 0) {
        return 32;
    }
    return (int32_t)__builtin_clz((unsigned)value);
}

int32_t ark_rt_leading_zeros_i64(int64_t value) {
    if (value == 0) {
        return 64;
    }
    return (int32_t)__builtin_clzll((unsigned long long)value);
}

int32_t ark_rt_trailing_zeros_i32(int32_t value) {
    if (value == 0) {
        return 32;
    }
    return (int32_t)__builtin_ctz((unsigned)value);
}

int32_t ark_rt_trailing_zeros_i64(int64_t value) {
    if (value == 0) {
        return 64;
    }
    return (int32_t)__builtin_ctzll((unsigned long long)value);
}

int32_t ark_rt_is_power_of_two_i32(int32_t value) {
    return value > 0 && (value & (value - 1)) == 0;
}

int32_t ark_rt_is_power_of_two_i64(int64_t value) {
    return value > 0 && (value & (value - 1)) == 0;
}

int32_t ark_rt_math_gcd_i32(int32_t left, int32_t right) {
    int32_t a = left < 0 ? -left : left;
    int32_t b = right < 0 ? -right : right;
    while (b != 0) {
        int32_t next = a % b;
        a = b;
        b = next;
    }
    return a;
}

int32_t ark_rt_math_pow_i32(int32_t base, int32_t exp) {
    if (exp < 0) ark_rt_trap();
    int64_t result = 1;
    int64_t cur = base;
    int32_t power = exp;
    while (power > 0) {
        if ((power & 1) != 0) {
            result *= cur;
            if (result < INT32_MIN || result > INT32_MAX) ark_rt_trap();
        }
        power >>= 1;
        if (power > 0) {
            cur *= cur;
            if (cur < INT32_MIN || cur > INT32_MAX) ark_rt_trap();
        }
    }
    return (int32_t)result;
}

double ark_rt_math_sqrt_f64(double value) {
    if (value < 0.0) ark_rt_trap();
    return sqrt(value);
}

ark_vec *ark_rt_vec_new(uint32_t type_id) {
    ark_vec *vector = ark_rt_alloc_aligned(sizeof(*vector), 16u);
    ark_gc_set_kind(vector, ARK_GC_KIND_VEC);
    vector->header.type_id = type_id;
    vector->header.flags = 0;
    vector->data = NULL;
    vector->length = 0;
    vector->capacity = 0;
    return vector;
}

ark_vec *ark_rt_vec_new_with_capacity(uint32_t type_id, int32_t capacity) {
    if (capacity < 0) ark_rt_trap_kind(ARK_TRAP_BOUNDS);
    ark_vec *vector = ark_rt_vec_new(type_id);
    ark_vec_reserve(vector, (uint32_t)capacity);
    return vector;
}

ark_vec *ark_rt_array_new(uint32_t type_id, int32_t length) {
    if (length < 0) ark_rt_trap_kind(ARK_TRAP_BOUNDS);
    ark_vec *vector = ark_rt_vec_new_with_capacity(type_id, length);
    vector->length = (uint32_t)length;
    if (length > 0 && vector->data != NULL) {
        memset(vector->data, 0, (size_t)length * sizeof(*vector->data));
    }
    return vector;
}

int32_t ark_rt_arg_count(void) {
    return ark_rt_vec_len(ark_process_args);
}

ark_unit ark_rt_string_push_char(ark_string *string, uint32_t codepoint) {
    if (string == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    uint8_t encoded[4];
    uint32_t encoded_len = 0;
    if (codepoint <= 0x7Fu) {
        encoded[0] = (uint8_t)codepoint;
        encoded_len = 1;
    } else if (codepoint <= 0x7FFu) {
        encoded[0] = (uint8_t)(0xC0u | (codepoint >> 6));
        encoded[1] = (uint8_t)(0x80u | (codepoint & 0x3Fu));
        encoded_len = 2;
    } else if (codepoint <= 0xFFFFu) {
        encoded[0] = (uint8_t)(0xE0u | (codepoint >> 12));
        encoded[1] = (uint8_t)(0x80u | ((codepoint >> 6) & 0x3Fu));
        encoded[2] = (uint8_t)(0x80u | (codepoint & 0x3Fu));
        encoded_len = 3;
    } else if (codepoint <= 0x10FFFFu) {
        encoded[0] = (uint8_t)(0xF0u | (codepoint >> 18));
        encoded[1] = (uint8_t)(0x80u | ((codepoint >> 12) & 0x3Fu));
        encoded[2] = (uint8_t)(0x80u | ((codepoint >> 6) & 0x3Fu));
        encoded[3] = (uint8_t)(0x80u | (codepoint & 0x3Fu));
        encoded_len = 4;
    } else {
        ark_rt_trap_kind(ARK_TRAP_INVALID_CAST);
    }
    if (string->byte_length > UINT32_MAX - encoded_len) ark_rt_trap_kind(ARK_TRAP_ALLOC);
    uint32_t needed = string->byte_length + encoded_len;
    if (needed > string->capacity) {
        uint32_t capacity = string->capacity == 0 ? 16u : string->capacity;
        while (capacity < needed) {
            if (capacity > UINT32_MAX / 2u) ark_rt_trap_kind(ARK_TRAP_ALLOC);
            capacity *= 2u;
        }
        uint8_t *bytes = ark_side_bytes_string(capacity);
        if (string->byte_length != 0 && string->bytes != NULL) {
            memcpy(bytes, string->bytes, string->byte_length);
        }
        string->bytes = bytes;
        string->capacity = capacity;
    }
    memcpy(string->bytes + string->byte_length, encoded, encoded_len);
    string->byte_length = needed;
    return 0;
}

static void ark_vec_reserve(ark_vec *vector, uint32_t minimum) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (vector->capacity >= minimum) return;
    uint32_t capacity = vector->capacity == 0 ? 4u : vector->capacity;
    while (capacity < minimum) {
        if (capacity > UINT32_MAX / 2u) ark_rt_trap_kind(ARK_TRAP_ALLOC);
        capacity *= 2u;
    }
    size_t bytes = ark_checked_mul(capacity, sizeof(*vector->data));
    ark_value *data = ark_side_bytes_vec(bytes);
    if (vector->length != 0) memcpy(data, vector->data, vector->length * sizeof(*data));
    if (ark_gc_mode && vector->data != NULL) {
        uint64_t old_bytes = (uint64_t)vector->capacity * (uint64_t)sizeof(ark_value);
        if (ark_gc_vec_buffer_bytes >= old_bytes) ark_gc_vec_buffer_bytes -= old_bytes;
        else ark_gc_vec_buffer_bytes = 0;
        free(vector->data);
    }
    vector->data = data;
    vector->capacity = capacity;
}

int32_t ark_rt_vec_len(ark_vec *vector) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (vector->length > INT32_MAX) ark_rt_trap_kind(ARK_TRAP_BOUNDS);
    return (int32_t)vector->length;
}

ark_value ark_rt_vec_get(ark_vec *vector, int32_t index) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (index < 0 || (uint32_t)index >= vector->length) ark_rt_trap_kind(ARK_TRAP_BOUNDS);
    return vector->data[index];
}


void ark_rt_vec_sort_i32(ark_vec *vector) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    for (uint32_t i = 1; i < vector->length; i += 1) {
        int32_t value = vector->data[i].i32;
        uint32_t insertion = i;
        while (insertion > 0 && vector->data[insertion - 1u].i32 > value) {
            vector->data[insertion] = vector->data[insertion - 1u];
            insertion -= 1u;
        }
        vector->data[insertion].i32 = value;
    }
}

void ark_rt_vec_sort_i64(ark_vec *vector) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    for (uint32_t i = 1; i < vector->length; i += 1) {
        int64_t value = vector->data[i].i64;
        uint32_t insertion = i;
        while (insertion > 0 && vector->data[insertion - 1u].i64 > value) {
            vector->data[insertion] = vector->data[insertion - 1u];
            insertion -= 1u;
        }
        vector->data[insertion].i64 = value;
    }
}

void ark_rt_vec_sort_f64(ark_vec *vector) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    for (uint32_t i = 1; i < vector->length; i += 1) {
        double value = vector->data[i].f64;
        uint32_t insertion = i;
        while (insertion > 0 && vector->data[insertion - 1u].f64 > value) {
            vector->data[insertion] = vector->data[insertion - 1u];
            insertion -= 1u;
        }
        vector->data[insertion].f64 = value;
    }
}

int32_t ark_rt_vec_sum_i32(ark_vec *vector) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    int32_t total = 0;
    for (uint32_t i = 0; i < vector->length; i += 1) total += vector->data[i].i32;
    return total;
}

int32_t ark_rt_vec_product_i32(ark_vec *vector) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    int32_t total = 1;
    for (uint32_t i = 0; i < vector->length; i += 1) total *= vector->data[i].i32;
    return total;
}

ark_unit ark_rt_vec_reverse(ark_vec *vector) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (vector->length == 0) return 0;
    uint32_t lo = 0;
    uint32_t hi = vector->length - 1u;
    while (lo < hi) {
        ark_value tmp = vector->data[lo];
        vector->data[lo] = vector->data[hi];
        vector->data[hi] = tmp;
        lo += 1u;
        hi -= 1u;
    }
    return 0;
}

ark_unit ark_rt_vec_remove(ark_vec *vector, int32_t index) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (index < 0 || (uint32_t)index >= vector->length) ark_rt_trap_kind(ARK_TRAP_BOUNDS);
    for (uint32_t i = (uint32_t)index + 1u; i < vector->length; i += 1) {
        vector->data[i - 1u] = vector->data[i];
    }
    vector->length -= 1u;
    return 0;
}

int32_t ark_rt_vec_contains_i32(ark_vec *vector, int32_t value) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    for (uint32_t i = 0; i < vector->length; i += 1) {
        if (vector->data[i].i32 == value) return 1;
    }
    return 0;
}

ark_object_header *ark_rt_vec_get_option(ark_vec *vector, int32_t index, uint32_t option_type_id) {
    ark_struct_object *option = ark_rt_struct_new(option_type_id, 2u);
    if (vector == NULL || index < 0 || (uint32_t)index >= vector->length) {
        option->fields[0].i32 = 1;
        option->fields[1].i32 = 0;
        return &option->header;
    }
    option->fields[0].i32 = 0;
    option->fields[1] = vector->data[index];
    return &option->header;
}

ark_unit ark_rt_vec_set(ark_vec *vector, int32_t index, ark_value value) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (index < 0 || (uint32_t)index >= vector->length) ark_rt_trap_kind(ARK_TRAP_BOUNDS);
    vector->data[index] = value;
    return 0;
}

ark_unit ark_rt_vec_push(ark_vec *vector, ark_value value) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (vector->length == UINT32_MAX) ark_rt_trap_kind(ARK_TRAP_ALLOC);
    ark_vec_reserve(vector, vector->length + 1u);
    vector->data[vector->length] = value;
    vector->length += 1u;
    return 0;
}

ark_unit ark_rt_vec_clear(ark_vec *vector) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    vector->length = 0u;
    return 0;
}

ark_value ark_rt_vec_pop(ark_vec *vector) {
    if (vector == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    if (vector->length == 0) ark_rt_trap_kind(ARK_TRAP_BOUNDS);
    vector->length -= 1u;
    return vector->data[vector->length];
}

ark_vec *ark_rt_args(void) {
    return ark_process_args;
}

static ark_string *ark_read_stream(FILE *stream) {
    ark_vec *bytes = ark_rt_vec_new(0);
    uint8_t buffer[16384];
    while (!feof(stream)) {
        size_t count = fread(buffer, 1, sizeof(buffer), stream);
        for (size_t index = 0; index < count; index += 1) {
            ark_value value;
            value.i32 = buffer[index];
            ark_rt_vec_push(bytes, value);
        }
        if (ferror(stream)) ark_rt_trap();
    }
    ark_string *result = ark_rt_string_from_bytes(NULL, 0);
    result->byte_length = bytes->length;
    result->capacity = bytes->length;
    if (bytes->length != 0) {
        result->bytes = ark_side_bytes_string(bytes->length);
        for (uint32_t index = 0; index < bytes->length; index += 1) {
            result->bytes[index] = (uint8_t)bytes->data[index].i32;
        }
    }
    return result;
}

static char *ark_path(ark_string *path) {
    if (path == NULL || memchr(path->bytes, 0, path->byte_length) != NULL) ark_rt_trap();
    char *result = malloc((size_t)path->byte_length + 1u);
    if (result == NULL) ark_rt_trap();
    memcpy(result, path->bytes, path->byte_length);
    result[path->byte_length] = '\0';
    return result;
}

ark_string *ark_rt_read_stdin(void) {
    return ark_read_stream(stdin);
}

static ark_object_header *ark_result_string(int32_t tag, ark_string *payload) {
    ark_object_header *result = (ark_object_header *)ark_rt_struct_new(0u, 2u);
    ark_rt_struct_set(result, 0u, (ark_value){ .i32 = tag });
    ark_rt_struct_set(result, 1u, (ark_value){ .ref = (ark_object_header *)payload });
    return result;
}

ark_object_header *ark_rt_fs_read_file(ark_string *path) {
    FILE *file = fopen(ark_path(path), "rb");
    if (file == NULL) {
        static const uint8_t message[] = "file open error";
        return ark_result_string(
            1,
            ark_rt_string_from_bytes(message, (uint32_t)(sizeof(message) - 1u))
        );
    }
    ark_string *result = ark_read_stream(file);
    if (fclose(file) != 0) ark_rt_trap();
    return ark_result_string(0, result);
}

static void ark_write_stream(FILE *stream, ark_string *text) {
    if (text == NULL) ark_rt_trap();
    size_t offset = 0;
    while (offset < text->byte_length) {
        size_t count = fwrite(text->bytes + offset, 1, text->byte_length - offset, stream);
        if (count == 0) ark_rt_trap();
        offset += count;
    }
}

ark_unit ark_rt_print(ark_string *text) {
    ark_write_stream(stdout, text);
    return 0;
}

ark_unit ark_rt_println(ark_string *text) {
    ark_write_stream(stdout, text);
    fputc('\n', stdout);
    return 0;
}

ark_unit ark_rt_eprintln(ark_string *text) {
    ark_write_stream(stderr, text);
    fputc('\n', stderr);
    return 0;
}

static ark_object_header *ark_write_success(void) {
    ark_object_header *result = (ark_object_header *)ark_rt_struct_new(0u, 2u);
    ark_rt_struct_set(result, 0u, (ark_value){ .i32 = 0 });
    ark_rt_struct_set(result, 1u, (ark_value){ .i32 = 0 });
    return result;
}

static ark_object_header *ark_write_error(const char *message) {
    return ark_result_string(
        1,
        ark_rt_string_from_bytes((const uint8_t *)message, (uint32_t)strlen(message))
    );
}

static ark_object_header *ark_try_write_file(ark_string *path, ark_string *text) {
    char *cpath = ark_path(path);
    FILE *file = fopen(cpath, "wb");
    free(cpath);
    if (file == NULL) {
        return ark_write_error("file open error");
    }
    if (text == NULL) {
        fclose(file);
        return ark_write_error("null text");
    }
    size_t offset = 0;
    while (offset < text->byte_length) {
        size_t count = fwrite(text->bytes + offset, 1, text->byte_length - offset, file);
        if (count == 0) {
            fclose(file);
            return ark_write_error("file write error");
        }
        offset += count;
    }
    if (fclose(file) != 0) {
        return ark_write_error("file close error");
    }
    return ark_write_success();
}

ark_object_header *ark_rt_write_bytes(ark_string *path, ark_vec *bytes) {
    if (bytes == NULL) ark_rt_trap_kind(ARK_TRAP_NULL_REF);
    ark_string view;
    view.header.type_id = 0;
    view.header.flags = 0;
    view.byte_length = bytes->length;
    view.capacity = bytes->length;
    view.bytes = ark_rt_alloc_aligned(bytes->length, 16u);
    for (uint32_t index = 0; index < bytes->length; index += 1) {
        view.bytes[index] = (uint8_t)bytes->data[index].i32;
    }
    return ark_try_write_file(path, &view);
}

ark_object_header *ark_rt_write_string(ark_string *path, ark_string *text) {
    return ark_try_write_file(path, text);
}

ark_unit ark_rt_process_exit(int32_t status) {
    exit(status);
}

int64_t ark_rt_clock_now_ms(void) {
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) ark_rt_trap();
    return (int64_t)now.tv_sec * INT64_C(1000) + now.tv_nsec / INT64_C(1000000);
}

int32_t ark_rt_f64_bits_hi(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return (int32_t)(uint32_t)(bits >> 32);
}

int32_t ark_rt_f64_bits_lo(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return (int32_t)(uint32_t)bits;
}

int32_t ark_div_i32(int32_t left, int32_t right) {
    if (right == 0 || (left == INT32_MIN && right == -1)) ark_rt_trap_kind(ARK_TRAP_DIV_BY_ZERO);
    return left / right;
}

int64_t ark_div_i64(int64_t left, int64_t right) {
    if (right == 0 || (left == INT64_MIN && right == -1)) ark_rt_trap_kind(ARK_TRAP_DIV_BY_ZERO);
    return left / right;
}

int32_t ark_rem_i32(int32_t left, int32_t right) {
    if (right == 0) ark_rt_trap_kind(ARK_TRAP_DIV_BY_ZERO);
    if (left == INT32_MIN && right == -1) return 0;
    return left % right;
}

int64_t ark_rem_i64(int64_t left, int64_t right) {
    if (right == 0) ark_rt_trap_kind(ARK_TRAP_DIV_BY_ZERO);
    if (left == INT64_MIN && right == -1) return 0;
    return left % right;
}

static double ark_trunc_f64(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    uint64_t exponent_bits = (bits >> 52) & UINT64_C(0x7ff);
    if (exponent_bits == UINT64_C(0x7ff)) return value;
    int32_t exponent = (int32_t)exponent_bits - 1023;
    if (exponent < 0) {
        bits &= UINT64_C(0x8000000000000000);
    } else if (exponent < 52) {
        bits &= ~(UINT64_C(0x000fffffffffffff) >> exponent);
    }
    memcpy(&value, &bits, sizeof(value));
    return value;
}

double ark_rem_f64(double left, double right) {
    return left - right * ark_trunc_f64(left / right);
}

int32_t ark_shl_i32(int32_t left, int32_t right) {
    return (int32_t)((uint32_t)left << ((uint32_t)right & 31u));
}

int64_t ark_shl_i64(int64_t left, int64_t right) {
    return (int64_t)((uint64_t)left << ((uint64_t)right & 63u));
}

int32_t ark_shr_i32(int32_t left, int32_t right) {
    uint32_t count = (uint32_t)right & 31u;
    uint32_t shifted = (uint32_t)left >> count;
    if (left < 0 && count != 0) shifted |= UINT32_MAX << (32u - count);
    return (int32_t)shifted;
}

int64_t ark_shr_i64(int64_t left, int64_t right) {
    uint64_t count = (uint64_t)right & 63u;
    uint64_t shifted = (uint64_t)left >> count;
    if (left < 0 && count != 0) shifted |= UINT64_MAX << (64u - count);
    return (int64_t)shifted;
}
