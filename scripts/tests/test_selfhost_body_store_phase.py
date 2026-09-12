"""Guard the selfhost body-store phase boundary and serialized scans."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def _read(relative_path: str) -> str:
    return (ROOT / relative_path).read_text(encoding="utf-8")


def test_body_store_compaction_stays_between_mir_verify_and_emit() -> None:
    entry = _read("src/compiler/mir/lower/entry.ark")
    backend = _read("src/compiler/driver/pipeline_backend.ark")

    assert "MirModule_compact_before_emission" not in entry
    verify_pos = backend.index("mir_api::verify_mir_pipeline(mir_module)")
    compact_pos = backend.index("body_store::MirModule_compact_before_emission(mir_module)")
    emit_pos = backend.index("emit::emit_output(")
    assert verify_pos < compact_pos < emit_pos


def test_lowering_uses_body_store_from_function_discovery_for_all_targets() -> None:
    entry = _read("src/compiler/mir/lower/entry.ark")
    enable_pos = entry.index("body_store::MirModule_enable_body_store(m)")
    assert "eq(clone(input.target), String_from(\"wasm32-gc\"))" not in entry[:enable_pos]


def test_reachability_consumes_recorded_edges_at_a_named_boundary() -> None:
    entry = _read("src/compiler/mir/lower/entry.ark")
    edges = _read("src/compiler/mir/lower/ctx_call_edges.ark")
    walk = _read("src/compiler/mir/reachability_walk.ark")

    assert "fn mir_lower_prune_reachable(" in entry
    assert "ctx_edge_table(ctx)" in entry
    assert "fn ctx_edge_table(ctx: LowerCtx) -> Vec<i32>" in edges
    assert "fn ctx_edge_fn_names(ctx: LowerCtx) -> Vec<String>" in edges
    assert "fn ctx_edge_overflow_names(ctx: LowerCtx) -> Vec<String>" in edges
    assert "struct MirRecordedEdgeTable" in walk
    assert "mir_build_recorded_edge_table" in walk
    assert "mir_build_recorded_edge_lists" not in walk
    assert "events: Vec<i32>" in walk
    assert "targets: Vec<i32>" not in walk
    assert "next: Vec<i32>" not in walk


def test_local_only_mir_passes_do_not_replace_serialized_bodies() -> None:
    module_functions = _read("src/compiler/mir/module_functions.ark")
    typed_sync = _read("src/compiler/mir/typed_mir_sync_module.ark")
    propagate = _read("src/compiler/mir/post_pass_type_propagate.ark")

    assert "fn MirModule_set_function_header_at" in module_functions
    assert "MirModule_set_function_header_at(m, fi, synced)" in typed_sync
    assert "MirModule_set_function_header_at(m, fi, f)" in propagate


def test_body_store_push_keeps_only_function_headers() -> None:
    module_functions = _read("src/compiler/mir/module_functions.ark")
    push_pos = module_functions.index("fn MirModule_push_function")
    push_body = module_functions[push_pos:]

    assert "body_store::MirModule_store_function_body(m, idx, f)" in push_body
    assert "MirFunction_set_blocks(f, Vec::new<MirBlock>())" in push_body
    assert "MirFunction_set_inst_table(" in push_body
    assert "push(m.functions, f)" in push_body


def test_emitter_reuses_one_decoded_body_scratch() -> None:
    table = _read("src/compiler/mir/inst_table.ark")
    body_store = _read("src/compiler/mir/body_store.ark")
    code = _read("src/compiler/wasm/code.ark")

    assert "pub fn mir_inst_table_reset(table: MirInstTable)" in table
    assert "pub fn MirModule_function_body_at_reusable" in body_store
    assert "SelfEmitCtx_current_function(ctx)" in code
    assert "MirModule_function_body_for_emit_at_reusable" in code


def test_lowering_context_drops_completed_function_body_after_push() -> None:
    module = _read("src/compiler/mir/lower/ctx_module.ark")
    push_pos = module.index("fn ctx_push_current_function")
    push_body = module[push_pos:]
    assert "let finished = ctx_current_storage::ctx_current_func_value(ctx)" in push_body
    assert "MirModule_function_header_at(m, mir_index)" in push_body


def test_lowering_releases_frontend_inputs_after_body_emission() -> None:
    entry = _read("src/compiler/mir/lower/entry.ark")
    entry_input = _read("src/compiler/mir/lower/entry_input.ark")

    emit_pos = entry.index("entry_decls::mir_emit_view_decls(")
    release_pos = entry.index("mir_entry_input_release_frontend_scratch(input)")
    assert emit_pos < release_pos
    assert "pub fn mir_entry_input_release_frontend_scratch" in entry_input
    assert "input.decl_nodes = Vec::new<AstNode>()" in entry_input
    assert "input.wit_paths = Vec::new<String>()" in entry_input
    assert "input.source_text = String_new()" in entry_input


def test_lowering_moves_declarations_out_of_checked_program_bundle() -> None:
    checked_program = _read("src/compiler/driver/checked_program.ark")
    lower = _read("src/compiler/driver/lower.ark")

    assert "fn checked_program_take_all_decls" in checked_program
    take_pos = checked_program.index("fn checked_program_take_all_decls")
    take_body = checked_program[take_pos:]
    assert "let decls = program.bundle.decls" in take_body
    assert "program.bundle.decls = Vec::new<AstNode>()" in take_body
    assert lower.count("checked_program::checked_program_take_all_decls(program)") == 3


def test_lowering_drops_corehir_columns_after_body_emission() -> None:
    raw_record = _read("src/compiler/corehir/raw_record.ark")
    entry = _read("src/compiler/mir/lower/entry.ark")
    lower_input = _read("src/compiler/mir/lower/input.ark")

    assert "fn corehir_raw_program_release_after_body_emit" in raw_record
    assert "raw_program: CoreHirRawProgram" in lower_input
    assert "raw_record::corehir_raw_program_release_after_body_emit(input.raw_program)" in entry




def test_lowering_size_scan_only_runs_when_timing_is_requested() -> None:
    entry = _read("src/compiler/mir/lower/entry.ark")
    before = entry.index("let mut blocks_before")
    after = entry.index("let mut blocks_after")
    assert "if timing != 0" in entry[before:after]
    assert "if timing != 0" in entry[after:]


def test_body_store_host_feature_checks_use_serialized_calls() -> None:
    module_functions = _read("src/compiler/mir/module_functions.ark")

    needs_http = module_functions.index("fn mir_module_needs_wasi_http_outgoing")
    needs_runtime = module_functions.index("fn mir_module_needs_runtime_host")
    assert "MirModule_function_body_at" not in module_functions[needs_http:]
    assert "MirModule_function_body_at" not in module_functions[needs_runtime:]
    assert "MirModule_body_store_has_wasi_http_outgoing" in module_functions[needs_http:]
    assert "MirModule_body_store_has_runtime_host_call" in module_functions[needs_runtime:]


def test_non_compact_passes_read_full_functions_from_the_module() -> None:
    paths = (
        "src/compiler/mir_opt/async_lower/mod.ark",
        "src/compiler/mir_opt/stdlib_resolve_validated.ark",
        "src/compiler/mir/verify_call.ark",
        "src/compiler/wasm/code.ark",
    )
    for path in paths:
        source = _read(path)
        assert "module_functions::MirModule_function_at" in source


def test_emitter_context_resolves_function_bodies_through_the_module() -> None:
    context = _read("src/compiler/wasm/ctx_record.ark")
    constructor = _read("src/compiler/wasm/wasm_context.ark")
    sections = _read("src/compiler/wasm/wasm_sections.ark")

    assert "module: MirModule" in context
    assert "module_functions::MirModule_function_at(ctx.module, idx)" in context
    assert "module_functions::MirModule_function_header_at(ctx.module, idx)" in context
    assert "current_function: MirFunction" in context
    assert "SelfEmitCtx_set_current_function(ctx, f)" in _read("src/compiler/wasm/code.ark")
    assert "module: MirModule" in constructor
    assert "emit_wasm_context_new(\n        mir," in sections
    assert "MirModule_function_body_for_emit_at_reusable" in _read("src/compiler/wasm/code.ark")


def test_emitter_body_decode_skips_cfg_analysis_vectors() -> None:
    body_store = _read("src/compiler/mir/body_store.ark")

    assert "MirModule_function_body_for_emit_at_reusable" in body_store
    assert "mir_body_prepare_body_blocks(function, table, block_count)" in body_store
    assert "mir_body_skip_i32_vec(reader)" in body_store
    assert "mir_body_skip_phis(reader)" in body_store


def test_callee_inference_uses_headers_without_decoding_bodies() -> None:
    paths = (
        "src/compiler/wasm/code_ref_locals_infer_callee.ark",
        "src/compiler/wasm/code_ref_locals_infer_dest.ark",
        "src/compiler/wasm/intrinsic_vec_access_gc.ark",
        "src/compiler/wasm/call_fallback_resolved.ark",
        "src/compiler/wasm/sections_name.ark",
        "src/compiler/wasm/sections_debug_source_map.ark",
    )
    for path in paths:
        source = _read(path)
        assert "SelfEmitCtx_function_header_at" in source


def test_call_resolution_uses_serialized_forwarder_metadata() -> None:
    call_resolve = _read("src/compiler/wasm/call_resolve.ark")
    inst_ctx = _read("src/compiler/wasm/inst_ctx.ark")
    exports = _read("src/compiler/wasm/sections_exports.ark")

    assert "use mir::module_functions" in call_resolve
    assert "body_store::MirModule_body_store_sole_call_target(module, idx)" in call_resolve
    assert "module_functions::MirModule_function_header_at(module, idx)" in call_resolve
    assert "ctx.module,\n            functions," in inst_ctx
    assert "mir_collision_export_name(mir, functions" in exports


def test_body_store_keeps_counts_and_variant_slots_without_body_decode() -> None:
    body_store = _read("src/compiler/mir/body_store.ark")
    module_functions = _read("src/compiler/mir/module_functions.ark")
    callee_cache = _read("src/compiler/mir/post_pass_callee_cache.ark")

    assert "pub fn MirModule_body_store_block_count" in body_store
    assert "pub fn MirModule_body_store_inst_count" in body_store
    assert "body_store_return_variant_slots" in body_store
    assert "MirModule_body_store_return_variant_slot" in callee_cache
    count_pos = module_functions.index("fn MirModule_block_count")
    count_body = module_functions[count_pos:]
    assert "MirModule_body_store_block_count" in count_body
    assert "MirModule_body_store_inst_count" in count_body


def test_type_propagation_reuses_one_decoded_function() -> None:
    source = _read("src/compiler/mir/post_pass_type_propagate.ark")

    assert "let reusable = function_factory::MirFunction_new(" in source
    assert "body_store::MirModule_function_body_at_reusable(m, fi, reusable)" in source


def test_typed_sync_uses_headers_when_body_store_is_enabled() -> None:
    source = _read("src/compiler/mir/typed_mir_sync_module.ark")

    assert "module_functions::MirModule_function_header_at(m, fi)" in source
    assert "body_store::MirModule_body_store_enabled(m)" in source


def test_lowering_compacts_ast_fallback_for_shared_core_bodies() -> None:
    session = _read("src/compiler/compiler/session_corehir.ark")
    input_source = _read("src/compiler/mir/lower/input.ark")

    assert "session_lower_mir_fallback(all_decls, body_source, true)" in session
    assert "let body_source_decls = if lazy_body" in input_source
    assert "body_source_decls" in input_source
    assert "source_text: String,\n    lazy_body: bool" in input_source


def test_forwarder_resolution_can_scan_serialized_body_metadata() -> None:
    body_store = _read("src/compiler/mir/body_store.ark")
    call_resolve = _read("src/compiler/wasm/call_resolve.ark")

    assert "pub fn MirModule_body_store_sole_call_target" in body_store
    assert "mir_body_is_forwarder_computation" in body_store
    assert "body_store::MirModule_body_store_sole_call_target(module, idx)" in call_resolve
