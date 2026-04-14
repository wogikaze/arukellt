#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_TARGET = "wasm32-wasi-p1"
DEFAULT_CURRENT_RESULTS = ROOT / "tests" / "baselines" / "perf" / "current.json"
DEFAULT_BASELINE_RESULTS = ROOT / "tests" / "baselines" / "perf" / "baselines.json"
DEFAULT_RESULTS_MARKDOWN = ROOT / "docs" / "process" / "benchmark-results.md"
SCHEMA_VERSION = "arukellt-bench-v1"
THRESHOLDS = {
    "compile_ms": 20,
    "run_ms": 10,
    "binary_bytes": 15,
}


@dataclass(frozen=True)
class BenchmarkCase:
    name: str
    source: str
    expected: str
    description: str
    tags: tuple[str, ...]
    runtime_args: tuple[str, ...] = ()
    metrics: tuple[str, ...] = ("compile", "runtime", "size", "memory")


BENCHMARKS: tuple[BenchmarkCase, ...] = (
    BenchmarkCase(
        name="fib",
        source="benchmarks/fib.ark",
        expected="benchmarks/fib.expected",
        description="Iterative Fibonacci(35)",
        tags=("cpu-bound", "loop", "scalar"),
    ),
    BenchmarkCase(
        name="binary_tree",
        source="benchmarks/binary_tree.ark",
        expected="benchmarks/binary_tree.expected",
        description="Recursive node counting (depth 20)",
        tags=("recursion-heavy", "allocation-light", "call-heavy"),
    ),
    BenchmarkCase(
        name="vec_ops",
        source="benchmarks/vec_ops.ark",
        expected="benchmarks/vec_ops.expected",
        description="Vec push/sum/contains (1k elements)",
        tags=("allocation-heavy", "container", "iteration"),
    ),
    BenchmarkCase(
        name="string_concat",
        source="benchmarks/string_concat.ark",
        expected="benchmarks/string_concat.expected",
        description="String concat in loop (100 iterations)",
        tags=("string-heavy", "allocation-heavy", "gc-pressure"),
    ),
    BenchmarkCase(
        name="parse_tree_distance",
        source="benchmarks/bench_parse_tree_distance.ark",
        expected="benchmarks/bench_parse_tree_distance.expected",
        description="Packed-tree distance validator on a 1200-node star matrix",
        tags=("parse", "allocation-heavy", "container", "iteration"),
        runtime_args=("run", "--dir=."),
    ),
)

MODE_PRESETS: dict[str, dict[str, Any]] = {
    "quick": {
        "compile_iterations": 1,
        "runtime_iterations": 1,
        "runtime_warmups": 0,
        "description": "single-sample local smoke benchmark",
    },
    "full": {
        "compile_iterations": 5,
        "runtime_iterations": 5,
        "runtime_warmups": 1,
        "description": "default local benchmark profile",
    },
    "compare": {
        "compile_iterations": 5,
        "runtime_iterations": 5,
        "runtime_warmups": 1,
        "description": "measure current results and compare with baseline",
    },
    "ci": {
        "compile_iterations": 5,
        "runtime_iterations": 5,
        "runtime_warmups": 1,
        "description": "compare current results with baseline and fail on regression",
    },
    "update-baseline": {
        "compile_iterations": 5,
        "runtime_iterations": 5,
        "runtime_warmups": 1,
        "description": "measure current results and replace baseline",
    },
}


def rel(path: Path) -> str:
    return str(path.relative_to(ROOT))


def resolve_compiler(explicit: str | None) -> Path:
    if explicit:
        return Path(explicit).expanduser().resolve()
    env_compiler = os.environ.get("ARUKELLT_BIN") or os.environ.get("ARUKELLT")
    if env_compiler:
        return Path(env_compiler).expanduser().resolve()
    candidates = [
        ROOT / "target" / "release" / "arukellt",
        ROOT / "target" / "debug" / "arukellt",
    ]
    for candidate in candidates:
        if candidate.exists() and os.access(candidate, os.X_OK):
            return candidate
    raise SystemExit(
        "arukellt binary not found. Run `cargo build --release -p arukellt` or use `mise bench`."
    )


def iso_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat()


def command_display(cmd: list[str]) -> str:
    return " ".join(str(part) for part in cmd)


PHASE_PATTERN = __import__("re").compile(r"\[arukellt\]\s+(\w+):\s+([\d.]+)ms")


