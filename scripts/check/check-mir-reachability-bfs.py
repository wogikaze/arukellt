#!/usr/bin/env python3
"""MIR reachability queue-BFS gate (#823).

Ensures a selfhost compiler artifact exists, runs the reachability unit tests
(no silent skip), and optionally records legacy-fixpoint vs queue-BFS wall/RSS
when ``ARUKELLT_REACHABILITY_AB=1``.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import time
import unittest
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
RECEIPT = REPO_ROOT / ".build" / "selfhost" / "reachability-bfs-receipt.json"
AB_WORKLOAD = "src/compiler/main.ark"


def _ensure_compiler() -> Path:
    sys.path.insert(0, str(REPO_ROOT / "scripts"))
    from selfhost import checks

    runtime = REPO_ROOT / checks.S2_RUNTIME_WASM_REL
    if runtime.is_file():
        invalid = checks._reject_invalid_compiler_wasm(runtime)
        if not invalid:
            os.environ["ARUKELLT_SELFHOST_WASM"] = str(runtime)
            return runtime
    print("[reachability-bfs] building stubbed s2-runtime via selfhost build-compiler...")
    path, err, elapsed = checks.rebuild_current_s2(REPO_ROOT, force=True)
    if path is None:
        print(f"FAIL: could not build compiler artifact: {err}", file=sys.stderr)
        raise SystemExit(2)
    print(f"[reachability-bfs] compiler ready: {path} ({elapsed:.1f}s)")
    os.environ["ARUKELLT_SELFHOST_WASM"] = str(path)
    return path


def _run_unit_tests() -> dict:
    if str(REPO_ROOT) not in sys.path:
        sys.path.insert(0, str(REPO_ROOT))
    loader = unittest.TestLoader()
    suite = loader.loadTestsFromName("scripts.tests.test_mir_reachability_bfs")
    runner = unittest.TextTestRunner(stream=sys.stderr, verbosity=2)
    result = runner.run(suite)
    if result.errors or result.failures or result.unexpectedSuccesses:
        print("FAIL: mir reachability BFS unit tests", file=sys.stderr)
        raise SystemExit(1)
    if result.testsRun == 0:
        print("FAIL: no reachability BFS tests were collected", file=sys.stderr)
        raise SystemExit(1)
    if result.skipped:
        # Gate builds the artifact first; any skip is a hard failure.
        names = [str(t) for t, _ in result.skipped]
        print(f"FAIL: tests skipped (not acceptable evidence): {names}", file=sys.stderr)
        raise SystemExit(1)
    return {"tests_run": result.testsRun, "failures": 0, "skipped": 0}


def _parse_time_v(stderr: str) -> dict:
    wall = None
    rss = None
    m = re.search(r"Elapsed \(wall clock\) time \(h:mm:ss or m:ss\): ([0-9:]+)", stderr)
    if m:
        parts = m.group(1).split(":")
        if len(parts) == 2:
            wall = int(parts[0]) * 60 + float(parts[1])
        elif len(parts) == 3:
            wall = int(parts[0]) * 3600 + int(parts[1]) * 60 + float(parts[2])
    m = re.search(r"Maximum resident set size \(kbytes\): (\d+)", stderr)
    if m:
        rss = int(m.group(1))
    return {"wall_s": wall, "peak_rss_kib": rss}


def _parse_phase_ms(text: str) -> dict[str, int]:
    out: dict[str, int] = {}
    for label in (
        "decl_emit",
        "reachability",
        "sync",
        "propagate",
        "mir_opt",
        "mir_verify",
        "emit",
        "lower",
        "total",
    ):
        # lower.X or bare X
        m = re.search(rf"\[arukellt\] (?:lower\.)?{label}: (\d+)ms", text)
        if m:
            out[label] = int(m.group(1))
    m = re.search(r"lower\.reachability_fns: before=(\d+) after=(\d+)", text)
    if m:
        out["fns_before"] = int(m.group(1))
        out["fns_after"] = int(m.group(2))
    m = re.search(r"lower\.reachability_blocks: before=(\d+) after=(\d+)", text)
    if m:
        out["blocks_before"] = int(m.group(1))
        out["blocks_after"] = int(m.group(2))
    m = re.search(r"lower\.reachability_insts: before=(\d+) after=(\d+)", text)
    if m:
        out["insts_before"] = int(m.group(1))
        out["insts_after"] = int(m.group(2))
    return out


def _compile_workload(compiler: Path, *, legacy: bool) -> dict:
    sys.path.insert(0, str(REPO_ROOT / "scripts"))
    from selfhost import checks

    wasmtime = shutil.which("wasmtime")
    if wasmtime is None:
        raise SystemExit("wasmtime required for A/B measurement")
    workspace = checks._prepare_bootstrap_workspace(REPO_ROOT)
    out_name = "bootstrap-out-ab-legacy.wasm" if legacy else "bootstrap-out-ab-bfs.wasm"
    staged = workspace / out_name
    if staged.exists():
        staged.unlink()
    env = {**os.environ}
    if legacy:
        env["MIR_REACHABILITY_LEGACY_FIXPOINT"] = "1"
    else:
        env.pop("MIR_REACHABILITY_LEGACY_FIXPOINT", None)
    cmd = [
        "/usr/bin/time",
        "-v",
        wasmtime,
        "run",
        *checks.WASMTIME_SELFHOST_WASM_FLAGS,
        "--dir",
        str(workspace),
        "--dir",
        str(REPO_ROOT),
        str(compiler),
        "--",
        "compile",
        checks.SELFHOST_SOURCE_REL,
        "--target",
        checks.SELFHOST_TARGET,
        "--wasi-version",
        checks.SELFHOST_WASI_VERSION,
        "--time",
        "-o",
        out_name,
    ]
    started = time.time()
    result = subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )
    text = (result.stderr or "") + "\n" + (result.stdout or "")
    metrics = _parse_time_v(text)
    metrics.update(_parse_phase_ms(text))
    metrics["returncode"] = result.returncode
    metrics["legacy"] = legacy
    metrics["inner_wall_s"] = time.time() - started
    metrics["compilation_succeeded"] = "compilation succeeded" in text
    return metrics


def _maybe_run_ab(compiler: Path) -> dict | None:
    if os.environ.get("ARUKELLT_REACHABILITY_AB") != "1":
        return {
            "ran": False,
            "reason": "ARUKELLT_REACHABILITY_AB!=1; A/B not executed in this gate run",
        }
    print("[reachability-bfs] A/B: queue BFS vs legacy fixpoint on full selfhost...")
    bfs = _compile_workload(compiler, legacy=False)
    legacy = _compile_workload(compiler, legacy=True)
    return {"ran": True, "workload": AB_WORKLOAD, "bfs": bfs, "legacy_fixpoint": legacy}


def main() -> int:
    compiler = _ensure_compiler()
    tests = _run_unit_tests()
    ab = _maybe_run_ab(compiler)
    receipt = {
        "schema_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "compiler_wasm": str(compiler.relative_to(REPO_ROOT)),
        "unit_tests": tests,
        "ab_comparison": ab,
    }
    RECEIPT.parent.mkdir(parents=True, exist_ok=True)
    RECEIPT.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(f"PASS: mir reachability BFS gate (receipt {RECEIPT.relative_to(REPO_ROOT)})")
    if ab and not ab.get("ran"):
        print(f"NOTE: {ab.get('reason')}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
