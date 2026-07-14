#!/usr/bin/env python3
from __future__ import annotations

import json
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
BIN = ROOT / "target" / "release" / "arukellt"
SELFHOST_WRAPPER = ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
PERF_CASES = [
    ROOT / "docs" / "examples" / "hello.ark",
    ROOT / "docs" / "examples" / "vec.ark",
    ROOT / "docs" / "examples" / "closure.ark",
    ROOT / "docs" / "sample" / "parser.ark",
]
FIXTURE_MANIFEST = ROOT / "tests" / "fixtures" / "manifest.txt"
OUTPUT_DIR = ROOT / "tests" / "baselines"
TMP_DIR = ROOT / "target" / "ci" / "baselines"


def run(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True, check=False)


def ensure_binary() -> None:
    if BIN.exists():
        return
    if SELFHOST_WRAPPER.exists() and SELFHOST_WRAPPER.stat().st_mode & 0o111:
        BIN.parent.mkdir(parents=True, exist_ok=True)
        import shutil
        shutil.copy2(str(SELFHOST_WRAPPER), str(BIN))
        return
    raise SystemExit(
        f"error: no arukellt entrypoint found.\n"
        f"  Tried: {BIN} (binary) and {SELFHOST_WRAPPER} (selfhost wrapper).\n"
        f"  Use scripts/run/arukellt-selfhost.sh or set ARUKELLT_BIN."
    )


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
    TMP_DIR.mkdir(parents=True, exist_ok=True)
    for case in PERF_CASES:
        rel_case = str(case.relative_to(ROOT))
        t1_out = TMP_DIR / "arukellt-baseline-t1.wasm"
        t3_out = TMP_DIR / "arukellt-baseline-t3.wasm"
        check_result = benchmark_command([str(BIN), "check", rel_case])
        compile_t1 = benchmark_command([str(BIN), "compile", rel_case, "--target", "wasm32", "--output", str(t1_out.relative_to(ROOT))])
        compile_t3 = benchmark_command([str(BIN), "compile", rel_case, "--target", "wasm32-gc", "--output", str(t3_out.relative_to(ROOT))])
        t1_size = t1_out.stat().st_size if t1_out.exists() else None
        t3_size = t3_out.stat().st_size if t3_out.exists() else None
        rows.append(
            {
                "file": str(case.relative_to(ROOT)),
                "check": check_result,
                "compile_t1": compile_t1,
                "compile_t3": compile_t3,
                "binary_size_t1": t1_size,
                "binary_size_t3": t3_size,
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
        proc = run([str(BIN), "run", f"tests/fixtures/{rel}"])
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
