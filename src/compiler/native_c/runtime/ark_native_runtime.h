#ifndef ARK_NATIVE_RUNTIME_H
#define ARK_NATIVE_RUNTIME_H

#include <stddef.h>
#include <stdint.h>

#define ARK_NATIVE_RUNTIME_ABI_VERSION 1u

typedef uint8_t ark_unit;

typedef struct {
    uint32_t type_id;
    uint32_t flags;
} ark_object_header;

typedef union ark_value {
    int32_t i32;
    int64_t i64;
    float f32;
    double f64;
    ark_object_header *ref;
} ark_value;

typedef struct {
    ark_object_header header;
    uint32_t field_count;
    ark_value fields[];
} ark_struct_object;

typedef struct {
    ark_object_header header;
    uint8_t *bytes;
    uint32_t byte_length;
    uint32_t capacity;
} ark_string;

typedef struct {
    ark_object_header header;
    ark_value *data;
    uint32_t length;
    uint32_t capacity;
} ark_vec;


typedef struct ark_gc_allocation {
    struct ark_gc_allocation *next;
    size_t allocation_size;
    uint8_t mark;
    uint8_t reserved[7];
} ark_gc_allocation;

void ark_gc_push_frame(size_t slot_count);
void ark_gc_pop_frame(void);
void ark_gc_set_root(size_t slot, ark_object_header **slot_ptr);
/** Clear rooted locals by GC frame slot index (compact codegen for safepoints). */
void ark_gc_clear_root_slots(size_t count, const size_t *slots);
void ark_gc_collect(void);
void ark_gc_set_current_function(const char *name);

uint64_t ark_rt_stats_requested_bytes(void);
uint64_t ark_rt_stats_committed_bytes(void);
uint64_t ark_rt_stats_live_bytes(void);
uint64_t ark_rt_stats_collection_count(void);
uint64_t ark_rt_stats_reclaimed_bytes(void);
uint32_t ark_rt_stats_chunk_count(void);

typedef enum ark_trap_kind {
    ARK_TRAP_GENERIC = 0,
    ARK_TRAP_BOUNDS = 1,
    ARK_TRAP_DIV_BY_ZERO = 2,
    ARK_TRAP_NULL_REF = 3,
    ARK_TRAP_ALLOC = 4,
    ARK_TRAP_OOM = 5,
    ARK_TRAP_INVALID_CAST = 6
} ark_trap_kind;

void ark_rt_init(int argc, char **argv);
void ark_rt_shutdown(void);
void ark_rt_trap(void);
void ark_rt_trap_kind(ark_trap_kind kind);
void ark_rt_panic(ark_string *message);
void *ark_rt_alloc_aligned(size_t size, size_t alignment);
ark_struct_object *ark_rt_struct_new(uint32_t type_id, uint32_t field_count);
ark_value ark_rt_struct_get(ark_object_header *object, uint32_t field_index);
void ark_rt_struct_set(ark_object_header *object, uint32_t field_index, ark_value value);

ark_string *ark_rt_string_from_bytes(const uint8_t *bytes, uint32_t length);
ark_string *ark_rt_string_from_vec_bytes(ark_vec *bytes);
ark_string *ark_rt_string_clone(ark_string *source);
ark_string *ark_rt_string_concat(ark_string *left, ark_string *right);
ark_string *ark_rt_string_slice(ark_string *source, int32_t start, int32_t end);
int32_t ark_rt_string_len(ark_string *source);
int32_t ark_rt_string_char_at(ark_string *source, int32_t index);
int32_t ark_rt_string_eq(ark_string *left, ark_string *right);
int32_t ark_rt_string_contains(ark_string *source, ark_string *needle);
int32_t ark_rt_string_starts_with(ark_string *source, ark_string *prefix);
int32_t ark_rt_string_ends_with(ark_string *source, ark_string *suffix);
int32_t ark_rt_string_index_of(ark_string *source, ark_string *needle);
ark_string *ark_rt_char_to_string(uint32_t value);
ark_string *ark_rt_i32_to_string(int32_t value);
ark_string *ark_rt_i64_to_string(int64_t value);
ark_string *ark_rt_f64_to_string(double value);
ark_object_header *ark_rt_parse_f64(ark_string *source);
ark_object_header *ark_rt_parse_i32(ark_string *source);
ark_object_header *ark_rt_parse_i64(ark_string *source);
void ark_rt_assert(int32_t condition);
void ark_rt_assert_eq_i32(int32_t left, int32_t right);
void ark_rt_assert_eq_i64(int64_t left, int64_t right);
void ark_rt_assert_eq_str(ark_string *left, ark_string *right);

