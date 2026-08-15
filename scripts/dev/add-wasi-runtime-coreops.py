#!/usr/bin/env python3
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
for rel in (
    "scripts/dev/apply-wasi-runtime-abi-rename.py",
    "scripts/dev/apply-wasi-runtime-bridge.py",
    "scripts/dev/apply-wasi-runtime-launcher.py",
    "scripts/dev/add-wasi-runtime-coreops.py",
):
    (ROOT / rel).unlink(missing_ok=True)
subprocess.run([sys.executable, "scripts/gen/generate-docs.py"], cwd=ROOT, check=True)
