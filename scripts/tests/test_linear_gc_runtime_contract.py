from __future__ import annotations

from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]


class LinearGcRuntimeContractTest(unittest.TestCase):
    def test_runtime_has_allocator_root_and_cycle_collection_boundaries(self) -> None:
        runtime = (ROOT / "src/compiler/wasm/linear_gc_runtime.ark").read_text(
            encoding="utf-8"
        )
        required_symbols = (
            "emit_linear_gc_runtime_root_frame_enter_body",
            "emit_linear_gc_runtime_root_push_body",
            "emit_linear_gc_runtime_root_update_body",
            "emit_linear_gc_runtime_root_frame_leave_body",
            "emit_linear_gc_runtime_commit_top_body",
            "emit_linear_gc_runtime_mark_candidate_body",
            "emit_linear_gc_runtime_collect_body",
            "emit_linear_gc_runtime_alloc_body",
        )
        for symbol in required_symbols:
            self.assertIn(f"fn {symbol}", runtime)
        self.assertIn("emit_linear_gc_mark_transitive", runtime)
        self.assertIn("emit_linear_gc_sweep", runtime)
        self.assertNotIn("GC_PREFIX()", runtime)
        self.assertNotIn("GC_STRUCT_", runtime)
        self.assertNotIn("GC_ARRAY_", runtime)

    def test_linear_target_wires_recording_without_enabling_gc_target_runtime(self) -> None:
        target = (ROOT / "src/compiler/wasm/emit_target.ark").read_text(encoding="utf-8")
        code = (ROOT / "src/compiler/wasm/code.ark").read_text(encoding="utf-8")
        heap = (ROOT / "src/compiler/wasm/intrinsics/helpers_core_heap.ark").read_text(
            encoding="utf-8"
        )
        self.assertIn("fn is_linear_memory_target", target)
        self.assertIn("linear_gc_runtime_function_count", code)
        self.assertIn("writer_set_linear_gc_commit_function_index", code)
        self.assertIn("writer_linear_gc_commit_function_index", heap)
        self.assertIn("emit_heap_get(w)", heap)
        self.assertIn("OP_CALL()", heap)

    def test_managed_aggregate_constructors_use_private_allocator(self) -> None:
        writer = (ROOT / "src/compiler/wasm/writer_core.ark").read_text(encoding="utf-8")
        code = (ROOT / "src/compiler/wasm/code.ark").read_text(encoding="utf-8")
        struct = (ROOT / "src/compiler/wasm/inst_struct_record.ark").read_text(encoding="utf-8")
        array = (ROOT / "src/compiler/wasm/inst_array.ark").read_text(encoding="utf-8")
        vec = (ROOT / "src/compiler/wasm/intrinsic_vec_new_layout.ark").read_text(encoding="utf-8")
        string_basic = (ROOT / "src/compiler/wasm/intrinsic_string_basic.ark").read_text(encoding="utf-8")
        string_from_bytes = (ROOT / "src/compiler/wasm/intrinsic_string_from_bytes.ark").read_text(encoding="utf-8")
        string_slice = (ROOT / "src/compiler/wasm/intrinsic_string_slice.ark").read_text(encoding="utf-8")
        string_trim = (ROOT / "src/compiler/wasm/intrinsic_string_trim_helpers.ark").read_text(encoding="utf-8")
        string_trim_full = (ROOT / "src/compiler/wasm/intrinsic_string_trim_full_helpers.ark").read_text(encoding="utf-8")
        string_case = (ROOT / "src/compiler/wasm/intrinsic_string_case.ark").read_text(encoding="utf-8")
        string_join = (ROOT / "src/compiler/wasm/intrinsic_string_join.ark").read_text(encoding="utf-8")
        string_repeat = (ROOT / "src/compiler/wasm/intrinsic_string_repeat.ark").read_text(encoding="utf-8")
        string_pad = (ROOT / "src/compiler/wasm/intrinsic_string_pad_helpers.ark").read_text(encoding="utf-8")
        string_replace = (ROOT / "src/compiler/wasm/intrinsic_string_replace_all_helpers.ark").read_text(encoding="utf-8")
        string_split = (ROOT / "src/compiler/wasm/intrinsic_string_split.ark").read_text(encoding="utf-8")
        string_chars = (ROOT / "src/compiler/wasm/intrinsic_string_chars.ark").read_text(encoding="utf-8")
        format_i32 = (ROOT / "src/compiler/wasm/intrinsic_string_format_i32.ark").read_text(encoding="utf-8")
        format_i64 = (ROOT / "src/compiler/wasm/intrinsic_string_format_i64.ark").read_text(encoding="utf-8")
        format_f64 = (ROOT / "src/compiler/wasm/intrinsic_string_format_f64_assemble.ark").read_text(encoding="utf-8")
        format_char = (ROOT / "src/compiler/wasm/intrinsic_string_format_char.ark").read_text(encoding="utf-8")
        vec_push = (ROOT / "src/compiler/wasm/intrinsic_vec_push.ark").read_text(encoding="utf-8")
        vec_raw_grow = (ROOT / "src/compiler/wasm/intrinsic_vec_raw_grow.ark").read_text(encoding="utf-8")
        vec_access = (ROOT / "src/compiler/wasm/intrinsic_vec_access.ark").read_text(encoding="utf-8")
        vec_pop = (ROOT / "src/compiler/wasm/intrinsic_vec_pop.ark").read_text(encoding="utf-8")
        range_source = (ROOT / "src/compiler/wasm/intrinsic_range.ark").read_text(encoding="utf-8")
        parse_result = (ROOT / "src/compiler/wasm/intrinsic_parse_result.ark").read_text(encoding="utf-8")
        parse_result_errors = (ROOT / "src/compiler/wasm/intrinsic_parse_result_errors.ark").read_text(encoding="utf-8")

        self.assertIn("linear_gc_alloc_function_index", writer)
        self.assertIn("writer_set_linear_gc_alloc_function_index", code)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", struct)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", array)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", vec)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", string_basic)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", string_from_bytes)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", string_slice)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", string_trim)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", string_trim_full)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", string_case)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", string_join)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", string_repeat)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", string_pad)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", string_replace)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", string_split)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", string_chars)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", format_i32)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", format_i64)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", format_f64)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", format_char)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", vec_push)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", vec_raw_grow)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", vec_access)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", vec_pop)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", range_source)
        self.assertNotIn("helpers_core::emit_heap_set(w)", range_source)
        self.assertNotIn("helpers_core_heap::emit_heap_set(w)", range_source)
        f64_ok_start = parse_result.index("fn emit_f64_ok_result")
        f64_ok_end = parse_result.index("fn emit_i32_ok_result")
        f64_ok = parse_result[f64_ok_start:f64_ok_end]
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", f64_ok)
        self.assertNotIn("helpers_core::emit_heap_set(w)", f64_ok)
        float_error_string_start = parse_result_errors.index("fn emit_invalid_float_error_string")
        float_error_record_start = parse_result_errors.index("fn emit_invalid_float_error_record")
        integer_error_string_start = parse_result_errors.index("fn emit_invalid_integer_error_string")
        float_error_string = parse_result_errors[float_error_string_start:float_error_record_start]
        float_error_record = parse_result_errors[float_error_record_start:integer_error_string_start]
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", float_error_string)
        self.assertNotIn("helpers_core::emit_heap_set(w)", float_error_string)
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", float_error_record)
        self.assertNotIn("helpers_core::emit_heap_set(w)", float_error_record)
        integer_ok_start = parse_result.index("fn emit_integer_ok_header")
        integer_ok = parse_result[integer_ok_start:]
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", integer_ok)
        self.assertNotIn("helpers_core::emit_heap_set(w)", integer_ok)
        integer_error_start = parse_result_errors.index("fn emit_invalid_integer_error_string")
        integer_error_record_start = parse_result_errors.index("fn emit_invalid_integer_error_record")
        integer_error_string = parse_result_errors[integer_error_start:integer_error_record_start]
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", integer_error_string)
        self.assertNotIn("helpers_core::emit_heap_set(w)", integer_error_string)
        integer_error_record = parse_result_errors[integer_error_record_start:]
        self.assertIn("helpers_core_heap::emit_heap_alloc_from_stack(w)", integer_error_record)
        self.assertNotIn("helpers_core::emit_heap_set(w)", integer_error_record)
        for source in (
            string_slice,
            string_trim,
            string_trim_full,
            string_case,
            string_join,
            string_repeat,
            string_pad,
            string_replace,
            string_split,
            string_chars,
            format_i32,
            format_i64,
            format_f64,
            format_char,
            vec_push,
            vec_raw_grow,
            vec_access,
            vec_pop,
        ):
            self.assertNotIn("helpers_core::emit_heap_get(w)", source)
            self.assertNotIn("helpers_core::emit_heap_set(w)", source)
            self.assertNotIn("helpers_core_heap::emit_heap_set(w)", source)

    def test_compiler_connects_linear_root_frames_without_collecting(self) -> None:
        code = (ROOT / "src/compiler/wasm/code.ark").read_text(encoding="utf-8")
        body = (ROOT / "src/compiler/wasm/code_body.ark").read_text(encoding="utf-8")
        locals_source = (ROOT / "src/compiler/wasm/inst_locals.ark").read_text(encoding="utf-8")
        returns = (ROOT / "src/compiler/wasm/inst_convert.ark").read_text(encoding="utf-8")
        scratch = (ROOT / "src/compiler/wasm/code_scratch_locals.ark").read_text(encoding="utf-8")
        lowering = (ROOT / "src/compiler/wasm/linear_root_lowering.ark").read_text(encoding="utf-8")

        for symbol in (
            "writer_set_linear_gc_root_frame_enter_function_index",
            "writer_set_linear_gc_root_push_function_index",
            "writer_set_linear_gc_root_update_function_index",
            "writer_set_linear_gc_root_frame_leave_function_index",
        ):
            self.assertIn(symbol, code)
        self.assertIn("emit_frame_enter_and_push", body)
        self.assertIn("emit_frame_leave(w, ctx)", body)
        self.assertIn("emit_update_after_local_set(w, ctx, dest)", locals_source)
        self.assertIn("linear_root_lowering::emit_frame_leave(w, ctx)", returns)
        self.assertIn("scratch_linear_root_frame_local_count", scratch)
        self.assertIn("SelfEmitCtx_linear_root_frame_local", lowering)
        self.assertNotIn("LINEAR_GC_COLLECT", lowering)

    def test_same_source_fixture_and_capability_diagnostic_remain_present(self) -> None:
        gc_source = (ROOT / "tests/fixtures/target_capability/gc_linear_same_source.ark").read_text(
            encoding="utf-8"
        )
        linear_source = (ROOT / "tests/fixtures/target_capability/wasm32_same_source.ark").read_text(
            encoding="utf-8"
        )
        self.assertEqual(gc_source, linear_source)
        self.assertIn("--target wasm32", (ROOT / "tests/fixtures/target_capability/wasm32_same_source.flags").read_text(encoding="utf-8"))
        diagnostic = ROOT / "tests/fixtures/diagnostics/target_memory_width_unavailable"
        self.assertIn("--target wasm32", diagnostic.with_suffix(".flags").read_text(encoding="utf-8"))
        self.assertIn("memory64", diagnostic.with_suffix(".diag").read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
