#!/usr/bin/env python3
"""Split emit_intrinsic.ark into 4 sub-modules based on function group mapping."""

import re
import os

SRC_FILE = "src/compiler/emit_intrinsic.ark"
OUT_DIR = "src/compiler"

STRING_FUNCS = {
    'emit_concat', 'emit_clone', 'emit_bool_to_string', 'emit_char_to_string',
    'emit_to_string', 'emit_i64_to_string', 'emit_f64_to_string', 'emit_f32_to_string',
    'emit_len', 'emit_push_char', 'emit_text_is_empty', 'emit_text_len_bytes',
    'emit_char_at', 'emit_pad_left', 'emit_pad_right', 'emit_trim_start', 'emit_trim_end',
    'emit_trim', 'emit_contains', 'emit_contains_String', 'emit_starts_with',
    'emit_ends_with', 'emit_repeat', 'emit_to_upper', 'emit_to_lower', 'emit_index_of',
    'emit_replace', 'emit_slice', 'emit_join', 'emit_text_chars', 'emit_split',
    'emit_reverse_String', 'emit_String_new',
}

MATH_FUNCS = {
    'emit_sqrt', 'emit_parse_f64', 'emit_parse_i32', 'emit_parse_i64',
    'emit_f64_bits_lo', 'emit_f64_bits_hi', 'emit_eq', 'emit_sort_i32', 'emit_sort_i64',
    'emit_sort_f64', 'emit_abs', 'emit_min', 'emit_max', 'emit_pow_i32', 'emit_gcd',
    'emit_clamp', 'emit_range_new', 'emit_range_contains', 'emit_range_len',
    'emit_sum_i32', 'emit_product_i32', 'emit_seq_min_i32', 'emit_seq_max_i32',
    'emit_seq_count_eq', 'emit_seq_binary_search', 'emit_seq_unique', 'emit_seq_take_i32',
    'emit_seq_skip_i32', 'emit_memory_copy', 'emit_memory_fill',
}

VEC_FUNCS = {
    'emit_Vec_new_i64', 'emit_vec_len', 'emit_push', 'emit_push_i64',
    'emit_push_f64', 'emit_get_unchecked', 'emit_vec_get_unchecked_i64',
    'emit_vec_get_unchecked_f64', 'emit_vec_get', 'emit_vec_pop', 'emit_vec_set',
    'emit_reverse_i32', 'emit_remove_i32', 'emit_contains_i32', 'emit_is_empty',
}

IO_FUNCS = {
    'emit_println', 'emit_print', 'emit_eprintln', 'emit_process_exit',
    'emit_env_var', 'emit_env_get_var', 'emit_env_var_or_default', 'emit_env_arg_count',
    'emit_env_args', 'emit_env_has_flag', 'emit_env_arg_at', 'emit_fs_exists',
    'emit_fs_read_to_string', 'emit_fs_write_string', 'emit_fs_write_bytes',
    'emit_assert', 'emit_assert_eq', 'emit_assert_eq_i64', 'emit_assert_ne',
    'emit_assert_eq_str', 'emit_unwrap_l4771', 'emit_unwrap_or_l4779',
    'emit_unwrap_l4827', 'emit_unwrap_or_l4835', 'emit_is_some', 'emit_is_none',
    'emit_is_ok', 'emit_is_err', 'emit_i32_to_i64', 'emit_i64_to_i32', 'emit_i8_to_i32',
    'emit_i32_to_u8', 'emit_u32_to_u64', 'emit_i32_to_u32', 'emit_f32_to_f64',
}

MODULE_MAP = {}
for func in STRING_FUNCS:
    MODULE_MAP[func] = "emit_intrinsic_string"
for func in MATH_FUNCS:
    MODULE_MAP[func] = "emit_intrinsic_math"
for func in VEC_FUNCS:
    MODULE_MAP[func] = "emit_intrinsic_vec"
for func in IO_FUNCS:
    MODULE_MAP[func] = "emit_intrinsic_io"

# Verify we have a mapping for all expected functions
FUNC_PATTERN = re.compile(r'^pub fn (emit_\w+)\(.*\) -> bool \{')