def parse_phase_timings(stderr_text: str) -> dict[str, float]:
    """Parse per-phase timings from `arukellt compile --time` stderr output.

    Returns a dict mapping phase name → time in milliseconds.
    Phases: lex, parse, resolve, typecheck, lower, opt, emit, total.
    Returns an empty dict if no phase lines are found.
    """
    result: dict[str, float] = {}
    for match in PHASE_PATTERN.finditer(stderr_text):
        result[match.group(1)] = float(match.group(2))
    return result


def maybe_read_expected(path: Path) -> str | None:
    if not path.exists():
        return None
    return path.read_text()


def command_with_optional_time(cmd: list[str], time_bin: str | None, mem_out: Path | None) -> list[str]:
    if time_bin is None or mem_out is None:
        return cmd
    return [time_bin, "-f", "%M", "-o", str(mem_out), *cmd]


def run_measured(
    cmd: list[str],
    *,
    cwd: Path,
    time_bin: str | None,
    capture_output: bool = True,
) -> dict[str, Any]:
    mem_file: Path | None = None
    if time_bin is not None:
        handle = tempfile.NamedTemporaryFile(prefix="arukellt-bench-rss-", suffix=".txt", delete=False)
        handle.close()
        mem_file = Path(handle.name)
    wrapped = command_with_optional_time(cmd, time_bin, mem_file)
    started = time.perf_counter()
    proc = subprocess.run(
        wrapped,
        cwd=cwd,
        text=True,
        capture_output=capture_output,
        check=False,
    )
    elapsed_ms = round((time.perf_counter() - started) * 1000.0, 3)
    rss_kb = None
    if mem_file is not None:
        try:
            text = mem_file.read_text().strip()
            rss_kb = int(text) if text else None
        except (OSError, ValueError):
            rss_kb = None
        finally:
            try:
                mem_file.unlink()
            except OSError:
                pass
    return {
        "returncode": proc.returncode,
        "stdout": proc.stdout,
        "stderr": proc.stderr,
        "elapsed_ms": elapsed_ms,
        "max_rss_kb": rss_kb,
        "command": command_display(cmd),
    }


def summarize_samples(samples: list[dict[str, Any]], field: str) -> float | int | None:
    values = [sample[field] for sample in samples if sample.get(field) is not None]
    if not values:
        return None
    median = statistics.median(values)
    return int(median) if field == "max_rss_kb" else round(float(median), 3)


