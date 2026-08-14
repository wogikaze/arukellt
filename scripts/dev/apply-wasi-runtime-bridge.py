#!/usr/bin/env python3
"""Temporary mechanical wiring for the WASI P2 runtime bridge.

Removed before the PR is marked ready. The resulting source changes are the
product; this script only makes multi-file bootstrap edits reproducible.
"""
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WASM = ROOT / "src/compiler/wasm"
RUNTIME_MODULE = "arukellt:runtime/host@0.1.0"


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if old not in text:
        if new in text:
            return
        raise RuntimeError(f"missing replacement anchor in {path}: {old[:80]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def replace_function(path: Path, start: str, next_start: str, replacement: str) -> None:
    text = path.read_text(encoding="utf-8")
    begin = text.find(start)
    if begin < 0:
        if replacement.strip() in text:
            return
        raise RuntimeError(f"missing function {start!r} in {path}")
    end = text.find(next_start, begin)
    if end < 0:
        raise RuntimeError(f"missing function boundary {next_start!r} in {path}")
    path.write_text(text[:begin] + replacement.rstrip() + "\n\n" + text[end:], encoding="utf-8")


def patch_constants() -> None:
    replace_once(WASM / "constants.ark", "fn NUM_RUNTIME_HOST_IMPORTS() -> i32 { 8 }", "fn NUM_RUNTIME_HOST_IMPORTS() -> i32 { 14 }")


def patch_runtime_flag() -> None:
    path = WASM / "wasm_sections.ark"
    old = "    let needs_runtime_host = module_functions::mir_module_needs_runtime_host(mir)"
    new = "    let needs_runtime_host = if emit_target::is_p2_wasi(clone(wasi_version)) { 1 } else { module_functions::mir_module_needs_runtime_host(mir) }"
    replace_once(path, old, new)


def patch_type_sigs() -> None:
    path = WASM / "sections_types_sigs.ark"
    text = path.read_text(encoding="utf-8")
    start = text.find("    if needs_runtime_host == 1 {")
    if start < 0:
        raise RuntimeError("runtime host type block missing")
    end = text.find("\n    }", start)
    if end < 0:
        raise RuntimeError("runtime host type block end missing")
    end += len("\n    }")
    block = '''    if needs_runtime_host == 1 {
        // Internal guest-core runtime ABI. The P2 component bridge translates
        // these pointer/scalar shapes to canonical component functions.
        push(type_sigs, "i32_i32_i32_1i32")
        push(type_sigs, "i32_i32_i32_i32_i32_i32_i32_1i32")
        push(type_sigs, "i32_i32_i32_i32_1i32")
    }'''
    path.write_text(text[:start] + block + text[end:], encoding="utf-8")


def patch_imports() -> None:
    path = WASM / "sections_imports.ark"
    text = path.read_text(encoding="utf-8")
    if "fn runtime_host_module_name()" not in text:
        anchor = '''fn arukellt_io_module_name() -> String {
    String_from("arukellt_io")
}
'''
        insert = anchor + '''
fn runtime_host_module_name() -> String {
    String_from("arukellt:runtime/host@0.1.0")
}
'''
        if anchor not in text:
            raise RuntimeError("module-name anchor missing")
        text = text.replace(anchor, insert, 1)

    old_base = '''    emit_host_func_import(import_sec, String_from("wasi:filesystem/types@0.2.0"), String_from("open-at"), get_unchecked(host_type_map, 2))
    // Bridged P1 fd_read/fd_write ABI for bootstrap host-linker (#834).
    // Keep off wasi:filesystem/types — stock WASI validate rejects bare
    // `read`/`write` there and breaks component close-gates (#074/#510).
    emit_host_func_import(import_sec, String_from("arukellt:fs@0.1.0"), String_from("read"), get_unchecked(host_type_map, 0))
    emit_host_func_import(import_sec, String_from("arukellt:fs@0.1.0"), String_from("write"), get_unchecked(host_type_map, 0))
    emit_host_func_import(import_sec, String_from("wasi:cli/stdin@0.2.0"), String_from("read"), get_unchecked(host_type_map, 0))
    emit_host_func_import(import_sec, String_from("wasi:filesystem/types@0.2.0"), String_from("close"), get_unchecked(host_type_map, 3))'''
    new_base = '''    // Filesystem operations are an internal, versioned core ABI. The P2
    // component bridge owns the pointer/fd compatibility shape and lowers to
    // arukellt:runtime@0.1.0 canonical functions backed by real WASI.
    let runtime_host = runtime_host_module_name()
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_fs_open_at"), get_unchecked(host_type_map, 2))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_fs_read"), get_unchecked(host_type_map, 0))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_fs_write"), get_unchecked(host_type_map, 0))
    emit_host_func_import(import_sec, String_from("wasi:cli/stdin@0.2.0"), String_from("read"), get_unchecked(host_type_map, 0))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_fs_close"), get_unchecked(host_type_map, 3))'''
    if old_base in text:
        text = text.replace(old_base, new_base, 1)
    elif new_base not in text:
        raise RuntimeError("P2 filesystem import block missing")

    old_call = "        emit_network_wit_import_entries(import_sec, host_type_map, clone(wasi_version), needs_wasi_http_outgoing)"
    new_call = "        emit_p2_runtime_host_import_entries(import_sec, host_type_map, needs_wasi_http_outgoing)"
    p2_pos = text.find("fn emit_p2_import_entries(")
    call_pos = text.find(old_call, p2_pos)
    if call_pos >= 0:
        text = text[:call_pos] + new_call + text[call_pos + len(old_call):]

    if "fn emit_p2_runtime_host_import_entries(" not in text:
        anchor = "fn emit_t2_import_entries(import_sec: WasmWriter, host_type_map: Vec<i32>) {"
        pos = text.find(anchor)
        if pos < 0:
            raise RuntimeError("T2 import anchor missing")
        helper = '''fn emit_p2_runtime_host_import_entries(
    import_sec: WasmWriter,
    host_type_map: Vec<i32>,
    needs_wasi_http_outgoing: i32
) {
    let host_type_off = network_host_type_offset(String_from("wasi-p2"), needs_wasi_http_outgoing)
    let runtime_host = runtime_host_module_name()
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_http_get"), get_unchecked(host_type_map, constants::WASI_HOST_TYPE_HTTP_GET() + host_type_off))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_http_request"), get_unchecked(host_type_map, constants::WASI_HOST_TYPE_HTTP_REQUEST() + host_type_off))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_sockets_connect"), get_unchecked(host_type_map, constants::WASI_HOST_TYPE_SOCKETS_CONNECT() + host_type_off))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_sockets_read"), get_unchecked(host_type_map, constants::WASI_HOST_TYPE_HTTP_GET() + host_type_off))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_sockets_write"), get_unchecked(host_type_map, constants::WASI_HOST_TYPE_SOCKETS_CONNECT() + host_type_off))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_sockets_listen"), get_unchecked(host_type_map, constants::WASI_HOST_TYPE_SOCKETS_CONNECT() + host_type_off))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_sockets_accept"), get_unchecked(host_type_map, 1))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_http_serve"), get_unchecked(host_type_map, constants::WASI_HOST_TYPE_SOCKETS_CONNECT() + host_type_off))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_fs_read_dir"), get_unchecked(host_type_map, constants::WASI_HOST_TYPE_HTTP_GET() + host_type_off))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_fs_metadata"), get_unchecked(host_type_map, constants::WASI_HOST_TYPE_HTTP_GET() + host_type_off))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_fs_remove_file"), get_unchecked(host_type_map, constants::WASI_HOST_TYPE_HTTP_GET() + host_type_off))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_fs_create_dir_all"), get_unchecked(host_type_map, constants::WASI_HOST_TYPE_HTTP_GET() + host_type_off))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_env_vars"), get_unchecked(host_type_map, 3))
    emit_host_func_import(import_sec, clone(runtime_host), String_from("runtime_env_current_dir"), get_unchecked(host_type_map, 3))
}

'''
        text = text[:pos] + helper + text[pos:]
    path.write_text(text, encoding="utf-8")


def patch_import_indices() -> None:
    path = WASM / "import_indices.ark"
    text = path.read_text(encoding="utf-8")
    if "fn fs_read_dir_import_idx" in text:
        return
    text += '''
fn fs_read_dir_import_idx(ctx: SelfEmitCtx) -> i32 { runtime_host_import_base(ctx) + 8 }
fn fs_metadata_import_idx(ctx: SelfEmitCtx) -> i32 { runtime_host_import_base(ctx) + 9 }
fn fs_remove_file_import_idx(ctx: SelfEmitCtx) -> i32 { runtime_host_import_base(ctx) + 10 }
fn fs_create_dir_all_import_idx(ctx: SelfEmitCtx) -> i32 { runtime_host_import_base(ctx) + 11 }
fn env_vars_import_idx(ctx: SelfEmitCtx) -> i32 { runtime_host_import_base(ctx) + 12 }
fn env_current_dir_import_idx(ctx: SelfEmitCtx) -> i32 { runtime_host_import_base(ctx) + 13 }
'''
    path.write_text(text, encoding="utf-8")


def main() -> None:
    patch_constants()
    patch_runtime_flag()
    patch_type_sigs()
    patch_imports()
    patch_import_indices()


if __name__ == "__main__":
    main()
