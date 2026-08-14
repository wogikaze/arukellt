#!/usr/bin/env python3
"""Temporary CoreOp registry expansion for #676; removed before PR readiness."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PATH = ROOT / "data/core-ops.toml"
text = PATH.read_text(encoding="utf-8")

ALIASES = {
    "__runtime_abi_fs_read_dir": "runtime.fs_read_dir",
    "__runtime_abi_fs_metadata": "runtime.fs_metadata",
    "__runtime_abi_fs_remove_file": "runtime.fs_remove_file",
    "__runtime_abi_fs_create_dir_all": "runtime.fs_create_dir_all",
    "__runtime_abi_env_vars": "runtime.env_vars",
    "__runtime_abi_env_current_dir": "runtime.env_current_dir",
}

for alias, op_id in ALIASES.items():
    marker = f'alias = "{alias}"'
    if marker not in text:
        text += f'\n[[legacy_bindings]]\nalias = "{alias}"\ncore_op_id = "{op_id}"\n'

OPS = [
    ("runtime.fs_read_dir", "fs_read_dir", 1),
    ("runtime.fs_metadata", "fs_metadata", 1),
    ("runtime.fs_remove_file", "fs_remove_file", 1),
    ("runtime.fs_create_dir_all", "fs_create_dir_all", 1),
    ("runtime.env_vars", "env_vars", 0),
    ("runtime.env_current_dir", "env_current_dir", 0),
]

for op_id, symbol, arity in OPS:
    marker = f'id = "{op_id}"'
    if marker in text:
        continue
    inputs = "[]" if arity == 0 else '[{ name = "arg0", type = { kind = "primitive", name = "i32" } }]'
    text += f'''\n[[operations]]
id = "{op_id}"
visibility = "public"
classification = {{ layer = "runtime" }}
binding = {{ policy = "optional", reason = "WASI runtime ABI production", tracking_issue = "676" }}
description = "Versioned runtime ABI operation {symbol}"
[operations.signature]
inputs = {inputs}
outputs = [{{ type = {{ kind = "primitive", name = "i32" }} }}]
generic_params = []
constraints = []
[operations.semantics]
const_evaluable = false
overflow = "none"
nan = "none"
trap = "none"
equivalence = "exact_bitwise"
[operations.effect]
memory = "read_write"
allocates = true
may_trap = false
noreturn = false
external_io = true
nondeterminism = "runtime"
atomic = false
volatile = false
[operations.inline]
policy = "never"
[operations.lowering]
kind = "runtime_call"
[operations.lowering.runtime]
kind = "internal"
symbol = "{symbol}"
abi_version = "0.1"
[operations.fallback]
required = false
'''

PATH.write_text(text, encoding="utf-8")
