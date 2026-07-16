#!/usr/bin/env python3
"""Compile selfhost source with s3 to get s4, check s3==s4 (true fixpoint)."""
import sys, os, subprocess, hashlib, shutil
sys.stderr = open(os.devnull, 'w')

from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
from selfhost.checks import (
    _prepare_flattened_selfhost_source,
    _find_wasmtime,
    _ensure_bootstrap_compiler_wasm,
    _find_pinned_wasm,
)

root = Path('.')
wasmtime = _find_wasmtime()
overlay = _prepare_flattened_selfhost_source(root)
os.stderr = sys.__stderr__

print(f"overlay: {overlay}")

# s3 is the s2-compiled selfhost
s3_path = root / ".build" / "selfhost" / "s3_det_a.wasm"
print(f"s3: {s3_path} ({s3_path.stat().st_size} bytes)")

# Patch s3 for runtime (heap grow + memory64)
pinned = _find_pinned_wasm(root)
patcher = root / "scripts" / "bootstrap" / "wasm-heap-grow-patcher" / "target" / "release" / "wasm-heap-grow-patcher"
s3_runtime = overlay / "s3rt.wasm"
r = subprocess.run([str(patcher), str(s3_path), str(s3_runtime), "--to-memory64"], capture_output=True, text=True)
print(f"patched s3: {r.stderr.strip()}")

# Compile selfhost source with s3-runtime → s4
out_name = "s4.wasm"
out_path = overlay / out_name
if out_path.exists():
    out_path.unlink()
cmd = [wasmtime, "run", "--wasm", "gc", "--wasm", "function-references", "-W", "memory64=y",
       "--dir", f"{overlay}::.", str(s3_runtime), "--",
       "compile", "src/compiler/main.ark", "--target", "wasm32-gc", "--wasi-version", "wasi-p2",
       "-o", out_name]
r = subprocess.run(cmd, cwd=str(overlay), capture_output=True, text=True, timeout=180)
if out_path.exists() and out_path.stat().st_size > 0:
    s4_data = out_path.read_bytes()
    s3_data = s3_path.read_bytes()
    s3_sha = hashlib.sha256(s3_data).hexdigest()
    s4_sha = hashlib.sha256(s4_data).hexdigest()
    print(f"s3: {len(s3_data)} bytes sha={s3_sha[:16]}")
    print(f"s4: {len(s4_data)} bytes sha={s4_sha[:16]}")
    if s3_sha == s4_sha:
        print("FIXPOINT REACHED: s3 == s4!")
    else:
        print(f"NOT fixpoint: diff={len(s4_data)-len(s3_data)} bytes")
        # Save s4
        shutil.copy2(out_path, root / ".build" / "selfhost" / "s4.wasm")
        print("saved s4 to .build/selfhost/s4.wasm")
else:
    print(f"s4 FAILED rc={r.returncode}")
    print(f"  stderr: {r.stderr[:500]}")
