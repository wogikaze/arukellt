#!/usr/bin/env python3
from __future__ import annotations

import json
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "arukellt"
PERF_CASES = [
    ROOT / "docs" / "examples" / "hello.ark",
    ROOT / "docs" / "examples" / "vec.ark",
    ROOT / "docs" / "examples" / "closure.ark",
    ROOT / "docs" / "sample" / "parser.ark",
]
FIXTURE_MANIFEST = ROOT / "tests" / "fixtures" / "manifest.txt"
OUTPUT_DIR = ROOT / "tests" / "baselines"


def run(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True, check=False)


def ensure_binary() -> None:
    if BIN.exists():
        return
    build = run(["cargo", "build", "--release", "-p", "arukellt"])
    if build.returncode != 0:
        sys.stderr.write(build.stdout)
        sys.stderr.write(build.stderr)
        raise SystemExit(build.returncode)


def benchmark_command(args: list[str], iterations: int = 5) -> dict:
    samples: list[float] = []
    last_proc: subprocess.CompletedProcess[str] | None = None
    for _ in range(iterations):
        start = time.perf_counter()
        proc = run(args)
        elapsed_ms = (time.perf_counter() - start) * 1000.0
        last_proc = proc
        if proc.returncode != 0:
            return {
                "status": proc.returncode,
                "median_ms": None,
                "stderr_head": proc.stderr.splitlines()[:10],
                "stdout_head": proc.stdout.splitlines()[:10],
            }
        samples.append(elapsed_ms)
    assert last_proc is not None
    return {
        "status": 0,
        "median_ms": round(statistics.median(samples), 3),
        "stderr_head": last_proc.stderr.splitlines()[:10],
        "stdout_head": last_proc.stdout.splitlines()[:10],
    }


def collect_perf_baseline() -> dict:
    rows = []
    for case in PERF_CASES:
        check_result = benchmark_command([str(BIN), "check", str(case)])
        compile_result = benchmark_command([str(BIN), "compile", str(case), "--output", "/tmp/arukellt-baseline.wasm"])
        rows.append(
            {
                "file": str(case.relative_to(ROOT)),
                "check": check_result,
                "compile": compile_result,
            }
        )
    return {
        "generated_at": time.strftime("%Y-%m-%d"),
        "compile_time_thresholds": {
            "check_vs_baseline_max_regression_percent": 10,
            "compile_vs_baseline_max_regression_percent": 20,
        },
        "cases": rows,
    }


def parse_manifest() -> list[tuple[str, str]]:
    entries: list[tuple[str, str]] = []
    for line in FIXTURE_MANIFEST.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        kind, path = line.split(":", 1)
        entries.append((kind, path))
    return entries


def collect_fixture_baseline() -> dict:
    rows = []
    for kind, rel in parse_manifest():
        fixture = ROOT / "tests" / "fixtures" / rel
        proc = run([str(BIN), "run", str(fixture)])
        primary_stream = proc.stderr if "diag" in kind else proc.stdout
        rows.append(
            {
                "kind": kind,
                "path": rel,
                "status": proc.returncode,
                "stdout": proc.stdout,
                "stderr": proc.stderr,
                "primary_text": primary_stream.strip().splitlines()[:5],
            }
        )
    return {
        "generated_at": time.strftime("%Y-%m-%d"),
        "fixture_count": len(rows),
        "entries": rows,
    }


def collect_api_baseline() -> dict:
    parse_i64 = (ROOT / "tests" / "fixtures" / "stdlib_string" / "parse_i64.expected").read_text().splitlines()
    parse_f64 = (ROOT / "tests" / "fixtures" / "stdlib_string" / "parse_f64.expected").read_text().splitlines()
    return {
        "generated_at": time.strftime("%Y-%m-%d"),
        "parse_i64": {
            "return_shape": "Result<i64, String>",
            "observed_output": parse_i64,
        },
        "parse_f64": {
            "return_shape": "Result<f64, String>",
            "observed_output": parse_f64,
        },
    }


def main() -> None:
    ensure_binary()
    OUTPUT_DIR.mkdir(exist_ok=True)
    (OUTPUT_DIR / "perf-baseline.json").write_text(json.dumps(collect_perf_baseline(), indent=2) + "\n")
    (OUTPUT_DIR / "fixture-baseline.json").write_text(json.dumps(collect_fixture_baseline(), indent=2) + "\n")
    (OUTPUT_DIR / "api-baseline.json").write_text(json.dumps(collect_api_baseline(), indent=2) + "\n")
    print("Wrote tests/baselines/perf-baseline.json")
    print("Wrote tests/baselines/fixture-baseline.json")
    print("Wrote tests/baselines/api-baseline.json")


if __name__ == "__main__":
    main()
