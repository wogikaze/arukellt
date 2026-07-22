#define _POSIX_C_SOURCE 200809L
#include "ark_native_runtime.h"

#include <errno.h>
#include <inttypes.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

typedef struct ark_arena_chunk {
    struct ark_arena_chunk *next;
    void *allocation;
    uint8_t *data;
    size_t used;
    size_t capacity;
} ark_arena_chunk;

static ark_arena_chunk *ark_arena_head;
static uint64_t ark_allocation_bytes;
static uint32_t ark_chunk_count;
static ark_vec *ark_process_args;

static size_t ark_checked_add(size_t left, size_t right) {
    if (left > SIZE_MAX - right) ark_rt_trap();
    return left + right;
}

static size_t ark_checked_mul(size_t left, size_t right) {
    if (left != 0 && right > SIZE_MAX / left) ark_rt_trap();
    return left * right;
}

static ark_arena_chunk *ark_add_chunk(size_t minimum) {
    size_t capacity = 1024u * 1024u;
    if (capacity < minimum) capacity = minimum;
    void *allocation = malloc(ark_checked_add(capacity, 15u));
    ark_arena_chunk *chunk = malloc(sizeof(*chunk));
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
    return chunk;
}

void *ark_rt_alloc_aligned(size_t size, size_t alignment) {
    if (alignment < 16u) alignment = 16u;
    if ((alignment & (alignment - 1u)) != 0u) ark_rt_trap();
    size_t required = ark_checked_add(size, alignment - 1u);
    ark_arena_chunk *chunk = ark_arena_head;
    if (chunk == NULL) chunk = ark_add_chunk(required);
    uintptr_t base = (uintptr_t)chunk->data;
    uintptr_t aligned = (base + chunk->used + alignment - 1u) & ~(uintptr_t)(alignment - 1u);
    size_t end = ark_checked_add((size_t)(aligned - base), size);
    if (end > chunk->capacity) {
        chunk = ark_add_chunk(required);
        base = (uintptr_t)chunk->data;
        aligned = (base + alignment - 1u) & ~(uintptr_t)(alignment - 1u);
        end = ark_checked_add((size_t)(aligned - base), size);
    }
    chunk->used = end;
    ark_allocation_bytes += size;
    void *result = (void *)aligned;
    memset(result, 0, size);
    return result;
}

void ark_rt_init(int argc, char **argv) {
    ark_arena_head = NULL;
    ark_allocation_bytes = 0;
    ark_chunk_count = 0;
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
    ark_arena_chunk *chunk = ark_arena_head;
    while (chunk != NULL) {
        ark_arena_chunk *next = chunk->next;
        free(chunk->allocation);
        free(chunk);
        chunk = next;
    }
    ark_arena_head = NULL;
}

void ark_rt_trap(void) {
    abort();
}

ark_struct_object *ark_rt_struct_new(uint32_t type_id, uint32_t field_count) {
    size_t size = ark_checked_add(
        offsetof(ark_struct_object, fields),
        ark_checked_mul(field_count, sizeof(ark_value))
    );
    ark_struct_object *object = ark_rt_alloc_aligned(size, 16u);
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
    result->header.type_id = 0;
    result->header.flags = 0;
    result->byte_length = length;
    result->capacity = length;
    if (length != 0) {
        result->bytes = ark_rt_alloc_aligned(length, 16u);
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
        result->bytes = ark_rt_alloc_aligned(bytes->length, 16u);
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
        result->bytes = ark_rt_alloc_aligned(length, 16u);
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
    char *buffer = ark_rt_alloc_aligned((size_t)source->byte_length + 1u, 16u);
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
    ark_value *data = ark_rt_alloc_aligned(ark_checked_mul(capacity, sizeof(*data)), 16u);
    if (vector->length != 0) memcpy(data, vector->data, vector->length * sizeof(*data));
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
        result->bytes = ark_rt_alloc_aligned(bytes->length, 16u);
        for (uint32_t index = 0; index < bytes->length; index += 1) {
            result->bytes[index] = (uint8_t)bytes->data[index].i32;
        }
    }
    return result;
}

static char *ark_path(ark_string *path) {
    if (path == NULL || memchr(path->bytes, 0, path->byte_length) != NULL) ark_rt_trap();
    char *result = ark_rt_alloc_aligned((size_t)path->byte_length + 1u, 16u);
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
