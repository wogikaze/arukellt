#!/usr/bin/env python3
from pathlib import Path
import os
import shutil
import subprocess

root = Path(__file__).resolve().parents[2]
if shutil.which("wasmtime") is None:
    subprocess.run(
        ["bash", "-lc", "curl https://wasmtime.dev/install.sh -sSf | bash -s -- --version v46.0.1"],
        cwd=root,
        check=True,
    )
env = dict(os.environ)
env["PATH"] = str(Path.home() / ".wasmtime/bin") + os.pathsep + env.get("PATH", "")
env["ARUKELLT_SELFHOST_WASM"] = str(root / "bootstrap/arukellt-selfhost.wasm")
wrapper = str(root / "scripts/run/arukellt-selfhost.sh")
subprocess.run([wrapper, "fmt", "--fix", "std/host/fs.ark"], cwd=root, env=env, check=True, timeout=420)
subprocess.run([wrapper, "fmt", "--check", "std/host/fs.ark"], cwd=root, env=env, check=True, timeout=420)
