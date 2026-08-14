#!/usr/bin/env python3
"""Mechanical ownership migration for #819/#841.

This script is intentionally idempotent and is removed before the PR is marked
ready. It renames host-operation emitter modules to the runtime ABI namespace,
renames the runtime-routing flag, moves internal host-operation spellings from
__intrinsic_* to __runtime_*, and applies temporary branch-local source edits
that are easier to express mechanically while the ABI is being migrated.
"""
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WASM = ROOT / "src" / "compiler" / "wasm"
FAMILIES = ("http", "sockets", "fs", "env", "process", "stdio", "time", "random")
RUNTIME_ADAPTER_METHODS = (
    "buffer_new", "buffer_push", "buffer_len", "buffer_byte", "buffer_close",
    "http_get", "http_request", "http_serve",
    "socket_connect", "socket_read", "socket_write", "socket_listen", "socket_accept", "socket_close",
    "fs_read_file", "fs_write_file", "fs_write_bytes", "fs_read_dir", "fs_metadata", "fs_remove_file", "fs_create_dir_all",
    "env_vars", "env_current_dir", "env_var",
)


def rename_files() -> None:
    for path in sorted(WASM.glob("call_host*.ark")):
        target = path.with_name(path.name.replace("call_host", "call_runtime", 1))
        if target.exists():
            path.unlink()
        else:
            path.rename(target)
    for family in FAMILIES:
        for path in sorted(WASM.glob(f"intrinsic_{family}*.ark")):
            target = path.with_name(path.name.replace("intrinsic_", "runtime_abi_", 1))
            if target.exists():
                path.unlink()
            else:
                path.rename(target)


def editable_files() -> list[Path]:
    roots = [
        ROOT / "src",
        ROOT / "std",
        ROOT / "data",
        ROOT / "tests",
        ROOT / "scripts",
        ROOT / "docs" / "plans",
    ]
    suffixes = {".ark", ".py", ".sh", ".toml", ".md", ".txt", ".json", ".yml", ".yaml"}
    out: list[Path] = []
    for base in roots:
        if not base.exists():
            continue
        for path in base.rglob("*"):
            if path.is_file() and path.suffix in suffixes and path != Path(__file__).resolve():
                out.append(path)
    return out


def rewrite_text() -> None:
    replacements: list[tuple[str, str]] = [
        ("needs_arukellt_host", "needs_runtime_host"),
        ("needs_network_runtime", "needs_runtime_host"),
        ("mir_call_is_arukellt_host", "mir_call_is_runtime_host"),
        ("NUM_ARUKELLT_HOST_IMPORTS", "NUM_RUNTIME_HOST_IMPORTS"),
        ("arukellt_host_import_base", "runtime_host_import_base"),
        ("wasm::call_host", "wasm::call_runtime"),
        ("call_host::", "call_runtime::"),
        ("call_host_", "call_runtime_"),
    ]
    for family in FAMILIES:
        replacements.extend(
            [
                (f"wasm::intrinsic_{family}", f"wasm::runtime_abi_{family}"),
                (f"intrinsic_{family}::", f"runtime_abi_{family}::"),
                (f"intrinsic_{family}_", f"runtime_abi_{family}_"),
                (f"__intrinsic_{family}", f"__runtime_{family}"),
            ]
        )
    for path in editable_files():
        try:
            old = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        new = old
        for before, after in replacements:
            new = new.replace(before, after)
        if new != old:
            path.write_text(new, encoding="utf-8")


def _replace_function(text: str, start: str, next_start: str, replacement: str) -> str:
    begin = text.find(start)
    if begin < 0:
        return text
    end = text.find(next_start, begin)
    if end < 0:
        raise RuntimeError(f"unable to find function boundary after {start!r}")
    return text[:begin] + replacement.rstrip() + "\n\n" + text[end:]


def rewrite_runtime_adapter() -> None:
    path = ROOT / "runtime" / "wasi-p2-adapter" / "src" / "lib.rs"
    text = path.read_text(encoding="utf-8")
    text = text.replace(
        "impl exports::arukellt::runtime::host::Guest for RuntimeAdapter {",
        "impl Guest for RuntimeAdapter {",
    )
    for name in RUNTIME_ADAPTER_METHODS:
        text = text.replace(f"    fn {name}(", f"    fn runtime_{name}(")

    marker = "impl Guest for RuntimeAdapter {\n"
    if "fn runtime_buffer_new(" not in text:
        methods = '''impl Guest for RuntimeAdapter {
    fn runtime_buffer_new() -> u32 {
        state()
            .lock()
            .expect("runtime state poisoned")
            .insert(HandleValue::Buffer(Vec::new()))
    }

    fn runtime_buffer_push(handle: u32, byte: u8) {
        let mut guard = state().lock().expect("runtime state poisoned");
        if let Some(HandleValue::Buffer(bytes)) = guard.get_mut(handle) {
            bytes.push(byte);
        }
    }
'''
        text = text.replace(marker, methods, 1)

    text = _replace_function(
        text,
        "    fn runtime_socket_write(",
        "    fn runtime_socket_listen(",
        '''    fn runtime_socket_write(socket: u32, buffer: u32) -> i32 {
        let mut guard = state().lock().expect("runtime state poisoned");
        let bytes = match guard.get(buffer) {
            Some(HandleValue::Buffer(bytes)) => bytes.clone(),
            _ => {
                drop(guard);
                return store_error("invalid socket buffer handle");
            }
        };
        let Some(HandleValue::Stream(stream)) = guard.get_mut(socket) else {
            drop(guard);
            return store_error("invalid socket handle");
        };
        match stream.write(&bytes) {
            Ok(written) => written as i32,
            Err(error) => {
                drop(guard);
                store_error(format!("socket write failed: {error}"))
            }
        }
    }''',
    )
    text = _replace_function(
        text,
        "    fn runtime_fs_write_bytes(",
        "    fn runtime_fs_read_dir(",
        '''    fn runtime_fs_write_bytes(path: String, buffer: u32) -> i32 {
        let bytes = {
            let guard = state().lock().expect("runtime state poisoned");
            match guard.get(buffer) {
                Some(HandleValue::Buffer(bytes)) => bytes.clone(),
                _ => return store_error("invalid filesystem buffer handle"),
            }
        };
        store_io(fs::write(path, bytes), |_| 0)
    }''',
    )
    path.write_text(text, encoding="utf-8")


def main() -> None:
    rename_files()
    rewrite_text()
    rewrite_runtime_adapter()


if __name__ == "__main__":
    main()
