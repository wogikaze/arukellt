#!/usr/bin/env python3
"""Measure wasmtime peak heap and phase timing for selfhost compiler compiles.

Records peak linear memory (KiB) and optional ARUKELLT_DUMP_PHASES stderr markers
when compiling ``src/compiler/main.ark`` (or another entry) under wasmtime.

Usage:
    python3 scripts/selfhost/measure_compile_heap.py
    python3 scripts/selfhost/measure_compile_heap.py --compiler bootstrap/arukellt-selfhost.wasm
    python3 scripts/selfhost/measure_compile_heap.py --overlay --phases parse,resolve,mir,emit
    python3 scripts/selfhost/measure_compile_heap.py --json
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_COMPILER = REPO_ROOT / "bootstrap" / "arukellt-selfhost.wasm"
DEFAULT_SOURCE = "src/compiler/main.ark"
DEFAULT_TARGET = "wasm32-wasi-p1"
PHASE_RE = re.compile(r"\[phase:(\w+)\]")


def _find_wasmtime() -> str:
    wasmtime = shutil.which("wasmtime")
    if wasmtime is None:
        print("error: wasmtime not found in PATH", file=sys.stderr)
        sys.exit(2)
    return wasmtime


def _compiler_candidates() -> list[Path]:
    return [
        REPO_ROOT / ".build" / "selfhost" / "arukellt-pinned-bootstrap.wasm",
        REPO_ROOT / ".build" / "selfhost" / "arukellt-hop-bootstrap.wasm",
        REPO_ROOT / "bootstrap" / "arukellt-selfhost.wasm",
    ]


def _resolve_compiler(explicit: str | None) -> Path:
    if explicit:
        path = Path(explicit)
        if not path.is_file():
            print(f"error: compiler wasm not found: {path}", file=sys.stderr)
            sys.exit(2)
        return path
    for cand in _compiler_candidates():
        if cand.is_file():
            return cand
    print("error: no selfhost compiler wasm found", file=sys.stderr)
    sys.exit(2)


def _prepare_overlay_workspace() -> Path:
    # Reuse checks.py overlay builder when available.
    sys.path.insert(0, str(REPO_ROOT / "scripts"))
    from selfhost.checks import _prepare_bootstrap_workspace  # noqa: WPS433

    return _prepare_bootstrap_workspace(REPO_ROOT)


def _parse_phases(stderr: str) -> list[str]:
    return PHASE_RE.findall(stderr)


def _peak_heap_kib(stderr: str) -> int | None:
    # wasmtime 30+ reports: "Peak heap usage: N bytes"
    match = re.search(r"Peak heap usage:\s*(\d+)\s*bytes", stderr)
    if match:
        return (int(match.group(1)) + 1023) // 1024
    # Older builds: "maximum resident set size: N" from /usr/bin/time wrapper
    match = re.search(r"maximum resident set size:\s*(\d+)", stderr)
    if match:
        return int(match.group(1))
    return None


def measure_compile(
    compiler: Path,
    source: str,
    *,
    overlay: bool,
    phases: str,
    timeout: int,
) -> dict:
    wasmtime = _find_wasmtime()
    workspace: Path | None = None
    cwd = REPO_ROOT
    if overlay:
        workspace = _prepare_overlay_workspace()
        cwd = workspace

    env = os.environ.copy()
    if phases:
        env["ARUKELLT_DUMP_PHASES"] = phases

    out_rel = ".build/selfhost/measure-out.wasm"
    guest_out = out_rel
    dirs = ["--dir", str(REPO_ROOT)]
    run_cwd = REPO_ROOT
    if workspace is not None:
        dirs = ["--dir", str(workspace), "--dir", str(REPO_ROOT)]
        guest_out = "bootstrap-out.wasm"
        run_cwd = REPO_ROOT

    with tempfile.TemporaryDirectory(prefix="ark-measure-") as tmp:
        cmd = [
            wasmtime,
            "run",
            *dirs,
            str(compiler.resolve()),
            "--",
            "compile",
            source,
            "--target",
            DEFAULT_TARGET,
            "-o",
            guest_out,
        ]
        proc = subprocess.run(
            cmd,
            cwd=str(run_cwd),
            capture_output=True,
            text=True,
            timeout=timeout,
            env=env,
        )
        if workspace is not None and proc.returncode == 0:
            staged = workspace / guest_out
            final = REPO_ROOT / out_rel
            final.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(staged, final)
        out_abs = REPO_ROOT / out_rel
        wasm_size = out_abs.stat().st_size if out_abs.is_file() else None
        return {
            "compiler": str(compiler.relative_to(REPO_ROOT)),
            "source": source,
            "overlay": overlay,
            "exit_code": proc.returncode,
            "peak_heap_kib": _peak_heap_kib(proc.stderr),
            "phases_seen": _parse_phases(proc.stderr),
            "wasm_bytes": wasm_size,
            "stderr_tail": "\n".join(proc.stderr.splitlines()[-8:]),
        }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", help="Path to compiler wasm (default: first available)")
    parser.add_argument("--source", default=DEFAULT_SOURCE, help="Source entry relative to repo or overlay root")
    parser.add_argument("--overlay", action="store_true", help="Compile via bootstrap overlay workspace")
    parser.add_argument("--phases", default="", help="ARUKELLT_DUMP_PHASES value (comma-separated)")
    parser.add_argument("--timeout", type=int, default=900, help="Compile timeout seconds")
    parser.add_argument("--json", action="store_true", help="Emit JSON only")
    args = parser.parse_args()

    compiler = _resolve_compiler(args.compiler)
    result = measure_compile(
        compiler,
        args.source,
        overlay=args.overlay,
        phases=args.phases,
        timeout=args.timeout,
    )

    if args.json:
        print(json.dumps(result, indent=2))
        return

    print(f"compiler: {result['compiler']}")
    print(f"source:   {result['source']}")
    print(f"overlay:  {result['overlay']}")
    print(f"exit:     {result['exit_code']}")
    if result["peak_heap_kib"] is not None:
        print(f"peak heap: {result['peak_heap_kib']} KiB")
    else:
        print("peak heap: (not reported — try wasmtime ≥ 30)")
    if result["wasm_bytes"] is not None:
        print(f"wasm size: {result['wasm_bytes']} bytes")
    if result["phases_seen"]:
        print(f"phases:   {', '.join(result['phases_seen'])}")
    if result["exit_code"] != 0:
        print("--- stderr (tail) ---")
        print(result["stderr_tail"])
        sys.exit(1)


if __name__ == "__main__":
    main()
