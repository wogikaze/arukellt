#!/usr/bin/env python3
"""Measure the clean selfhost gc-host overlay and write a reproducible receipt."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from selfhost import checks  # noqa: E402
from selfhost.write_overlay_receipt import parse_stderr  # noqa: E402
from util.percentiles import percentile_linear  # noqa: E402

SOURCE = Path("src/compiler/main.ark")
DEFAULT_HOST = Path(".build/selfhost/arukellt-s3.wasm")
DEFAULT_S2 = Path(".build/selfhost/arukellt-s2.wasm")
DEFAULT_S3 = Path(".build/selfhost/arukellt-s3.wasm")
DEFAULT_PINNED = Path("bootstrap/arukellt-selfhost.wasm")
DEFAULT_WORKSPACE = Path(".build/selfhost/flat-src")
SCHEMA = "arukellt-selfhost-overlay-gate-summary-v2"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def require_clean_worktree() -> str:
    result = subprocess.run(
        ["git", "status", "--porcelain", "--untracked-files=all"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"git status failed: {result.stderr.strip()}")
    if result.stdout.strip():
        raise RuntimeError(
            "measurement requires a clean worktree; commit or remove changes first:\n"
            + result.stdout
        )
    revision = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    return revision.stdout.strip()


def tool_version(command: str) -> str:
    result = subprocess.run(
        [command, "--version"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    return (result.stdout or result.stderr).strip()


def validate(wasm_tools: str, path: Path) -> bool:
    result = subprocess.run(
        [wasm_tools, "validate", str(path)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    return result.returncode == 0


def parse_rss_max_kb(path: Path) -> int:
    """Parse GNU time's RSS output, including its failure-status prefix."""
    lines = [line.strip() for line in path.read_text(encoding="utf-8").splitlines()]
    for line in reversed(lines):
        if not line:
            continue
        try:
            return int(line)
        except ValueError:
            continue
    raise RuntimeError(f"/usr/bin/time wrote no numeric RSS value: {path}")


def summarize(samples: list[float]) -> dict[str, float | int]:
    if not samples:
        raise ValueError("cannot summarize an empty sample list")
    p95 = percentile_linear(samples, 95.0)
    if p95 is None:
        raise AssertionError("p95 disappeared for a non-empty sample list")
    return {
        "median": statistics.median(samples),
        "p95": round(p95, 3),
        "min": min(samples),
        "max": max(samples),
    }


def relative_path(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(ROOT.resolve()))
    except ValueError as exc:
        raise RuntimeError(f"path must be inside repository preopen: {path}") from exc


def run_one(
    run_number: int,
    host: Path,
    wasm_tools: str,
    wasmtime: str,
    workspace: Path,
    work_dir: Path,
    timeout: int,
) -> dict[str, object]:
    output = work_dir / f"overlay-{run_number:02d}.wasm"
    rss_file = work_dir / f"overlay-{run_number:02d}.rss"
    guest_output = f".overlay-goal-{work_dir.name}-{run_number:02d}.wasm"
    workspace_output = workspace / guest_output
    if host.suffix == ".cwasm":
        precompiled_flags = ["--allow-precompiled"]
    else:
        precompiled_flags = []
    command = [
        "/usr/bin/time",
        "-f",
        "%M",
        "-o",
        str(rss_file),
        wasmtime,
        "run",
        *precompiled_flags,
        *checks.WASMTIME_SELFHOST_WASM_FLAGS,
        "--wasm",
        "max-wasm-stack=16777216",
        "--dir",
        str(workspace),
        str(host),
        "--",
        "compile",
        str(SOURCE),
        "--target",
        "wasm32-gc",
        "--wasi-version",
        "wasi-p2",
        "--time",
        "-o",
        guest_output,
    ]
    started = time.perf_counter()
    result = subprocess.run(
        command,
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )
    wall_ms = round((time.perf_counter() - started) * 1000)
    if not rss_file.is_file():
        raise RuntimeError(f"/usr/bin/time did not write RSS for run {run_number}")
    rss_max_kb = parse_rss_max_kb(rss_file)
    if workspace_output.is_file():
        shutil.copyfile(workspace_output, output)
    phases, _slow_fns = parse_stderr(result.stderr)
    is_valid = output.is_file() and validate(wasm_tools, output)
    output_hash = sha256(output) if output.is_file() else ""
    if result.returncode != 0 or not is_valid:
        detail = (result.stdout + result.stderr).strip()[-1200:]
        raise RuntimeError(
            f"overlay run {run_number} failed: rc={result.returncode} "
            f"valid={is_valid}\n{detail}"
        )
    return {
        "run": run_number,
        "rc": result.returncode,
        "wall_ms": wall_ms,
        "rss_max_kb": rss_max_kb,
        "guest_total_ms": phases.get("total"),
        "output_sha256": output_hash,
        "valid": is_valid,
        "phases_ms": phases,
    }


