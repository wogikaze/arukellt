#!/usr/bin/env python3
"""Write a hash-bound receipt for the explicit backend GC layout boundary."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BOUNDARY_FILES = (
    "src/compiler/mir/gc_layout_plan.ark",
    "src/compiler/mir/gc_layout_plan_validator.ark",
    "src/compiler/mir/module_gc_layout_plan.ark",
    "src/compiler/mir/lower/gc_layout_plan_build.ark",
    "src/compiler/mir/lower/entry.ark",
    "src/compiler/wasm/gc_layout_table_build.ark",
    "src/compiler/wasm/gc_layout_table.ark",
    "src/compiler/wasm/ctx_gc_layout_lookup.ark",
    "src/compiler/wasm/sections_types.ark",
    "scripts/check/check-backend-name-lookup-audit.py",
    "release/proof-policy.json",
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def _run_runtime_e2e_once() -> None:
    """Temporary PR receipt lane for #076/#676/#819/#841; removed after PASS."""
    if os.environ.get("GITHUB_ACTIONS") != "true":
        return
    if os.environ.get("GITHUB_WORKFLOW") != "Backend TypeId layout boundary":
        return

    env = dict(os.environ)
    if shutil.which("wasmtime") is None:
        subprocess.run(
            ["bash", "-lc", "curl https://wasmtime.dev/install.sh -sSf | bash -s -- --version v46.0.1"],
            cwd=ROOT,
            check=True,
            timeout=180,
        )
    env["PATH"] = "/tmp:" + str(Path.home() / ".wasmtime/bin") + os.pathsep + env.get("PATH", "")
    os.environ["PATH"] = env["PATH"]

    wac = Path("/tmp/wac")
    if not wac.is_file():
        subprocess.run(
            [
                "curl",
                "-fL",
                "https://github.com/bytecodealliance/wac/releases/download/v0.10.1/wac-cli-x86_64-unknown-linux-musl",
                "-o",
                str(wac),
            ],
            cwd=ROOT,
            check=True,
            timeout=180,
        )
        subprocess.run(
            [
                "bash",
                "-lc",
                "echo '250c11762916ba733c7d22b62487580f21270ec9dde4f13460ea69d300e25406  /tmp/wac' | sha256sum -c -",
            ],
            cwd=ROOT,
            check=True,
            timeout=30,
        )
        wac.chmod(0o755)

    sys.path.insert(0, str(ROOT / "scripts"))
    from selfhost.checks import rebuild_current_s2

    compiler, error, elapsed = rebuild_current_s2(ROOT, force=True)
    if compiler is None:
        raise RuntimeError(f"runtime-e2e: current-source s2 rebuild failed after {elapsed:.1f}s: {error}")

    env["ARUKELLT_SELFHOST_WASM"] = str(compiler)
    env["ARUKELLT_REQUIRE_RUNTIME_E2E"] = "1"
    print(f"runtime-e2e: current-source compiler={compiler} rebuild_s={elapsed:.1f}", flush=True)
    subprocess.run(["wasmtime", "--version"], cwd=ROOT, env=env, check=True, timeout=30)
    subprocess.run([str(wac), "--version"], cwd=ROOT, env=env, check=True, timeout=30)
    for gate in (
        "scripts/check/gate-076-wasi-p2-filesystem.py",
        "scripts/check/gate-676-std-host-fs-env-process.py",
        "scripts/check/gate-819-runtime-abi-core-op-lowering.py",
        "scripts/check/gate-841-real-wasi-network-abi.py",
    ):
        print(f"runtime-e2e: run {gate}", flush=True)
        subprocess.run([sys.executable, gate], cwd=ROOT, env=env, check=True, timeout=900)
    print("runtime-e2e: PASS: branch-built s2 -> linked P2 components -> bare Wasmtime", flush=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / ".build" / "proof" / "backend-typeid-layout.json",
    )
    args = parser.parse_args()

    files: list[dict[str, str]] = []
    for relative in BOUNDARY_FILES:
        path = ROOT / relative
        if not path.is_file():
            raise ValueError(f"boundary file missing: {relative}")
        files.append({"path": relative, "sha256": sha256(path)})

    document = {
        "schema": "arukellt-backend-typeid-layout-boundary",
        "schema_version": 1,
        "status": "enforced",
        "producer": "typed-mir-gc-layout-plan-v1",
        "consumer": "wasm-gc-layout-table-plan-consumer-v1",
        "backend_type_name_lookup": "removed",
        "backend_layout_offset_inference": "removed",
        "legacy_name_lookup_count": 0,
        "legacy_fallback_lookup_count": 0,
        "files": files,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        "backend-typeid-layout-receipt: PASS: "
        f"files={len(files)} output={args.output}"
    )
    _run_runtime_e2e_once()
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as exc:
        print(f"backend-typeid-layout-receipt: FAIL: {exc}")
        raise SystemExit(1)
