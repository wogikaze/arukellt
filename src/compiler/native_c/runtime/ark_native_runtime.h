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

void ark_rt_init(int argc, char **argv);
void ark_rt_shutdown(void);
void ark_rt_trap(void);
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

ark_vec *ark_rt_vec_new(uint32_t type_id);
ark_vec *ark_rt_vec_new_with_capacity(uint32_t type_id, int32_t capacity);
int32_t ark_rt_vec_len(ark_vec *vector);
ark_value ark_rt_vec_get(ark_vec *vector, int32_t index);
ark_unit ark_rt_vec_set(ark_vec *vector, int32_t index, ark_value value);
ark_unit ark_rt_vec_push(ark_vec *vector, ark_value value);
ark_value ark_rt_vec_pop(ark_vec *vector);

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
