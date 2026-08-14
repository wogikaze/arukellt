#!/usr/bin/env python3
from pathlib import Path
import subprocess
import os

root = Path(__file__).resolve().parents[2]
env = dict(os.environ)
env["ARUKELLT_SELFHOST_WASM"] = str(root / "bootstrap/arukellt-selfhost.wasm")
subprocess.run(
    [str(root / "scripts/run/arukellt-selfhost.sh"), "fmt", "--fix", "std/host/fs.ark"],
    cwd=root,
    env=env,
    check=True,
)
