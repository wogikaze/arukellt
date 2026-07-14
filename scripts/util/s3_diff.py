#!/usr/bin/env python3
"""Compile selfhost source twice with s2-runtime and diff the outputs."""
import sys, os, subprocess, hashlib, shutil
sys.stderr = open(os.devnull, 'w')

from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
from selfhost.checks import (
    _prepare_flattened_selfhost_source,
    _find_wasmtime,
)

root = Path('.')
wasmtime = _find_wasmtime()
overlay = _prepare_flattened_selfhost_source(root)
os.stderr = sys.__stderr__

print(f"overlay: {overlay}")
s2_runtime = root / ".build" / "selfhost" / "arukellt-s2-runtime.wasm"
print(f"s2-runtime: {s2_runtime} ({s2_runtime.stat().st_size} bytes)")

# Copy s2-runtime into overlay so it can write output there
shutil.copy2(s2_runtime, overlay / "s2rt.wasm")

results = []
for i in range(2):
    out_name = f"s3_{i}.wasm"
    out_path = overlay / out_name
    if out_path.exists():
        out_path.unlink()
    cmd = [wasmtime, "run", "--wasm", "gc", "--wasm", "function-references",
           "--dir", f"{overlay}::.", str(overlay / "s2rt.wasm"), "--",
           "compile", "src/compiler/main.ark", "--target", "wasm32", "--wasi-version", "wasi-p1",
           "-o", out_name]
    r = subprocess.run(cmd, cwd=str(overlay), capture_output=True, text=True, timeout=180)
    if out_path.exists() and out_path.stat().st_size > 0:
        data = out_path.read_bytes()
        sha = hashlib.sha256(data).hexdigest()
        results.append((out_name, len(data), sha, data))
        print(f"  {out_name}: {len(data)} bytes sha={sha[:16]}")
    else:
        print(f"  {out_name}: FAILED rc={r.returncode}")
        print(f"    stderr: {r.stderr[:300]}")

if len(results) == 2:
    a, b = results
    if a[2] == b[2]:
        print("IDENTICAL - deterministic!")
    else:
        print(f"DIFFERENT: {a[0]}={a[1]}B vs {b[0]}={b[1]}B (diff={b[1]-a[1]}B)")
        da, db = a[3], b[3]
        for i in range(min(len(da), len(db))):
            if da[i] != db[i]:
                print(f"  first diff at offset {i}: a=0x{da[i]:02x} b=0x{db[i]:02x}")
                ctx_a = da[max(0,i-16):i+16].hex()
                ctx_b = db[max(0,i-16):i+16].hex()
                print(f"  a context: ...{ctx_a}...")
                print(f"  b context: ...{ctx_b}...")
                break
        # Save both for analysis
        shutil.copy2(overlay / a[0], root / ".build" / "selfhost" / "s3_det_a.wasm")
        shutil.copy2(overlay / b[0], root / ".build" / "selfhost" / "s3_det_b.wasm")
        print("  saved to .build/selfhost/s3_det_{a,b}.wasm")
else:
    print("Could not produce both outputs for comparison")
