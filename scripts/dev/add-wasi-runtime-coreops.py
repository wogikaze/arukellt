#!/usr/bin/env python3
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
path = root / "data/native-cpp-capabilities.toml"
text = path.read_text(encoding="utf-8")

entries = [
    "runtime.fs_read_dir",
    "runtime.fs_metadata",
    "runtime.fs_remove_file",
    "runtime.fs_create_dir_all",
    "runtime.env_vars",
    "runtime.env_current_dir",
]
for op_id in entries:
    if f'id = "{op_id}"' in text:
        continue
    text += f'''\n[[core_ops]]
id = "{op_id}"
registry_layer = "runtime"
registry_lowering = "runtime_call"
status = "unsupported"
reason = "WASI P2 runtime adapter operation; native-cpp support is outside this productionization slice"
'''
path.write_text(text, encoding="utf-8")

subprocess.run(
    [sys.executable, str(root / "scripts/check/check-native-cpp-capabilities.py"), "--write-generated"],
    cwd=root,
    check=True,
)
