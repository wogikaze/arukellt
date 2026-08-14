#!/usr/bin/env python3
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[2]

manifest = root / "std/manifest.toml"
text = manifest.read_text(encoding="utf-8")
entries = '''

# === WASI P2 host surface production additions (#676) ===

[[functions]]
name = "id"
module = "std::host::process"
stability = "stable"
params = []
returns = "Result<i32, String>"
doc_category = "host_process"
doc = "Return a stable Err on WASI 0.2 because the portable process model does not expose a POSIX-style process identifier."
availability = { t1 = true, t3 = true }

[[functions]]
name = "vars_snapshot"
module = "std::host::env"
stability = "stable"
params = []
returns = "Result<String, String>"
doc_category = "host_env"
intrinsic = "__runtime_abi_env_vars"
doc = "Return a stable NUL-delimited snapshot of KEY=VALUE environment records through the versioned runtime ABI."
availability = { t1 = false, t3 = true, note = "WASI Preview 2 runtime ABI." }

[[functions]]
name = "current_dir"
module = "std::host::env"
stability = "stable"
params = []
returns = "Result<String, String>"
doc_category = "host_env"
intrinsic = "__runtime_abi_env_current_dir"
doc = "Return the runtime current directory as UTF-8 through the versioned runtime ABI."
availability = { t1 = false, t3 = true, note = "WASI Preview 2 runtime ABI." }
'''
if 'name = "vars_snapshot"' not in text:
    text += entries
manifest.write_text(text, encoding="utf-8")

caps = root / "docs/data/capabilities.toml"
text = caps.read_text(encoding="utf-8")
text = text.replace('''id = "process"
module = "std::host::process"
path = "std/host/process.ark"
declared = true
registered = true
compiles = true
links = true
runs = true
user_reachable = true
grant_required = "no"
verified_on = ["wasm32", "wasm32-gc"]
notes = ""''', '''id = "process"
module = "std::host::process"
path = "std/host/process.ark"
declared = true
registered = true
compiles = true
links = true
runs = true
user_reachable = true
grant_required = "optional compile-time deny"
deny_flag = "--deny-process"
deny_enforcement = "compile_time_mir"
deny_intended_enforcement = "compile_time_mir"
deny_transitive = true
deny_applies_to = "compile, run, check"
verified_on = ["wasm32", "wasm32-gc"]
notes = "exit/abort are portable runtime operations; id() returns a stable Err on WASI 0.2."''')
text = text.replace('''id = "http"
module = "std::host::http"
path = "std/host/http.ark"
declared = true
registered = true
compiles = "partial"
links = "partial"
runs = false
user_reachable = false
grant_required = "n/a"
verified_on = []
notes = "host_http_user_reachable=false; WIT-bridged wasi:http imports (#727); real ABI #841"''', '''id = "http"
module = "std::host::http"
path = "std/host/http.ark"
declared = true
registered = true
compiles = true
links = true
runs = true
user_reachable = true
grant_required = "runtime network/HTTP grant"
verified_on = ["wasm32-gc"]
notes = "Real WASI 0.2 HTTP via the checked P2 runtime adapter (#841); no Arukellt host shim."''')
text = text.replace('''id = "sockets"
module = "std::host::sockets"
path = "std/host/sockets.ark"
declared = true
registered = true
compiles = "partial"
links = "partial"
runs = false
user_reachable = false
grant_required = "n/a"
verified_on = []
notes = "E0500 on wasm32; not user-reachable; WIT-bridged wasi:sockets/tcp (#727); real ABI #841"''', '''id = "sockets"
module = "std::host::sockets"
path = "std/host/sockets.ark"
declared = true
registered = true
compiles = true
links = true
runs = true
user_reachable = true
grant_required = "runtime network grant"
verified_on = ["wasm32-gc"]
notes = "Real WASI 0.2 TCP/streams via the checked P2 runtime adapter (#841); wasm32 remains target-gated."''')
caps.write_text(text, encoding="utf-8")

cli = root / "docs/cli-reference.md"
text = cli.read_text(encoding="utf-8")
section = '''

## Host capability defaults

WASI P2 host capabilities are explicit runtime authorities. Filesystem access is deny-by-default and requires a preopened directory (for example Wasmtime `--dir`). HTTP and sockets require the runtime network/HTTP grants used by the embedding runtime. Environment access sees only values supplied by the runtime. Process `exit`/`abort` are available by default on supported host targets; `--deny-process` rejects programs that use those process-control intrinsics during compilation/checking. `--deny-clock` and `--deny-random` provide the analogous compile-time deny controls for clock and host randomness.

The compiler does not manufacture a process ID on WASI 0.2: `std::host::process::id()` returns `Err`. Runtime directory traversal outside a granted preopen is rejected by the WASI filesystem boundary.
'''
if "## Host capability defaults" not in text:
    text += section
cli.write_text(text, encoding="utf-8")

for script in ("scripts/gen/generate-docs.py", "scripts/gen/generate-structured-state-docs.py"):
    subprocess.run([sys.executable, str(root / script)], cwd=root, check=True)
