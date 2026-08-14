#!/usr/bin/env python3
from pathlib import Path
import os
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]


def run(args, *, env=None, timeout=1800):
    print("+", " ".join(map(str, args)), flush=True)
    subprocess.run(args, cwd=ROOT, env=env, check=True, timeout=timeout)


if shutil.which("wasmtime") is None:
    run(["bash", "-lc", "curl https://wasmtime.dev/install.sh -sSf | bash -s -- --version v46.0.1"], timeout=180)

wac = Path("/tmp/wac")
if not wac.is_file():
    run([
        "curl", "-fL",
        "https://github.com/bytecodealliance/wac/releases/download/v0.10.1/wac-cli-x86_64-unknown-linux-musl",
        "-o", str(wac),
    ], timeout=180)
    run(["bash", "-lc", "echo '250c11762916ba733c7d22b62487580f21270ec9dde4f13460ea69d300e25406  /tmp/wac' | sha256sum -c -"])
    wac.chmod(0o755)

env = dict(os.environ)
env["PATH"] = "/tmp:" + str(Path.home() / ".wasmtime/bin") + os.pathsep + env.get("PATH", "")

run([sys.executable, "scripts/manager.py", "selfhost", "build-compiler"], env=env, timeout=1800)
s2 = ROOT / ".build/selfhost/arukellt-s2.wasm"
if not s2.is_file():
    raise SystemExit("branch-built s2 missing")

env["ARUKELLT_SELFHOST_WASM"] = str(s2)
env["ARUKELLT_REQUIRE_RUNTIME_E2E"] = "1"
run(["wasmtime", "--version"], env=env, timeout=30)
run([str(wac), "--version"], env=env, timeout=30)
for gate in (
    "scripts/check/gate-076-wasi-p2-filesystem.py",
    "scripts/check/gate-676-std-host-fs-env-process.py",
    "scripts/check/gate-819-runtime-abi-core-op-lowering.py",
    "scripts/check/gate-841-real-wasi-network-abi.py",
):
    run([sys.executable, gate], env=env, timeout=900)