def build_receipt(
    source_sha: str,
    pinned_sha: str,
    host: Path,
    workspace: Path,
    s2_sha: str,
    s3_sha: str,
    s3_valid: bool,
    samples: list[dict[str, object]],
    wasmtime_version: str,
    wasm_tools_version: str,
) -> dict[str, object]:
    wall = [int(sample["wall_ms"]) for sample in samples]
    rss = [int(sample["rss_max_kb"]) for sample in samples]
    guest = [
        int(sample["guest_total_ms"])
        for sample in samples
        if sample["guest_total_ms"] is not None
    ]
    output_hashes = sorted({str(sample["output_sha256"]) for sample in samples})
    return {
        "schema": SCHEMA,
        "recorded_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "source_sha": source_sha,
        "source": str(SOURCE),
        "host": relative_path(host),
        "host_sha256": sha256(host),
        "overlay": relative_path(workspace),
        "pinned": {
            "path": str(DEFAULT_PINNED),
            "sha256": pinned_sha,
        },
        "target": "wasm32-gc",
        "wasi_version": "wasi-p2",
        "source_cacheless": True,
        "fixpoint": {
            "s2_sha256": s2_sha,
            "s3_sha256": s3_sha,
            "s2_eq_s3": s2_sha == s3_sha,
            "s3_valid": s3_valid,
        },
        "tooling": {
            "wasmtime": wasmtime_version,
            "wasm_tools": wasm_tools_version,
        },
        "percentile_method": "linear-interpolation",
        "samples": samples,
        "wall_ms": summarize(wall),
        "rss_max_kb": summarize(rss),
        "guest_total_ms": summarize(guest) if guest else None,
        "output_sha256": output_hashes,
        "all_runs_exit_zero": all(sample["rc"] == 0 for sample in samples),
        "all_outputs_valid": all(sample["valid"] for sample in samples),
        "all_outputs_same_sha256": len(output_hashes) == 1,
        "user_goal": {
            "wall_p95_lt_10000": bool(percentile_linear(wall, 95.0) < 10000),
            "rss_max_lt_1000000": max(rss) < 1000000,
            "all_runs_exit_zero": all(sample["rc"] == 0 for sample in samples),
        },
        "canonical_plan_gate": {
            "wall_median_le_7000": statistics.median(wall) <= 7000,
            "rss_max_le_524288": max(rss) <= 524288,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=int, default=10)
    parser.add_argument("--host", type=Path, default=DEFAULT_HOST)
    parser.add_argument("--s2", type=Path, default=DEFAULT_S2)
    parser.add_argument("--s3", type=Path, default=DEFAULT_S3)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=600)
    args = parser.parse_args()
    if args.runs < 1:
        raise SystemExit("--runs must be positive")
    if not shutil.which("/usr/bin/time"):
        raise SystemExit("/usr/bin/time is required")
    source_sha = require_clean_worktree()
    host = (ROOT / args.host).resolve()
    s2 = (ROOT / args.s2).resolve()
    s3 = (ROOT / args.s3).resolve()
    pinned = (ROOT / DEFAULT_PINNED).resolve()
    workspace = (ROOT / DEFAULT_WORKSPACE).resolve()
    for path in (host, s2, s3):
        if not path.is_file():
            raise SystemExit(f"missing selfhost artifact: {path}")
    if not pinned.is_file():
        raise SystemExit(f"missing pinned selfhost artifact: {pinned}")
    wasm_tools = shutil.which("wasm-tools")
    if wasm_tools is None:
        raise SystemExit("wasm-tools is required")
    wasmtime = shutil.which("wasmtime")
    if wasmtime is None:
        raise SystemExit("wasmtime is required")
    try:
        workspace = checks._prepare_flattened_selfhost_source(ROOT).resolve()
    except Exception as exc:
        raise SystemExit(f"failed to prepare selfhost overlay: {exc}") from exc
    s2_sha = sha256(s2)
    s3_sha = sha256(s3)
    pinned_sha = sha256(pinned)
    s3_valid = validate(wasm_tools, s3)
    if not s3_valid:
        raise SystemExit(f"stage 3 artifact is invalid: {s3}")

    build_dir = ROOT / ".build"
    build_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="overlay-goal-", dir=build_dir) as raw_dir:
        work_dir = Path(raw_dir)
        samples = [
            run_one(index, host, wasm_tools, wasmtime, workspace, work_dir, args.timeout)
            for index in range(1, args.runs + 1)
        ]
    receipt = build_receipt(
        source_sha,
        pinned_sha,
        host,
        workspace,
        s2_sha,
        s3_sha,
        s3_valid,
        samples,
        tool_version("wasmtime"),
        tool_version("wasm-tools"),
    )
    output = (ROOT / args.out).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(
        f"overlay goal receipt: runs={args.runs} source={source_sha} "
        f"wall_p95={receipt['wall_ms']['p95']}ms rss_max={receipt['rss_max_kb']['max']}KiB"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
