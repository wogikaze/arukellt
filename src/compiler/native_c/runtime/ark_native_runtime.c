#define _POSIX_C_SOURCE 200809L
#include "ark_native_runtime.h"

#include <errno.h>
#include <inttypes.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <malloc.h>
#include <string.h>
#include <time.h>

/* ark_rt_trap is defined later; table helpers need it early. */
void ark_rt_trap(void);

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
static ark_gc_frame *ark_gc_frame_top;
static ark_gc_frame *ark_gc_frame_free;
static ark_gc_allocation **ark_gc_object_table;
static size_t ark_gc_object_table_cap;
static size_t ark_gc_object_table_len;
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
static uint32_t ark_chunk_count;
static ark_vec *ark_process_args;
static int ark_gc_collecting;
static const char *ark_gc_current_function;

static int ark_env_gc_enabled(void) {
    const char *enable = getenv("ARUKELLT_NATIVE_GC");
    /* Default on: process-lifetime arena exceeds the 2.4 GiB executor gate. */
    if (enable == NULL) return 1;
    return enable[0] == '1';
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

static void ark_gc_write_stats_file(void) {
    const char *path = getenv("ARUKELLT_NATIVE_GC_STATS_PATH");
    if (path == NULL || path[0] == '\0') return;
    FILE *out = fopen(path, "w");
    if (out == NULL) return;
    uint64_t table_bytes = (uint64_t)ark_gc_object_table_cap * (uint64_t)sizeof(ark_gc_allocation *);
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

static size_t ark_checked_add(size_t left, size_t right) {
    if (left > SIZE_MAX - right) ark_rt_trap();
    return left + right;
}

static size_t ark_checked_mul(size_t left, size_t right) {
    if (left != 0 && right > SIZE_MAX / left) ark_rt_trap();
    return left * right;
}

static ark_gc_allocation *ark_gc_header_from_object(void *object) {
    return ((ark_gc_allocation *)object) - 1;
}

static void *ark_gc_object_from_header(ark_gc_allocation *header) {
    return (void *)(header + 1);
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
    if (fresh == NULL) ark_rt_trap();
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
    if (object == NULL || ark_gc_object_table_cap == 0) return NULL;
    size_t mask = ark_gc_object_table_cap - 1u;
    size_t slot = ark_gc_hash_ptr(object) & mask;
    for (;;) {
        ark_gc_allocation *header = ark_gc_object_table[slot];
        if (header == NULL) return NULL;
        if (ark_gc_object_from_header(header) == object) return header;
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

static void ark_gc_table_rebuild_from_heap(void) {
    size_t count = 0;
    for (ark_gc_allocation *node = ark_gc_heap_head; node != NULL; node = node->next) {
        count += 1;
    }
    size_t desired = count * 2u;
    if (desired < 1024u) desired = 1024u;
    desired = ark_next_pow2_size(desired);
    /* Grow when short; shrink only when capacity is more than 4× desired. */
    int needs_realloc = ark_gc_object_table_cap == 0 ||
        ark_gc_object_table_cap < desired ||
        ark_gc_object_table_cap > desired * 4u;
    if (needs_realloc) {
        free(ark_gc_object_table);
        ark_gc_object_table = calloc(desired, sizeof(*ark_gc_object_table));
        if (ark_gc_object_table == NULL) ark_rt_trap();
        ark_gc_object_table_cap = desired;
        ark_gc_object_table_len = 0;
    } else {
        ark_gc_table_clear();
    }
    for (ark_gc_allocation *node = ark_gc_heap_head; node != NULL; node = node->next) {
        ark_gc_table_insert(node);
    }
}

static void ark_gc_set_kind(void *object, uint8_t kind) {
    if (!ark_gc_mode || object == NULL) return;
    if (ark_gc_table_find(object) == NULL) return;
    ark_gc_header_from_object(object)->reserved[0] = kind;
}

static uint8_t ark_gc_kind(void *object) {
    if (!ark_gc_mode || object == NULL) return ARK_GC_KIND_RAW;
    if (ark_gc_table_find(object) == NULL) return ARK_GC_KIND_RAW;
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

static void ark_gc_mark_object(ark_object_header *object) {
    if (object == NULL) return;
    /* Ignore scalar bit-patterns that are not heap object addresses. */
    ark_gc_allocation *header = ark_gc_table_find(object);
    if (header == NULL) return;
    if (header->mark) return;
    header->mark = 1;
    uint8_t kind = header->reserved[0];
    if (kind == ARK_GC_KIND_STRING || kind == ARK_GC_KIND_RAW) {
        return;
    }
    if (kind == ARK_GC_KIND_VEC) {
        ark_vec *vector = (ark_vec *)object;
        if (vector->data == NULL) return;
        for (uint32_t i = 0; i < vector->length; i += 1) {
            ark_gc_mark_value(vector->data[i]);
        }
        return;
    }
    if (kind == ARK_GC_KIND_STRUCT) {
        ark_struct_object *structure = (ark_struct_object *)object;
        for (uint32_t i = 0; i < structure->field_count; i += 1) {
            ark_gc_mark_value(structure->fields[i]);
        }
    }
}

static void ark_gc_mark_roots(void) {
    for (ark_gc_frame *frame = ark_gc_frame_top; frame != NULL; frame = frame->parent) {
        for (size_t i = 0; i < frame->slot_count; i += 1) {
            ark_object_header **slot = frame->slots[i];
            if (slot != NULL && *slot != NULL) ark_gc_mark_object(*slot);
        }
    }
    if (ark_process_args != NULL) {
        ark_gc_mark_object((ark_object_header *)ark_process_args);
    }
}

static void ark_gc_table_maybe_shrink(size_t live_count) {
    /* Shrink policy is applied inside ark_gc_table_rebuild_from_heap. */
    (void)live_count;
}

void ark_gc_collect(void) {
    if (!ark_gc_mode || ark_gc_collecting) return;
    ark_gc_collecting = 1;
    ark_collection_count += 1;
    for (ark_gc_allocation *node = ark_gc_heap_head; node != NULL; node = node->next) {
        node->mark = 0;
    }
    ark_gc_mark_roots();
    ark_gc_allocation *live_head = NULL;
    uint64_t live = 0;
    uint64_t reclaimed = 0;
    size_t live_count = 0;
    ark_gc_allocation *node = ark_gc_heap_head;
    while (node != NULL) {
        ark_gc_allocation *next = node->next;
        if (node->mark) {
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
            if (ark_committed_bytes >= (sizeof(ark_gc_allocation) + node->allocation_size)) {
                ark_committed_bytes -= sizeof(ark_gc_allocation) + node->allocation_size;
            }
            free(node);
        }
        node = next;
    }
    ark_gc_heap_head = live_head;
    ark_gc_table_rebuild_from_heap();
    ark_gc_table_maybe_shrink(live_count);
    ark_live_bytes = live;
    ark_reclaimed_bytes += reclaimed;
    ark_gc_bytes_since_collection = 0;
    if (ark_gc_threshold_override != 0) {
        ark_gc_threshold_bytes = ark_gc_threshold_override;
    } else {
        /* Keep pressure high enough for the 2.4 GiB RSS gate (measured ~1.5 GiB). */
        ark_gc_threshold_bytes = live / 2ull;
        if (ark_gc_threshold_bytes < ARK_GC_INITIAL_THRESHOLD) {
            ark_gc_threshold_bytes = ARK_GC_INITIAL_THRESHOLD;
        }
    }
    malloc_trim(0);
    ark_gc_collecting = 0;
}

void ark_gc_push_frame(size_t slot_count) {
    if (!ark_gc_mode) return;
    ark_gc_frame *frame = ark_gc_frame_free;
    if (frame != NULL) {
        ark_gc_frame_free = frame->parent;
    } else {
        frame = malloc(sizeof(*frame));
        if (frame == NULL) ark_rt_trap();
        frame->slots = NULL;
        frame->slot_capacity = 0;
    }
    if (slot_count > frame->slot_capacity) {
        ark_object_header ***slots = realloc(frame->slots, slot_count * sizeof(*slots));
        if (slots == NULL) ark_rt_trap();
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
    if (!ark_gc_collecting && ark_gc_bytes_since_collection >= ark_gc_threshold_bytes) {
        ark_gc_collect();
    }
    size_t prefix = sizeof(ark_gc_allocation);
    size_t total = ark_checked_add(prefix, size);
    ark_gc_allocation *header = NULL;
    void *block = NULL;
    if (posix_memalign(&block, 16u, total) != 0 || block == NULL) ark_rt_trap();
    header = (ark_gc_allocation *)block;
    ark_committed_bytes += total;
    header->next = ark_gc_heap_head;
    header->allocation_size = size;
    header->mark = 0;
    memset(header->reserved, 0, sizeof(header->reserved));
    ark_gc_heap_head = header;
    ark_gc_table_insert(header);
    ark_requested_bytes += size;
    ark_gc_bytes_since_collection += size;
    ark_live_bytes += size;
    ark_gc_object_bytes += size;
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
    ark_gc_frame_top = NULL;
    ark_gc_frame_free = NULL;
    ark_gc_object_table = NULL;
    ark_gc_object_table_cap = 0;
    ark_gc_object_table_len = 0;
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
    for (int index = 0; index < argc; index += 1) {
        size_t length = strlen(argv[index]);
        if (length > UINT32_MAX) ark_rt_trap();
        ark_value value;
        value.ref = (ark_object_header *)ark_rt_string_from_bytes(
            (const uint8_t *)argv[index], (uint32_t)length
        );
        ark_rt_vec_push(ark_process_args, value);
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
        free(node);
        node = next;
    }
    node = ark_gc_free_list;
    while (node != NULL) {
        ark_gc_allocation *next = node->next;
        free(node);
        node = next;
    }
    ark_gc_heap_head = NULL;
    ark_gc_free_list = NULL;
    free(ark_gc_object_table);
    ark_gc_object_table = NULL;
    ark_gc_object_table_cap = 0;
    ark_gc_object_table_len = 0;
}

void ark_rt_trap(void) {
    if (ark_gc_mode) {
        ark_gc_dump_crash_state("ark_rt_trap");
    }
    abort();
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
    if (structure == NULL || field_index >= structure->field_count) ark_rt_trap();
    return structure->fields[field_index];
}

void ark_rt_struct_set(ark_object_header *object, uint32_t field_index, ark_value value) {
    ark_struct_object *structure = (ark_struct_object *)object;
    if (structure == NULL || field_index >= structure->field_count) ark_rt_trap();
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
    if (bytes == NULL) ark_rt_trap();
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
    if (source == NULL) ark_rt_trap();
    return source;
}

ark_string *ark_rt_string_concat(ark_string *left, ark_string *right) {
    if (left == NULL || right == NULL) ark_rt_trap();
    uint32_t length = left->byte_length + right->byte_length;
    if (length < left->byte_length) ark_rt_trap();
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
    if (source == NULL || start < 0 || end < start || (uint32_t)end > source->byte_length) {
        ark_rt_trap();
    }
    return ark_rt_string_from_bytes(source->bytes + start, (uint32_t)(end - start));
}

int32_t ark_rt_string_len(ark_string *source) {
    if (source == NULL || source->byte_length > INT32_MAX) ark_rt_trap();
    return (int32_t)source->byte_length;
}

int32_t ark_rt_string_char_at(ark_string *source, int32_t index) {
    if (source == NULL || index < 0 || (uint32_t)index >= source->byte_length) ark_rt_trap();
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
        return &parsed->header;
    }
    parsed->fields[0].i32 = 0;
    parsed->fields[1].f64 = result;
    return &parsed->header;
}

static void ark_vec_reserve(ark_vec *vector, uint32_t minimum);

ark_vec *ark_rt_vec_new(uint32_t type_id) {
    ark_vec *vector = ark_rt_alloc_aligned(sizeof(*vector), 16u);
    ark_gc_set_kind(vector, ARK_GC_KIND_VEC);
    vector->header.type_id = type_id;
    vector->header.flags = 0;
    return vector;
}

ark_vec *ark_rt_vec_new_with_capacity(uint32_t type_id, int32_t capacity) {
    if (capacity < 0) ark_rt_trap();
    ark_vec *vector = ark_rt_vec_new(type_id);
    ark_vec_reserve(vector, (uint32_t)capacity);
    return vector;
}

static void ark_vec_reserve(ark_vec *vector, uint32_t minimum) {
    if (vector == NULL) ark_rt_trap();
    if (vector->capacity >= minimum) return;
    uint32_t capacity = vector->capacity == 0 ? 4u : vector->capacity;
    while (capacity < minimum) {
        if (capacity > UINT32_MAX / 2u) ark_rt_trap();
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
    if (vector == NULL || vector->length > INT32_MAX) ark_rt_trap();
    return (int32_t)vector->length;
}

ark_value ark_rt_vec_get(ark_vec *vector, int32_t index) {
    if (vector == NULL || index < 0 || (uint32_t)index >= vector->length) ark_rt_trap();
    return vector->data[index];
}

ark_unit ark_rt_vec_set(ark_vec *vector, int32_t index, ark_value value) {
    if (vector == NULL || index < 0 || (uint32_t)index >= vector->length) ark_rt_trap();
    vector->data[index] = value;
    return 0;
}

ark_unit ark_rt_vec_push(ark_vec *vector, ark_value value) {
    if (vector == NULL || vector->length == UINT32_MAX) ark_rt_trap();
    ark_vec_reserve(vector, vector->length + 1u);
    vector->data[vector->length] = value;
    vector->length += 1u;
    return 0;
}

ark_value ark_rt_vec_pop(ark_vec *vector) {
    if (vector == NULL || vector->length == 0) ark_rt_trap();
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

static void ark_write_file(ark_string *path, ark_string *text) {
    FILE *file = fopen(ark_path(path), "wb");
    if (file == NULL) ark_rt_trap();
    ark_write_stream(file, text);
    if (fclose(file) != 0) ark_rt_trap();
}

static ark_object_header *ark_write_success(void) {
    ark_object_header *result = (ark_object_header *)ark_rt_struct_new(0u, 2u);
    ark_rt_struct_set(result, 0u, (ark_value){ .i32 = 0 });
    ark_rt_struct_set(result, 1u, (ark_value){ .i32 = 0 });
    return result;
}

ark_object_header *ark_rt_write_bytes(ark_string *path, ark_vec *bytes) {
    if (bytes == NULL) ark_rt_trap();
    ark_string view;
    view.header.type_id = 0;
    view.header.flags = 0;
    view.byte_length = bytes->length;
    view.capacity = bytes->length;
    view.bytes = ark_rt_alloc_aligned(bytes->length, 16u);
    for (uint32_t index = 0; index < bytes->length; index += 1) {
        view.bytes[index] = (uint8_t)bytes->data[index].i32;
    }
    ark_write_file(path, &view);
    return ark_write_success();
}

ark_object_header *ark_rt_write_string(ark_string *path, ark_string *text) {
    ark_write_file(path, text);
    return ark_write_success();
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
    if (right == 0 || (left == INT32_MIN && right == -1)) ark_rt_trap();
    return left / right;
}

int64_t ark_div_i64(int64_t left, int64_t right) {
    if (right == 0 || (left == INT64_MIN && right == -1)) ark_rt_trap();
    return left / right;
}

int32_t ark_rem_i32(int32_t left, int32_t right) {
    if (right == 0) ark_rt_trap();
    if (left == INT32_MIN && right == -1) return 0;
    return left % right;
}

int64_t ark_rem_i64(int64_t left, int64_t right) {
    if (right == 0) ark_rt_trap();
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