ark_string *ark_rt_bool_to_string(int32_t value);
ark_string *ark_rt_string_trim(ark_string *source);
ark_string *ark_rt_string_trim_start(ark_string *source);
ark_string *ark_rt_string_trim_end(ark_string *source);
ark_string *ark_rt_string_replace(ark_string *source, ark_string *from, ark_string *to);
ark_string *ark_rt_string_repeat(ark_string *source, int32_t count);
ark_string *ark_rt_string_to_lowercase(ark_string *source);
ark_string *ark_rt_string_to_uppercase(ark_string *source);
ark_string *ark_rt_string_reverse(ark_string *source);
ark_string *ark_rt_string_join(ark_vec *parts, ark_string *separator);
ark_vec *ark_rt_string_split(ark_string *source, ark_string *separator, uint32_t type_id);
ark_vec *ark_rt_string_lines(ark_string *source, uint32_t type_id);
ark_vec *ark_rt_string_chars(ark_string *source, uint32_t type_id);
int32_t ark_rt_string_is_empty(ark_string *source);

int32_t ark_rt_is_ok(ark_object_header *value);
int32_t ark_rt_is_err(ark_object_header *value);
ark_value ark_rt_result_unwrap(ark_object_header *value);
ark_value ark_rt_result_unwrap_or(ark_object_header *value, ark_value fallback);

int32_t ark_rt_vec_is_empty(ark_vec *vector);

int32_t ark_rt_math_abs_i32(int32_t value);
int32_t ark_rt_math_min_i32(int32_t left, int32_t right);
int32_t ark_rt_math_max_i32(int32_t left, int32_t right);
int32_t ark_rt_math_clamp_i32(int32_t value, int32_t low, int32_t high);
int32_t ark_rt_next_power_of_two_i32(int32_t value);
int32_t ark_rt_popcount_i32(int32_t value);
int32_t ark_rt_popcount_i64(int64_t value);
int32_t ark_rt_leading_zeros_i32(int32_t value);
int32_t ark_rt_leading_zeros_i64(int64_t value);
int32_t ark_rt_trailing_zeros_i32(int32_t value);
int32_t ark_rt_trailing_zeros_i64(int64_t value);
int32_t ark_rt_is_power_of_two_i32(int32_t value);
int32_t ark_rt_is_power_of_two_i64(int64_t value);
int32_t ark_rt_math_gcd_i32(int32_t left, int32_t right);
int32_t ark_rt_math_pow_i32(int32_t base, int32_t exp);
double ark_rt_math_sqrt_f64(double value);

ark_vec *ark_rt_vec_new(uint32_t type_id);
ark_vec *ark_rt_vec_new_with_capacity(uint32_t type_id, int32_t capacity);
ark_vec *ark_rt_array_new(uint32_t type_id, int32_t length);
int32_t ark_rt_vec_len(ark_vec *vector);
ark_value ark_rt_vec_get(ark_vec *vector, int32_t index);
ark_object_header *ark_rt_vec_get_option(ark_vec *vector, int32_t index, uint32_t option_type_id);
void ark_rt_vec_sort_i32(ark_vec *vector);
int32_t ark_rt_vec_sum_i32(ark_vec *vector);
int32_t ark_rt_vec_product_i32(ark_vec *vector);
ark_unit ark_rt_vec_reverse(ark_vec *vector);
ark_unit ark_rt_vec_remove(ark_vec *vector, int32_t index);
int32_t ark_rt_vec_contains_i32(ark_vec *vector, int32_t value);
ark_unit ark_rt_vec_set(ark_vec *vector, int32_t index, ark_value value);
ark_unit ark_rt_vec_push(ark_vec *vector, ark_value value);
ark_value ark_rt_vec_pop(ark_vec *vector);
int32_t ark_rt_arg_count(void);
ark_unit ark_rt_string_push_char(ark_string *string, uint32_t codepoint);

ark_vec *ark_rt_args(void);
ark_string *ark_rt_read_stdin(void);
ark_object_header *ark_rt_fs_read_file(ark_string *path);
ark_unit ark_rt_print(ark_string *text);
ark_unit ark_rt_println(ark_string *text);
ark_unit ark_rt_eprintln(ark_string *text);
ark_object_header *ark_rt_write_bytes(ark_string *path, ark_vec *bytes);
ark_object_header *ark_rt_write_string(ark_string *path, ark_string *text);
ark_unit ark_rt_process_exit(int32_t status);
int64_t ark_rt_clock_now_ms(void);
int32_t ark_rt_f64_bits_hi(double value);
int32_t ark_rt_f64_bits_lo(double value);

int32_t ark_div_i32(int32_t left, int32_t right);
int64_t ark_div_i64(int64_t left, int64_t right);
int32_t ark_rem_i32(int32_t left, int32_t right);
int64_t ark_rem_i64(int64_t left, int64_t right);
double ark_rem_f64(double left, double right);
int32_t ark_shl_i32(int32_t left, int32_t right);
int64_t ark_shl_i64(int64_t left, int64_t right);
int32_t ark_shr_i32(int32_t left, int32_t right);
int64_t ark_shr_i64(int64_t left, int64_t right);

#endif