def split_file():
    with open(SRC_FILE, 'r') as f:
        content = f.read()

    lines = content.split('\n')

    # First pass: find all function start lines and their function names
    func_starts = []  # list of (line_index, func_name)
    for i, line in enumerate(lines):
        m = FUNC_PATTERN.match(line)
        if m:
            func_starts.append((i, m.group(1)))

    print(f"Found {len(func_starts)} functions in {SRC_FILE}")

    # Verify all functions have a mapping
    unmapped = []
    for _, fname in func_starts:
        if fname not in MODULE_MAP:
            unmapped.append(fname)
    if unmapped:
        print(f"ERROR: Unmapped functions: {unmapped}")
        return False

    # Verify no duplicate mappings to different modules
    for _, fname in func_starts:
        mod = MODULE_MAP[fname]

    # Group start indices by module
    # We need to find the body end for each function
    # Strategy: for each function, track brace depth from its pub fn line

    module_funcs = {"emit_intrinsic_string": [], "emit_intrinsic_math": [],
                    "emit_intrinsic_vec": [], "emit_intrinsic_io": []}

    for i, (line_idx, fname) in enumerate(func_starts):
        mod = MODULE_MAP[fname]
        # Find the end of this function
        # Start from the line with `pub fn` and track brace depth
        depth = 0
        started = False
        end_idx = line_idx
        for j in range(line_idx, len(lines)):
            line = lines[j]
            for ch in line:
                if ch == '{':
                    depth += 1
                    started = True
                elif ch == '}':
                    depth -= 1
            if started and depth == 0:
                end_idx = j
                break

        # Extract the function text (including the surrounding blank lines for separation)
        # We'll extract from the line before the function to the line after its closing brace
        func_text_lines = []
        start_incl = line_idx
        end_incl = end_idx

        # Include comments/blank lines before the function
        k = line_idx - 1
        while k >= 0:
            stripped = lines[k].strip()
            if stripped == '' or stripped.startswith('//'):
                start_incl = k
                k -= 1
            else:
                break

        # Include blank line after the function
        if end_idx + 1 < len(lines) and lines[end_idx + 1].strip() == '':
            end_incl = end_idx + 1

        func_text = '\n'.join(lines[start_incl:end_incl + 1])
        module_funcs[mod].append((start_incl, end_incl, fname, func_text))

    # Write output files
    headers = {
        "emit_intrinsic_string": "// Arukellt Selfhost — Wasm Binary Emitter: String Intrinsic Handlers\n// Extracted from emit_intrinsic.ark\n\nuse emit_inst_ctx\n\n",
        "emit_intrinsic_math": "// Arukellt Selfhost — Wasm Binary Emitter: Math Intrinsic Handlers\n// Extracted from emit_intrinsic.ark\n\nuse emit_inst_ctx\n\n",
        "emit_intrinsic_vec": "// Arukellt Selfhost — Wasm Binary Emitter: Vec Intrinsic Handlers\n// Extracted from emit_intrinsic.ark\n\nuse emit_inst_ctx\n\n",
        "emit_intrinsic_io": "// Arukellt Selfhost — Wasm Binary Emitter: I/O and Misc Intrinsic Handlers\n// Extracted from emit_intrinsic.ark\n\nuse emit_inst_ctx\n\n",
    }

    total_funcs = 0
    for mod_name in ["emit_intrinsic_string", "emit_intrinsic_math", "emit_intrinsic_vec", "emit_intrinsic_io"]:
        funcs = module_funcs[mod_name]
        # Sort by original line position to maintain order
        funcs.sort(key=lambda x: x[0])

        out_path = os.path.join(OUT_DIR, f"{mod_name}.ark")
        with open(out_path, 'w') as f:
            f.write(headers[mod_name])
            for _, _, fname, text in funcs:
                f.write(text)
                f.write('\n')

        line_count = sum(len(text.split('\n')) for _, _, _, text in funcs)
        print(f"Wrote {len(funcs)} functions ({line_count} lines) to {mod_name}.ark")
        total_funcs += len(funcs)

    print(f"\nTotal: {total_funcs} functions split across 4 files")

    # Verify: check that all functions across modules cover all original functions
    all_split_funcs = set()
    for mod_funcs in module_funcs.values():
        for _, _, fname, _ in mod_funcs:
            all_split_funcs.add(fname)

    original_funcs = set(fname for _, fname in func_starts)
    missing = original_funcs - all_split_funcs
    extra = all_split_funcs - original_funcs
    if missing:
        print(f"ERROR: Missing functions in output: {missing}")
        return False
    if extra:
        print(f"WARNING: Extra functions in output (possibly duplicates): {extra}")

    return True


if __name__ == "__main__":
    success = split_file()
    if success:
        print("\nSplit completed successfully!")
    else:
        print("\nSplit FAILED!")
        exit(1)
