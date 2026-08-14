#!/usr/bin/env python3
"""One-shot branch-built WASI runtime E2E receipt runner."""
from pathlib import Path
import os
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]

def run(cmd, *, env=None):
    subprocess.run(cmd, cwd=ROOT, env=env, check=True)

# Match full-validation's real compiler lane, then add the WAC linker needed by
# the production component launcher.
run(["bash", "-lc", "curl https://wasmtime.dev/install.sh -sSf | bash -s -- --version v46.0.1"])
run(["bash", "-lc", "curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash"])
path = os.environ.get("PATH", "")
env = dict(os.environ)
env["PATH"] = f"{Path.home() / '.wasmtime/bin'}:{Path.home() / '.cargo/bin'}:{path}"
run(["cargo", "binstall", "--no-confirm", "wac-cli"], env=env)
run([sys.executable, "scripts/manager.py", "selfhost", "fixpoint", "--build"], env=env)
env["ARUKELLT_REQUIRE_RUNTIME_E2E"] = "1"
run([sys.executable, "scripts/check/gate-076-wasi-p2-filesystem.py"], env=env)
run([sys.executable, "scripts/check/gate-841-real-wasi-network-abi.py"], env=env)
