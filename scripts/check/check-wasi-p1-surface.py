#!/usr/bin/env python3
from __future__ import annotations
import re, sys
from pathlib import Path
REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "scripts" / "check"))
from _gate_open_skip import skip_if_any_open

EXPECTED = ["args_get","args_sizes_get","clock_res_get","clock_time_get","environ_get","environ_sizes_get","fd_close","fd_fdstat_get","fd_read","fd_seek","fd_tell","fd_write","path_open","proc_exit","random_get"]
STD_REQUIRED = {"clock_time_get","clock_res_get","random_get","args_sizes_get","args_get","environ_sizes_get","environ_get","proc_exit","fd_write","fd_read","path_open","fd_close","fd_seek","fd_tell","fd_fdstat_get"}

def main() -> int:
    if (s := skip_if_any_open(["073"], "check-wasi-p1-surface")) is not None: return s
    reg = REPO_ROOT / "src/compiler/wasm/wasi_p1_registry.ark"
    if not reg.is_file(): print("FAIL: registry", file=sys.stderr); return 1
    names = set(re.findall(r'String_from\("([a-z_]+)"\)', reg.read_text(encoding="utf-8")))
    missing = [x for x in EXPECTED if x not in names]
    if missing: print(f"FAIL: missing {missing}", file=sys.stderr); return 1
    imp = (REPO_ROOT / "src/compiler/wasm/sections_imports.ark").read_text(encoding="utf-8")
    for s in sorted(STD_REQUIRED):
        if f'"{s}"' not in imp: print(f"FAIL: import {s}", file=sys.stderr); return 1
    man = (REPO_ROOT / "tests/fixtures/manifest.txt").read_text(encoding="utf-8")
    for f in ("wasi_clock.ark","wasi_random.ark","wasi_args.ark"):
        if f not in man: print(f"FAIL: {f}", file=sys.stderr); return 1
    print("check-wasi-p1-surface: ok"); return 0
if __name__ == "__main__": raise SystemExit(main())