def ensure_parent(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def measure_compile(
    compiler: Path,
    case: BenchmarkCase,
    *,
    target: str,
    iterations: int,
    time_bin: str | None,
    work_dir: Path,
) -> dict[str, Any]:
    wasm_path = work_dir / f"{case.name}.wasm"
    samples: list[dict[str, Any]] = []
    phase_samples: dict[str, list[float]] = {}
    last_output: dict[str, Any] | None = None
    for _ in range(iterations):
        try:
            wasm_path.unlink()
        except FileNotFoundError:
            pass
        output = run_measured(
            [str(compiler), "compile", "--time", "--target", target, "-o", str(wasm_path), str(ROOT / case.source)],
            cwd=ROOT,
            time_bin=time_bin,
        )
        last_output = output
        if output["returncode"] != 0:
            return {
                "status": "error",
                "iterations": len(samples) + 1,
                "samples_ms": [sample["elapsed_ms"] for sample in samples],
                "median_ms": None,
                "max_rss_kb": None,
                "binary_bytes": None,
                "command": output["command"],
                "stderr_head": output["stderr"].splitlines()[:10],
            }
        samples.append(output)
        for phase, ms in parse_phase_timings(output["stderr"]).items():
            phase_samples.setdefault(phase, []).append(ms)
    binary_bytes = wasm_path.stat().st_size if wasm_path.exists() else None
    # Compute per-phase medians
    phase_ms: dict[str, float] | None = None
    if phase_samples:
        phase_ms = {
            phase: round(float(statistics.median(vals)), 3)
            for phase, vals in phase_samples.items()
        }
    result: dict[str, Any] = {
        "status": "ok",
        "iterations": iterations,
        "samples_ms": [sample["elapsed_ms"] for sample in samples],
        "median_ms": summarize_samples(samples, "elapsed_ms"),
        "max_rss_kb": summarize_samples(samples, "max_rss_kb"),
        "binary_bytes": binary_bytes,
        "command": last_output["command"] if last_output else None,
        "wasm_path": str(wasm_path),
    }
    if phase_ms is not None:
        result["phase_ms"] = phase_ms
    return result


def measure_runtime(
    wasm_path: Path,
    expected_text: str | None,
    *,
    iterations: int,
    warmups: int,
    wasmtime_bin: str | None,
    time_bin: str | None,
    runtime_args: tuple[str, ...] = (),
) -> dict[str, Any]:
    if wasmtime_bin is None:
        return {
            "status": "skipped",
            "reason": "wasmtime not found in PATH",
            "iterations": 0,
            "warmups": warmups,
            "samples_ms": [],
            "median_ms": None,
            "max_rss_kb": None,
            "stdout": None,
            "correctness": "skipped",
            "command": None,
        }
    runtime_cmd = [wasmtime_bin, *runtime_args, str(wasm_path)]
    warmup_cmd = runtime_cmd
    for _ in range(warmups):
        warm = run_measured(warmup_cmd, cwd=ROOT, time_bin=time_bin)
        if warm["returncode"] != 0:
            return {
                "status": "error",
                "reason": "warmup execution failed",
                "iterations": 0,
                "warmups": warmups,
                "samples_ms": [],
                "median_ms": None,
                "max_rss_kb": warm.get("max_rss_kb"),
                "stdout": warm["stdout"],
                "stderr_head": warm["stderr"].splitlines()[:10],
                "correctness": "failed",
                "command": warm["command"],
            }
    samples: list[dict[str, Any]] = []
    last_output: dict[str, Any] | None = None
    for _ in range(iterations):
        output = run_measured(runtime_cmd, cwd=ROOT, time_bin=time_bin)
        last_output = output
        if output["returncode"] != 0:
            return {
                "status": "error",
                "reason": "benchmark execution failed",
                "iterations": len(samples) + 1,
                "warmups": warmups,
                "samples_ms": [sample["elapsed_ms"] for sample in samples],
                "median_ms": None,
                "max_rss_kb": None,
                "stdout": output["stdout"],
                "stderr_head": output["stderr"].splitlines()[:10],
                "correctness": "failed",
                "command": output["command"],
            }
        samples.append(output)
    stdout = last_output["stdout"] if last_output else None
    correctness = "skipped"
    if expected_text is not None and stdout is not None:
        correctness = "pass" if stdout == expected_text else "fail"
    return {
        "status": "ok",
        "iterations": iterations,
        "warmups": warmups,
        "samples_ms": [sample["elapsed_ms"] for sample in samples],
        "median_ms": summarize_samples(samples, "elapsed_ms"),
        "max_rss_kb": summarize_samples(samples, "max_rss_kb"),
        "stdout": stdout,
        "correctness": correctness,
        "command": last_output["command"] if last_output else command_display(runtime_cmd),
    }


def tool_info(name: str) -> dict[str, Any]:
    path = shutil.which(name)
    return {
        "name": name,
        "available": path is not None,
        "path": path,
    }


def environment_info() -> dict[str, Any]:
    return {
        "platform": platform.platform(),
        "python": platform.python_version(),
        "machine": platform.machine(),
        "processor": platform.processor(),
        "kernel": platform.release(),
    }


def collect_results(args: argparse.Namespace) -> dict[str, Any]:
    preset = MODE_PRESETS[args.mode]
    compiler = resolve_compiler(args.arukellt)
    tools = {
        "wasmtime": tool_info("wasmtime"),
        "hyperfine": tool_info("hyperfine"),
        "time": tool_info("/usr/bin/time") if Path("/usr/bin/time").exists() else tool_info("time"),
    }
    time_bin = tools["time"]["path"]
    wasmtime_bin = tools["wasmtime"]["path"]

    with tempfile.TemporaryDirectory(prefix="arukellt-bench-") as tmp_dir:
        work_dir = Path(tmp_dir)
        benchmarks: list[dict[str, Any]] = []
        for case in BENCHMARKS:
            expected_text = maybe_read_expected(ROOT / case.expected)
            compile_result = measure_compile(
                compiler,
                case,
                target=args.target,
                iterations=args.compile_iterations or preset["compile_iterations"],
                time_bin=time_bin,
                work_dir=work_dir,
            )
            runtime_result: dict[str, Any]
            if compile_result["status"] == "ok":
                runtime_result = measure_runtime(
                    Path(compile_result["wasm_path"]),
                    expected_text,
                    iterations=args.runtime_iterations or preset["runtime_iterations"],
                    warmups=args.runtime_warmups if args.runtime_warmups is not None else preset["runtime_warmups"],
                    wasmtime_bin=wasmtime_bin,
                    time_bin=time_bin,
                    runtime_args=case.runtime_args,
                )
            else:
                runtime_result = {
                    "status": "skipped",
                    "reason": "compile failed",
                    "iterations": 0,
                    "warmups": 0,
                    "samples_ms": [],
                    "median_ms": None,
                    "max_rss_kb": None,
                    "stdout": None,
                    "correctness": "skipped",
                    "command": None,
                }
            benchmarks.append(
                {
                    "name": case.name,
                    "source": case.source,
                    "expected": case.expected,
                    "description": case.description,
                    "tags": list(case.tags),
                    "metrics": list(case.metrics),
                    "compile": {
                        key: value
                        for key, value in compile_result.items()
                        if key != "wasm_path"
                    },
                    "runtime": runtime_result,
                }
            )
    return {
        "schema_version": SCHEMA_VERSION,
        "generated_at": iso_now(),
        "mode": args.mode,
        "mode_description": preset["description"],
        "target": args.target,
        "thresholds": THRESHOLDS,
        "compiler": {
            "path": rel(compiler) if compiler.is_relative_to(ROOT) else str(compiler),
        },
        "environment": environment_info(),
        "tooling": tools,
        "benchmarks": benchmarks,
    }


def bench_by_name(data: dict[str, Any]) -> dict[str, dict[str, Any]]:
    if "benchmarks" in data:
        return {bench["name"]: bench for bench in data["benchmarks"]}
    converted: dict[str, dict[str, Any]] = {}
    for name, row in data.items():
        if not isinstance(row, dict):
            continue
        converted[name] = {
            "name": name,
            "compile": {
                "median_ms": row.get("compile_ms"),
                "binary_bytes": row.get("binary_bytes"),
            },
            "runtime": {
                "median_ms": row.get("run_ms"),
            },
        }
    return converted


def load_json(path: Path) -> dict[str, Any] | None:
    if not path.exists():
        return None
    return json.loads(path.read_text())


def compare_metric(current: Any, baseline: Any, threshold_pct: int) -> dict[str, Any]:
    if current is None or baseline is None:
        return {
            "status": "skipped",
            "delta_pct": None,
            "limit": None,
        }
    limit = baseline * (1 + threshold_pct / 100.0)
    delta_pct = ((current - baseline) / baseline * 100.0) if baseline else None
    return {
        "status": "pass" if current <= limit else "fail",
        "delta_pct": round(delta_pct, 2) if delta_pct is not None else None,
        "limit": round(limit, 3),
        "baseline": baseline,
        "current": current,
    }


def compare_results(current: dict[str, Any], baseline: dict[str, Any] | None) -> dict[str, Any]:
    if baseline is None:
        return {
            "baseline_available": False,
            "benchmarks": [],
            "totals": {"pass": 0, "fail": 0, "skipped": 0},
        }
    current_rows = bench_by_name(current)
    baseline_rows = bench_by_name(baseline)
    compared: list[dict[str, Any]] = []
    totals = {"pass": 0, "fail": 0, "skipped": 0}
    for name, row in current_rows.items():
        baseline_row = baseline_rows.get(name)
        if baseline_row is None:
            metric_rows = {
                "compile_ms": {"status": "skipped", "delta_pct": None, "limit": None},
                "run_ms": {"status": "skipped", "delta_pct": None, "limit": None},
                "binary_bytes": {"status": "skipped", "delta_pct": None, "limit": None},
            }
        else:
            metric_rows = {
                "compile_ms": compare_metric(
                    row.get("compile", {}).get("median_ms"),
                    baseline_row.get("compile", {}).get("median_ms"),
                    THRESHOLDS["compile_ms"],
                ),
                "run_ms": compare_metric(
                    row.get("runtime", {}).get("median_ms"),
                    baseline_row.get("runtime", {}).get("median_ms"),
                    THRESHOLDS["run_ms"],
                ),
                "binary_bytes": compare_metric(
                    row.get("compile", {}).get("binary_bytes"),
                    baseline_row.get("compile", {}).get("binary_bytes"),
                    THRESHOLDS["binary_bytes"],
                ),
            }
        for metric in metric_rows.values():
            totals[metric["status"]] += 1
        compared.append({"name": name, "metrics": metric_rows})
    return {
        "baseline_available": True,
        "baseline_schema_version": baseline.get("schema_version") if isinstance(baseline, dict) else None,
        "benchmarks": compared,
        "totals": totals,
    }


def format_ms(value: Any) -> str:
    return "n/a" if value is None else f"{value:.3f}"


def format_int(value: Any) -> str:
    return "n/a" if value is None else str(value)


def format_delta(metric: dict[str, Any]) -> str:
    if metric.get("delta_pct") is None:
        return "n/a"
    value = metric["delta_pct"]
    sign = "+" if value >= 0 else ""
    return f"{sign}{value:.2f}%"


def render_markdown(current: dict[str, Any], comparison: dict[str, Any], baseline_path: Path | None) -> str:
    lines: list[str] = []
    lines.append("# Benchmark Results")
    lines.append("")
    lines.append("Auto-generated by `scripts/benchmark_runner.py`.")
    lines.append("See [`benchmarks/README.md`](../../benchmarks/README.md) for the full workflow guide.")
    lines.append("")
    lines.append("## Quick Start")
    lines.append("")
    lines.append("```bash")
    lines.append("mise bench                  # full local benchmark (release build, all metrics)")
    lines.append("mise bench:quick            # single-sample smoke run")
    lines.append("mise bench:compare          # measure + compare against baseline")
    lines.append("mise bench:update-baseline  # replace baseline with current measurements")
    lines.append("mise bench:ci               # regression gate (fail on threshold breach)")
    lines.append("```")
    lines.append("")
    lines.append("## Current Run")
    lines.append("")
    lines.append(f"- Mode: `{current['mode']}`")
    lines.append(f"- Generated at: `{current['generated_at']}`")
    lines.append(f"- Target: `{current['target']}`")
    lines.append(f"- Compiler: `{current['compiler']['path']}`")
    lines.append("")
    lines.append("## Tool Availability")
    lines.append("")
    lines.append("| Tool | Status | Path |")
    lines.append("|------|--------|------|")
    for key in ("wasmtime", "hyperfine", "time"):
        tool = current["tooling"][key]
        status = "available" if tool["available"] else "missing"
        lines.append(f"| {tool['name']} | {status} | {tool['path'] or 'n/a'} |")
    lines.append("")
    lines.append("## Benchmark Matrix")
    lines.append("")
    lines.append("| Benchmark | Tags | Compile ms | Run ms | Binary bytes | Compile RSS KB | Run RSS KB | Correctness |")
    lines.append("|-----------|------|------------|--------|--------------|----------------|------------|-------------|")
    for bench in current["benchmarks"]:
        compile_row = bench["compile"]
        runtime_row = bench["runtime"]
        lines.append(
            "| {name} | {tags} | {compile_ms} | {run_ms} | {binary} | {compile_rss} | {run_rss} | {correctness} |".format(
                name=bench["name"],
                tags=", ".join(bench["tags"]),
                compile_ms=format_ms(compile_row.get("median_ms")),
                run_ms=format_ms(runtime_row.get("median_ms")),
                binary=format_int(compile_row.get("binary_bytes")),
                compile_rss=format_int(compile_row.get("max_rss_kb")),
                run_rss=format_int(runtime_row.get("max_rss_kb")),
                correctness=runtime_row.get("correctness", "skipped"),
            )
        )
    lines.append("")
    # Compile latency breakdown table
    phase_order = ["lex", "parse", "resolve", "typecheck", "lower", "opt", "emit", "total"]
    present_phases: list[str] = []
    for bench in current["benchmarks"]:
        pm = bench["compile"].get("phase_ms") or {}
        for ph in phase_order:
            if ph in pm and ph not in present_phases:
                present_phases.append(ph)
        for ph in pm:
            if ph not in phase_order and ph not in present_phases:
                present_phases.append(ph)
    if present_phases:
        lines.append("## Compile Latency Breakdown (ms)")
        lines.append("")
        header = "| Benchmark | " + " | ".join(present_phases) + " |"
        separator = "|-----------|" + "|".join("-" * (len(ph) + 2) for ph in present_phases) + "|"
        lines.append(header)
        lines.append(separator)
        for bench in current["benchmarks"]:
            pm = bench["compile"].get("phase_ms") or {}
            row = "| {} | {} |".format(
                bench["name"],
                " | ".join(format_ms(pm.get(ph)) for ph in present_phases),
            )
            lines.append(row)
        lines.append("")
    lines.append("## Threshold Policy")
    lines.append("")
    lines.append("| Metric | Allowed regression |")
    lines.append("|--------|--------------------|")
    lines.append(f"| Compile time | +{THRESHOLDS['compile_ms']}% |")
    lines.append(f"| Run time | +{THRESHOLDS['run_ms']}% |")
    lines.append(f"| Binary size | +{THRESHOLDS['binary_bytes']}% |")
    lines.append("")
    lines.append("## Baseline Comparison")
    lines.append("")
    if not comparison["baseline_available"]:
        lines.append(f"Baseline file not found: `{rel(baseline_path) if baseline_path else 'n/a'}`")
    else:
        lines.append(f"Baseline source: `{rel(baseline_path) if baseline_path else 'n/a'}`")
        lines.append("")
        lines.append("| Benchmark | Compile Δ | Run Δ | Size Δ | Status |")
        lines.append("|-----------|-----------|-------|--------|--------|")
        for bench in comparison["benchmarks"]:
            statuses = [metric["status"] for metric in bench["metrics"].values()]
            overall = "fail" if "fail" in statuses else "pass" if "pass" in statuses else "skipped"
            lines.append(
                f"| {bench['name']} | {format_delta(bench['metrics']['compile_ms'])} | {format_delta(bench['metrics']['run_ms'])} | {format_delta(bench['metrics']['binary_bytes'])} | {overall} |"
            )
        lines.append("")
        totals = comparison["totals"]
        lines.append(f"- Pass: {totals['pass']}")
        lines.append(f"- Fail: {totals['fail']}")
        lines.append(f"- Skipped: {totals['skipped']}")
    lines.append("")
    lines.append("## Schema")
    lines.append("")
    lines.append("Current benchmark JSON uses `schema_version = arukellt-bench-v1` and stores:")
    lines.append("- run metadata (`mode`, `target`, `environment`, tool availability)")
    lines.append("- per-benchmark `compile` metrics (`median_ms`, `binary_bytes`, `max_rss_kb`)")
    lines.append("- per-benchmark `runtime` metrics (`median_ms`, `max_rss_kb`, correctness)")
    lines.append("- workload tags for taxonomy and future grouped reporting")
    lines.append("")
    return "\n".join(lines) + "\n"


def render_text(current: dict[str, Any], comparison: dict[str, Any], baseline_path: Path | None) -> str:
    lines: list[str] = []
    lines.append("═══════════════════════════════════════════")
    lines.append(" Arukellt Benchmark Suite")
    lines.append("═══════════════════════════════════════════")
    lines.append(f"Mode     : {current['mode']} ({current['mode_description']})")
    lines.append(f"Target   : {current['target']}")
    lines.append(f"Compiler : {current['compiler']['path']}")
    lines.append(f"Wasmtime : {'yes' if current['tooling']['wasmtime']['available'] else 'no'}")
    hyperfine_msg = 'yes' if current['tooling']['hyperfine']['available'] else 'no (internal timer fallback)'
    lines.append(f"Hyperfine: {hyperfine_msg}")
    lines.append(f"RSS tool : {'yes' if current['tooling']['time']['available'] else 'no'}")
    lines.append("")
    lines.append("── Measurements ──────────────────────────")
    for bench in current["benchmarks"]:
        compile_row = bench["compile"]
        runtime_row = bench["runtime"]
        lines.append(f"{bench['name']} [{', '.join(bench['tags'])}]")
        if compile_row["status"] == "ok":
            lines.append(
                f"  compile : {format_ms(compile_row.get('median_ms'))} ms  size={format_int(compile_row.get('binary_bytes'))} B  rss={format_int(compile_row.get('max_rss_kb'))} KB"
            )
            phase_ms = compile_row.get("phase_ms")
            if phase_ms:
                phase_order = ["lex", "parse", "resolve", "typecheck", "lower", "opt", "emit", "total"]
                parts = []
                for ph in phase_order:
                    if ph in phase_ms:
                        parts.append(f"{ph}={format_ms(phase_ms[ph])}")
                for ph in phase_ms:
                    if ph not in phase_order:
                        parts.append(f"{ph}={format_ms(phase_ms[ph])}")
                lines.append(f"  phases  : {', '.join(parts)}")
        else:
            lines.append("  compile : ERROR")
            for line in compile_row.get("stderr_head", []):
                lines.append(f"    {line}")
        if runtime_row["status"] == "ok":
            lines.append(
                f"  runtime : {format_ms(runtime_row.get('median_ms'))} ms  rss={format_int(runtime_row.get('max_rss_kb'))} KB  correctness={runtime_row.get('correctness')}"
            )
        elif runtime_row["status"] == "skipped":
            lines.append(f"  runtime : SKIP ({runtime_row.get('reason', 'not available')})")
        else:
            lines.append("  runtime : ERROR")
            for line in runtime_row.get("stderr_head", []):
                lines.append(f"    {line}")
    if comparison["baseline_available"]:
        lines.append("")
        lines.append("── Baseline Comparison ───────────────────")
        lines.append(f"Baseline : {rel(baseline_path) if baseline_path else 'n/a'}")
        for bench in comparison["benchmarks"]:
            statuses = [metric["status"] for metric in bench["metrics"].values()]
            overall = "FAIL" if "fail" in statuses else "PASS" if "pass" in statuses else "SKIP"
            lines.append(
                f"{bench['name']}: compile {format_delta(bench['metrics']['compile_ms'])}, run {format_delta(bench['metrics']['run_ms'])}, size {format_delta(bench['metrics']['binary_bytes'])} => {overall}"
            )
        totals = comparison["totals"]
        lines.append(f"Summary  : pass={totals['pass']} fail={totals['fail']} skipped={totals['skipped']}")
    else:
        lines.append("")
        lines.append(f"Baseline : missing ({rel(baseline_path) if baseline_path else 'n/a'})")
    lines.append("")
    lines.append("Done.")
    return "\n".join(lines) + "\n"


def write_json(path: Path, payload: dict[str, Any]) -> None:
    ensure_parent(path)
    path.write_text(json.dumps(payload, indent=2) + "\n")


def failing_comparison(comparison: dict[str, Any]) -> bool:
    return any(metric["status"] == "fail" for bench in comparison["benchmarks"] for metric in bench["metrics"].values())


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Arukellt benchmark runner")
    parser.add_argument("--mode", choices=sorted(MODE_PRESETS), default="full")
    parser.add_argument("--arukellt", help="Path to arukellt binary")
    parser.add_argument("--target", default=DEFAULT_TARGET)
    parser.add_argument("--baseline", default=str(DEFAULT_BASELINE_RESULTS))
    parser.add_argument("--output-json", default=str(DEFAULT_CURRENT_RESULTS))
    parser.add_argument("--output-md", default=str(DEFAULT_RESULTS_MARKDOWN))
    parser.add_argument("--compile-iterations", type=int)
    parser.add_argument("--runtime-iterations", type=int)
    parser.add_argument("--runtime-warmups", type=int)
    parser.add_argument("--no-write-markdown", action="store_true")
    parser.add_argument("--no-write-json", action="store_true")
    parser.add_argument("--fail-on-regression", action="store_true")
    parser.add_argument("--print-json", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    current = collect_results(args)
    baseline_path = Path(args.baseline)
    baseline = load_json(baseline_path)
    comparison = compare_results(current, baseline)

    output_json = Path(args.output_json)
    output_md = Path(args.output_md)

    if not args.no_write_json:
        write_json(output_json, current)
    if not args.no_write_markdown:
        output_md.write_text(render_markdown(current, comparison, baseline_path))

    text = render_text(current, comparison, baseline_path)
    sys.stdout.write(text)

    if args.mode == "update-baseline":
        write_json(baseline_path, current)
        sys.stdout.write(f"Updated baseline: {rel(baseline_path)}\n")
    elif args.print_json:
        sys.stdout.write(json.dumps(current, indent=2) + "\n")

    should_fail = args.fail_on_regression or args.mode == "ci"
    if should_fail and not comparison["baseline_available"]:
        sys.stderr.write(f"baseline file not found: {baseline_path}\n")
        raise SystemExit(1)
    if should_fail and failing_comparison(comparison):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
