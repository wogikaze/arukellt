#!/usr/bin/env python3
from pathlib import Path
import re
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
path = root / "data/core-ops.toml"
text = path.read_text(encoding="utf-8")

aliases = {
    "__runtime_abi_fs_read_dir": "runtime.fs_read_dir",
    "__runtime_abi_fs_metadata": "runtime.fs_metadata",
    "__runtime_abi_fs_remove_file": "runtime.fs_remove_file",
    "__runtime_abi_fs_create_dir_all": "runtime.fs_create_dir_all",
    "__runtime_abi_env_vars": "runtime.env_vars",
    "__runtime_abi_env_current_dir": "runtime.env_current_dir",
}
for alias, op_id in aliases.items():
    if f'alias = "{alias}"' not in text:
        text += f'\n[[legacy_bindings]]\nalias = "{alias}"\ncore_op_id = "{op_id}"\n'

ops = [
    ("runtime.fs_read_dir", "fs_read_dir", 1),
    ("runtime.fs_metadata", "fs_metadata", 1),
    ("runtime.fs_remove_file", "fs_remove_file", 1),
    ("runtime.fs_create_dir_all", "fs_create_dir_all", 1),
    ("runtime.env_vars", "env_vars", 0),
    ("runtime.env_current_dir", "env_current_dir", 0),
]
for op_id, symbol, arity in ops:
    # Match an operation's own `id` field, not `core_op_id` in legacy bindings.
    if re.search(rf'(?m)^id = "{re.escape(op_id)}"$', text):
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
memory = "none"
allocates = false
may_trap = false
noreturn = false
external_io = false
nondeterminism = "deterministic"
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
path.write_text(text, encoding="utf-8")

# Remove the stale pre-production bridge comment. P2 filesystem imports above
# are now the versioned runtime ABI and the adapter lowers them to real WASI.
imports_path = root / "src/compiler/wasm/sections_imports.ark"
imports = imports_path.read_text(encoding="utf-8")
imports = imports.replace(
    '    // fd_write is `arukellt:fs@0.1.0::write` above (#834). Do not also emit\n'
    '    // wasi:filesystem/types write — that desyncs NUM_P2_IMPORTS_BASE and fails\n'
    '    // stock WASI validate (#074/#510 / merge of #807+#834).\n',
    '    // Filesystem calls above are routed through the versioned runtime ABI;\n'
    '    // the component adapter owns the real WASI filesystem resource boundary.\n',
)
imports_path.write_text(imports, encoding="utf-8")

# Keep all generated products in lock-step with the source-of-truth registry.
for generator in (
    "scripts/gen/generate-core-ops-registry.py",
    "scripts/gen/generate-core-op-bindings.py",
    "scripts/gen/generate-docs.py",
):
    subprocess.run([sys.executable, str(root / generator)], cwd=root, check=True)
