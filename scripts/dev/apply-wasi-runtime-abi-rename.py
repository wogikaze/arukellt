#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[2]
path = root / "src/compiler/wasm/sections_imports.ark"
text = path.read_text(encoding="utf-8")
replacements = {
    'emit_host_func_import(import_sec, String_from("wasi:filesystem/types@0.2.0"), String_from("open-at"),':
        'emit_host_func_import(import_sec, runtime_host_module_name(), String_from("runtime_fs_open_at"),',
    'emit_host_func_import(import_sec, String_from("arukellt:fs@0.1.0"), String_from("read"),':
        'emit_host_func_import(import_sec, runtime_host_module_name(), String_from("runtime_fs_read"),',
    'emit_host_func_import(import_sec, String_from("arukellt:fs@0.1.0"), String_from("write"),':
        'emit_host_func_import(import_sec, runtime_host_module_name(), String_from("runtime_fs_write"),',
    'emit_host_func_import(import_sec, String_from("wasi:filesystem/types@0.2.0"), String_from("close"),':
        'emit_host_func_import(import_sec, runtime_host_module_name(), String_from("runtime_fs_close"),',
}
for old, new in replacements.items():
    text = text.replace(old, new)
path.write_text(text, encoding="utf-8")
