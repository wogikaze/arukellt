#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
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
# Preserved across `render_markdown` regenerations when `compare-benchmarks.sh` embeds a table.
CROSS_LANG_COMPARE_START = "<!-- arukellt:cross-lang-compare:start -->"
CROSS_LANG_COMPARE_END = "<!-- arukellt:cross-lang-compare:end -->"
THRESHOLDS = {
    "compile_ms": 20,
    "run_ms": 10,
    "binary_bytes": 15,
}

# Variance thresholds
CV_THRESHOLD = 5.0   # percent — benchmarks with CV > this are flagged as unstable
REPRO_THRESHOLD = 10  # percent — reproducibility check: two sequential runs must agree within this

# History / trend constants
RESULTS_DIR = ROOT / "benchmarks" / "results"
HISTORY_N = 5  # moving median window and trend depth

WASM_SECTION_ID_NAMES: dict[int, str] = {
    0: "custom",
    1: "type",
    2: "import",
    3: "function",
    4: "table",
    5: "memory",
    6: "global",
    7: "export",
    8: "start",
    9: "element",
    10: "code",
    11: "data",
    12: "datacount",
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
    BenchmarkCase(
        name="enum_dispatch",
        source="benchmarks/bench_cpu_enum_dispatch.ark",
        expected="benchmarks/bench_cpu_enum_dispatch.expected",
        description="Enum variant dispatch via match (100,000 iterations, 3 variants)",
        tags=("cpu-bound", "match-heavy", "allocation-heavy", "iteration"),
    ),
)

MODE_PRESETS: dict[str, dict[str, Any]] = {
    "quick": {
        "compile_iterations": 1,
        "runtime_iterations": 1,
        "runtime_warmups": 0,
        "runtime_latency_iterations": 5,
        "description": "single-sample local smoke benchmark",
    },
    "full": {
        "compile_iterations": 5,
        "runtime_iterations": 5,
        "runtime_warmups": 1,
        "runtime_latency_iterations": 20,
        "description": "default local benchmark profile",
    },
    "compare": {
        "compile_iterations": 5,
        "runtime_iterations": 5,
        "runtime_warmups": 1,
        "runtime_latency_iterations": 20,
        "description": "measure current results and compare with baseline",
    },
    "ci": {
        "compile_iterations": 5,
        "runtime_iterations": 5,
        "runtime_warmups": 1,
        "runtime_latency_iterations": 20,
        "description": "compare current results with baseline and fail on regression",
    },
    "update-baseline": {
        "compile_iterations": 5,
        "runtime_iterations": 5,
        "runtime_warmups": 1,
        "runtime_latency_iterations": 20,
        "description": "measure current results and replace baseline",
    },
    "reproducibility": {
        "compile_iterations": 5,
        "runtime_iterations": 5,
        "runtime_warmups": 1,
        "runtime_latency_iterations": 20,
        "description": "run benchmarks twice sequentially and compare for reproducibility",
    },
    "scaling": {
        "compile_iterations": 3,
        "runtime_iterations": 1,
        "runtime_warmups": 0,
        "runtime_latency_iterations": 5,
        "description": "measure compile/runtime latency at 3 input-size points (10%, 50%, 100%) to detect scaling cliffs",
    },
    "trend": {
        "compile_iterations": 1,
        "runtime_iterations": 1,
        "runtime_warmups": 0,
        "runtime_latency_iterations": 5,
        "description": "show trend report from historical results (no new benchmark run)",
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


def _read_uleb128(data: bytes, offset: int) -> tuple[int, int]:
    """Read an unsigned LEB128 integer from *data* starting at *offset*.

    Returns ``(value, new_offset)`` where *new_offset* points to the first
    byte after the encoded integer.  Terminates on the first byte whose
    high bit is clear.
    """
    result = 0
    shift = 0
    while offset < len(data):
        byte = data[offset]
        offset += 1
        result |= (byte & 0x7F) << shift
        if not (byte & 0x80):
            break
        shift += 7
    return result, offset


def parse_wasm_sections(wasm_bytes: bytes) -> dict[str, int]:
    """Parse a WebAssembly binary and return a mapping of section-name → payload size (bytes).

    Implemented with Python built-in ``bytes`` indexing and LEB128 decoding —
    no external tools (``wasm-objdump`` etc.) are required.

    The size reported for each section is the *payload* size from the LEB128
    length field (i.e. the content bytes, not including the 1-byte section-id
    or the variable-length size prefix itself).

    Custom sections (id == 0) are aggregated under ``"custom_total"`` because
    they may appear multiple times (e.g. name section, producers section).

    Returns an empty dict if the Wasm magic / version header is absent.
    """
    WASM_MAGIC = b"\x00asm\x01\x00\x00\x00"
    if len(wasm_bytes) < 8 or wasm_bytes[:8] != WASM_MAGIC:
        return {}
    offset = 8
    sections: dict[str, int] = {}
    while offset < len(wasm_bytes):
        if offset >= len(wasm_bytes):
            break
        section_id = wasm_bytes[offset]
        offset += 1
        if offset >= len(wasm_bytes):
            break
        section_size, offset = _read_uleb128(wasm_bytes, offset)
        section_name = WASM_SECTION_ID_NAMES.get(section_id)
        if section_name == "custom":
            sections["custom_total"] = sections.get("custom_total", 0) + section_size
        elif section_name:
            sections[section_name] = sections.get(section_name, 0) + section_size
        offset += section_size
    return sections


def measure_wasm_size_attribution(wasm_path: Path) -> dict[str, Any]:
    """Return section-level Wasm size attribution for *wasm_path*.

    Parses the Wasm binary format directly using pure Python — no external
    tools are required.  If ``wasm-objdump`` (part of ``wabt``) is available,
    it can be used for deeper symbol inspection; install wabt and run::

        wasm-objdump -x <file>.wasm

    Function-level symbol attribution requires a ``name`` custom section.
    The standard Arukellt Wasm emitter does not emit name sections, so
    ``symbol_attribution`` is always ``"unavailable"``.

    Returns a dict with integer section byte counts plus ``symbol_attribution``.
    If the file cannot be parsed, includes an ``"error"`` key instead.
    """
    if not wasm_path.exists():
        return {"error": "wasm file not found", "symbol_attribution": "unavailable"}
    try:
        wasm_bytes = wasm_path.read_bytes()
    except OSError as exc:
        return {"error": str(exc), "symbol_attribution": "unavailable"}
    sections = parse_wasm_sections(wasm_bytes)
    if not sections:
        return {"error": "not a valid wasm binary", "symbol_attribution": "unavailable"}
    result: dict[str, Any] = dict(sections)
    result["symbol_attribution"] = "unavailable"
    return result


def compute_percentiles(samples_ms: list[float]) -> dict[str, float | None]:
    """Compute p50/p95/p99, stddev, and CV from timing samples in milliseconds.

    Returns a dict with p50_ms, p95_ms, p99_ms, stddev_ms, cv_pct (all in ms or
    percent, or None when there are insufficient samples).

    cv_pct (coefficient of variation) = stddev / mean * 100.  When cv_pct exceeds
    CV_THRESHOLD the benchmark should be treated as unstable.
    """
    n = len(samples_ms)
    if n == 0:
        return {"p50_ms": None, "p95_ms": None, "p99_ms": None, "stddev_ms": None, "cv_pct": None}
    sorted_s = sorted(samples_ms)

    def _percentile(p: float) -> float:
        idx = (n - 1) * p / 100.0
        lo = int(idx)
        hi = min(lo + 1, n - 1)
        frac = idx - lo
        return round(sorted_s[lo] * (1.0 - frac) + sorted_s[hi] * frac, 3)

    stdev_val = round(statistics.stdev(samples_ms), 3) if n >= 2 else 0.0
    mean_val = statistics.mean(samples_ms)
    cv_pct = round(stdev_val / mean_val * 100.0, 2) if mean_val > 0 else None
    return {
        "p50_ms": _percentile(50.0),
        "p95_ms": _percentile(95.0),
        "p99_ms": _percentile(99.0),
        "stddev_ms": stdev_val,
        "cv_pct": cv_pct,
    }


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
    wasm_sections: dict[str, Any] | None = (
        measure_wasm_size_attribution(wasm_path) if wasm_path.exists() else None
    )
    compile_samples_ms = [sample["elapsed_ms"] for sample in samples]
    compile_percs = compute_percentiles(compile_samples_ms)
    compile_cv_pct = compile_percs.get("cv_pct")
    result: dict[str, Any] = {
        "status": "ok",
        "iterations": iterations,
        "samples_ms": compile_samples_ms,
        "median_ms": summarize_samples(samples, "elapsed_ms"),
        "stddev_ms": compile_percs.get("stddev_ms"),
        "cv_pct": compile_cv_pct,
        "variance_unstable": compile_cv_pct is not None and compile_cv_pct > CV_THRESHOLD,
        "max_rss_kb": summarize_samples(samples, "max_rss_kb"),
        "binary_bytes": binary_bytes,
        "command": last_output["command"] if last_output else None,
        "wasm_path": str(wasm_path),
    }
    if phase_ms is not None:
        result["phase_ms"] = phase_ms
    if wasm_sections is not None:
        result["wasm_sections"] = wasm_sections
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
    raw_ms = [sample["elapsed_ms"] for sample in samples]
    percs = compute_percentiles(raw_ms)
    cv_pct = percs.get("cv_pct")
    return {
        "status": "ok",
        "iterations": iterations,
        "warmups": warmups,
        "samples_ms": raw_ms,
        "median_ms": summarize_samples(samples, "elapsed_ms"),
        "p50_ms": percs["p50_ms"],
        "p95_ms": percs["p95_ms"],
        "p99_ms": percs["p99_ms"],
        "stddev_ms": percs["stddev_ms"],
        "cv_pct": cv_pct,
        "variance_unstable": cv_pct is not None and cv_pct > CV_THRESHOLD,
        "max_rss_kb": summarize_samples(samples, "max_rss_kb"),
        "stdout": stdout,
        "correctness": correctness,
        "command": last_output["command"] if last_output else command_display(runtime_cmd),
    }


def measure_memory(
    compile_result: dict[str, Any],
    runtime_result: dict[str, Any],
) -> dict[str, Any]:
    """Consolidate memory/GC telemetry for a benchmark.

    Compiler and runtime peak RSS are sourced from the already-collected
    compile and runtime measurement dicts.

    GC pause telemetry is not exposed by the wasmtime CLI; those fields are
    explicitly recorded as "unavailable".  Live-set / allocation-rate metrics
    likewise require runtime instrumentation that is not available via the
    standard wasmtime CLI, so only RSS peak is collected.

    STOP-IF note (issue #143):
      wasmtime does not expose GC pause stats → all gc_pause_* = "unavailable".
      This is intentional and must remain documented here.
    """
    return {
        "compiler_rss_peak_kb": compile_result.get("max_rss_kb"),
        "runtime_rss_peak_kb": runtime_result.get("max_rss_kb"),
        "gc_pause_total_ms": "unavailable",
        "gc_pause_max_ms": "unavailable",
        "gc_pause_count": "unavailable",
        "gc_note": (
            "wasmtime CLI does not expose GC pause telemetry; "
            "collect via --profile or future WASM GC instrumentation"
        ),
        "alloc_note": (
            "live-set and allocation-rate metrics require runtime instrumentation "
            "not available via standard wasmtime CLI; only RSS peak is collected"
        ),
    }


def measure_startup_probe(
    compiler: Path,
    *,
    target: str,
    latency_iterations: int,
    wasmtime_bin: str | None,
    time_bin: str | None,
    work_dir: Path,
) -> float | None:
    """Compile and time the startup fixture (benchmarks/startup.ark) to measure
    wasmtime instantiation + process overhead separately from guest execution.

    Returns median wall-clock time in ms, or None if unavailable.
    """
    startup_source = ROOT / "benchmarks" / "startup.ark"
    if not startup_source.exists() or wasmtime_bin is None:
        return None
    wasm_path = work_dir / "startup-probe.wasm"
    compile_out = run_measured(
        [str(compiler), "compile", "--target", target, "-o", str(wasm_path), str(startup_source)],
        cwd=ROOT,
        time_bin=None,
    )
    if compile_out["returncode"] != 0 or not wasm_path.exists():
        return None
    runtime_cmd = [wasmtime_bin, str(wasm_path)]
    probe_samples: list[float] = []
    for _ in range(latency_iterations):
        out = run_measured(runtime_cmd, cwd=ROOT, time_bin=time_bin)
        if out["returncode"] == 0:
            probe_samples.append(out["elapsed_ms"])
    if not probe_samples:
        return None
    return round(float(statistics.median(probe_samples)), 3)


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
        "wasm-objdump": tool_info("wasm-objdump"),
    }
    time_bin = tools["time"]["path"]
    wasmtime_bin = tools["wasmtime"]["path"]
    latency_iters = (
        args.runtime_latency_iterations
        if args.runtime_latency_iterations is not None
        else preset["runtime_latency_iterations"]
    )

    with tempfile.TemporaryDirectory(prefix="arukellt-bench-") as tmp_dir:
        work_dir = Path(tmp_dir)
        # Measure the startup probe once for the whole run; used to separate
        # wasmtime instantiation overhead from guest execution time.
        startup_ms = measure_startup_probe(
            compiler,
            target=args.target,
            latency_iterations=latency_iters,
            wasmtime_bin=wasmtime_bin,
            time_bin=time_bin,
            work_dir=work_dir,
        )
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
                    iterations=latency_iters,
                    warmups=args.runtime_warmups if args.runtime_warmups is not None else preset["runtime_warmups"],
                    wasmtime_bin=wasmtime_bin,
                    time_bin=time_bin,
                    runtime_args=case.runtime_args,
                )
                # Annotate with startup overhead and derived guest execution time.
                if startup_ms is not None and runtime_result["status"] == "ok":
                    median = runtime_result.get("median_ms")
                    runtime_result["startup_ms"] = startup_ms
                    if median is not None:
                        runtime_result["guest_ms"] = round(max(median - startup_ms, 0.0), 3)
            else:
                runtime_result = {
                    "status": "skipped",
                    "reason": "compile failed",
                    "iterations": 0,
                    "warmups": 0,
                    "samples_ms": [],
                    "median_ms": None,
                    "p50_ms": None,
                    "p95_ms": None,
                    "p99_ms": None,
                    "stddev_ms": None,
                    "max_rss_kb": None,
                    "stdout": None,
                    "correctness": "skipped",
                    "command": None,
                }
            memory_result = measure_memory(compile_result, runtime_result)
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
                    "memory": memory_result,
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
        # Wasm section-level diff (only when both sides have wasm_sections data)
        section_diff: dict[str, Any] = {}
        if baseline_row is not None:
            current_sections = row.get("compile", {}).get("wasm_sections") or {}
            baseline_sections = baseline_row.get("compile", {}).get("wasm_sections") or {}
            all_keys = set(current_sections.keys()) | set(baseline_sections.keys())
            for key in sorted(all_keys):
                if key in ("symbol_attribution", "error"):
                    continue
                c_val = current_sections.get(key)
                b_val = baseline_sections.get(key)
                if isinstance(c_val, int) and isinstance(b_val, int):
                    delta = c_val - b_val
                    delta_pct = round(delta / b_val * 100.0, 2) if b_val else None
                    section_diff[key] = {
                        "current": c_val,
                        "baseline": b_val,
                        "delta": delta,
                        "delta_pct": delta_pct,
                    }
                elif isinstance(c_val, int):
                    section_diff[key] = {"current": c_val, "baseline": None, "delta": None, "delta_pct": None}
                elif isinstance(b_val, int):
                    section_diff[key] = {"current": None, "baseline": b_val, "delta": None, "delta_pct": None}
        compared.append({"name": name, "metrics": metric_rows, "wasm_section_diff": section_diff})
    return {
        "baseline_available": True,
        "baseline_schema_version": baseline.get("schema_version") if isinstance(baseline, dict) else None,
        "benchmarks": compared,
        "totals": totals,
    }


def compare_reproducibility(run1: dict[str, Any], run2: dict[str, Any]) -> dict[str, Any]:
    """Compare two sequential benchmark runs for reproducibility.

    Uses REPRO_THRESHOLD (default 10%) to flag benchmarks where the two runs
    deviate by more than the allowed amount.  Binary size deviations are compared
    at the same threshold.
    """
    rows1 = bench_by_name(run1)
    rows2 = bench_by_name(run2)
    compared: list[dict[str, Any]] = []
    totals = {"pass": 0, "fail": 0, "skipped": 0}
    for name, r1 in rows1.items():
        r2 = rows2.get(name)
        if r2 is None:
            compared.append({"name": name, "reproducible": None, "metrics": {}})
            totals["skipped"] += 3
            continue
        metrics = {
            "compile_ms": compare_metric(
                r2.get("compile", {}).get("median_ms"),
                r1.get("compile", {}).get("median_ms"),
                REPRO_THRESHOLD,
            ),
            "run_ms": compare_metric(
                r2.get("runtime", {}).get("median_ms"),
                r1.get("runtime", {}).get("median_ms"),
                REPRO_THRESHOLD,
            ),
            "binary_bytes": compare_metric(
                r2.get("compile", {}).get("binary_bytes"),
                r1.get("compile", {}).get("binary_bytes"),
                REPRO_THRESHOLD,
            ),
        }
        for metric in metrics.values():
            totals[metric["status"]] += 1
        reproducible = all(m["status"] != "fail" for m in metrics.values())
        compared.append({"name": name, "reproducible": reproducible, "metrics": metrics})
    return {
        "schema_version": SCHEMA_VERSION,
        "mode": "reproducibility",
        "run1_generated_at": run1.get("generated_at"),
        "run2_generated_at": run2.get("generated_at"),
        "threshold_pct": REPRO_THRESHOLD,
        "benchmarks": compared,
        "totals": totals,
        "reproducible": totals["fail"] == 0,
    }


def render_reproducibility_text(report: dict[str, Any]) -> str:
    """Render a human-readable text report for a reproducibility run."""
    lines: list[str] = []
    lines.append("═══════════════════════════════════════════")
    lines.append(" Arukellt Benchmark Reproducibility Report")
    lines.append("═══════════════════════════════════════════")
    lines.append(f"Run 1 : {report.get('run1_generated_at', 'n/a')}")
    lines.append(f"Run 2 : {report.get('run2_generated_at', 'n/a')}")
    lines.append(f"Threshold : {report['threshold_pct']}%")
    lines.append("")
    lines.append("── Results ───────────────────────────────")
    for bench in report["benchmarks"]:
        flag = "REPRODUCIBLE" if bench.get("reproducible") else "UNSTABLE"
        if bench.get("reproducible") is None:
            flag = "SKIP"
        m = bench.get("metrics", {})
        lines.append(
            f"{bench['name']}: {flag}"
            f"  compile {format_delta(m.get('compile_ms', {}))},"
            f"  run {format_delta(m.get('run_ms', {}))},"
            f"  size {format_delta(m.get('binary_bytes', {}))}"
        )
    totals = report["totals"]
    lines.append("")
    overall = "PASS — all benchmarks reproducible within threshold" if report["reproducible"] else "FAIL — some benchmarks exceeded reproducibility threshold"
    lines.append(f"Overall  : {overall}")
    lines.append(f"Summary  : pass={totals['pass']} fail={totals['fail']} skipped={totals['skipped']}")
    lines.append("")
    lines.append("Done.")
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# Scaling-curve infrastructure
# ---------------------------------------------------------------------------

SCALING_INPUT_PATH = ROOT / "benchmarks" / "bench_parse_tree_distance.input.txt"
SCALING_SUBJECT = "bench_parse_tree_distance"
# Three input-size fractions: 10 %, 50 %, 100 % of full input.
SCALING_FRACTIONS = [0.10, 0.50, 1.00]


def _generate_scaling_subset(original_lines: list[str], frac: float) -> str:
    """Return a scaled-down version of the distance-matrix input file.

    The first line contains n (number of nodes).  Each subsequent line i holds
    the upper-triangle row: d[i][i+1], …, d[i][n-1].  For a subset of
    ``m = round(n * frac)`` nodes we take the first m-1 rows and truncate
    each row to its first ``m - i - 1`` values.
    """
    n_orig = int(original_lines[0].strip())
    n_sub = max(2, round(n_orig * frac))
    out_lines = [f"{n_sub}\n"]
    for row in range(n_sub - 1):  # rows 0 … n_sub-2
        values = original_lines[1 + row].split()
        take = n_sub - row - 1
        out_lines.append(" ".join(values[:take]) + "\n")
    return "".join(out_lines)


def _estimate_scaling_class(sizes: list[int], latencies: list[float | None]) -> str:
    """Estimate the asymptotic scaling class from (size, latency) pairs.

    Uses the log-log slope between the first and last valid measurement points.
    Slope thresholds (approximate):
      < 0.5   → O(1) or sub-linear
      0.5-1.3 → O(n)
      1.3-1.7 → O(n log n)
      1.7-2.3 → O(n²)
      > 2.3   → super-quadratic
    """
    valid = [(s, t) for s, t in zip(sizes, latencies) if t is not None and t > 0]
    if len(valid) < 2:
        return "unknown (insufficient data)"
    s0, t0 = valid[0]
    s1, t1 = valid[-1]
    if s0 <= 0 or s1 <= s0 or t0 <= 0:
        return "unknown"
    slope = math.log(t1 / t0) / math.log(s1 / s0)
    if slope < 0.5:
        return "O(1) or sub-linear"
    if slope < 1.3:
        return "O(n)"
    if slope < 1.7:
        return "O(n log n)"
    if slope < 2.3:
        return "O(n\u00b2)"  # O(n²)
    return f"super-quadratic (slope\u2248{slope:.2f})"


def _detect_scaling_cliffs(
    points: list[dict[str, Any]],
) -> list[str]:
    """Return warning strings for any adjacent-pair runtime cliff.

    A cliff is flagged when the time ratio exceeds 1.5 × the expected ratio
    under the estimated scaling class (O(n²) expected: size_ratio²; O(n):
    size_ratio).  We use size_ratio² as a conservative threshold so that a
    true O(n) benchmark does not trigger a false positive.
    """
    warnings: list[str] = []
    for i in range(1, len(points)):
        prev, curr = points[i - 1], points[i]
        t_prev = prev.get("run_ms")
        t_curr = curr.get("run_ms")
        if not t_prev or not t_curr or t_prev <= 0:
            continue
        size_ratio = curr["input_size"] / prev["input_size"]
        time_ratio = t_curr / t_prev
        # Flag if growth is faster than O(n²) by a 1.5× margin
        if time_ratio > (size_ratio ** 2) * 1.5:
            warnings.append(
                f"Cliff at n={prev['input_size']}→{curr['input_size']}: "
                f"size\u00d7{size_ratio:.1f} but time\u00d7{time_ratio:.2f}"
            )
    return warnings


def measure_scaling(
    compiler: Path,
    *,
    target: str,
    wasmtime_bin: str | None,
    time_bin: str | None,
    work_dir: Path,
) -> dict[str, Any]:
    """Measure compile/runtime latency at SCALING_FRACTIONS of the full input.

    Compiles the subject benchmark once (the WASM is independent of the input
    data).  For each input-size fraction the benchmark input file is
    temporarily replaced with a subset; the original file is always restored
    in a ``finally`` block.
    """
    if not SCALING_INPUT_PATH.exists():
        raise FileNotFoundError(f"Scaling input not found: {SCALING_INPUT_PATH}")

    original_lines = SCALING_INPUT_PATH.read_text().splitlines(keepends=True)
    n_orig = int(original_lines[0].strip())
    sizes = [max(2, round(n_orig * f)) for f in SCALING_FRACTIONS]

    case = next(c for c in BENCHMARKS if c.name == SCALING_SUBJECT)
    expected_text = maybe_read_expected(ROOT / case.expected)

    # Compile once — the source does not change between size points.
    compile_result = measure_compile(
        compiler,
        case,
        target=target,
        iterations=3,
        time_bin=time_bin,
        work_dir=work_dir,
    )

    points: list[dict[str, Any]] = []
    original_content = SCALING_INPUT_PATH.read_text()

    try:
        for size, frac in zip(sizes, SCALING_FRACTIONS):
            subset_content = _generate_scaling_subset(original_lines, frac)
            SCALING_INPUT_PATH.write_text(subset_content)

            if compile_result["status"] == "ok":
                runtime_result = measure_runtime(
                    Path(compile_result["wasm_path"]),
                    expected_text=None,  # output differs per subset; skip correctness check
                    iterations=5,
                    warmups=1,
                    wasmtime_bin=wasmtime_bin,
                    time_bin=time_bin,
                    runtime_args=case.runtime_args,
                )
            else:
                runtime_result = {"status": "compile_error", "median_ms": None}

            points.append(
                {
                    "input_size": size,
                    "fraction": round(frac, 2),
                    "compile_ms": compile_result.get("median_ms"),
                    "binary_bytes": compile_result.get("binary_bytes"),
                    "run_ms": runtime_result.get("median_ms"),
                    "run_status": runtime_result.get("status", "unknown"),
                }
            )
    finally:
        SCALING_INPUT_PATH.write_text(original_content)

    run_sizes = [p["input_size"] for p in points]
    run_latencies = [p.get("run_ms") for p in points]
    scaling_class = _estimate_scaling_class(run_sizes, run_latencies)
    cliff_warnings = _detect_scaling_cliffs(points)

    return {
        "subject": SCALING_SUBJECT,
        "compile_status": compile_result["status"],
        "compile_ms": compile_result.get("median_ms"),
        "binary_bytes": compile_result.get("binary_bytes"),
        "sizes": sizes,
        "fractions": [round(f, 2) for f in SCALING_FRACTIONS],
        "scaling_class": {
            "run": scaling_class,
            "compile": "O(1) — compile time is independent of input data",
        },
        "cliff_warnings": cliff_warnings,
        "points": points,
    }


def render_scaling_text(report: dict[str, Any]) -> str:
    """Render a human-readable text report for a scaling-curve run."""
    lines: list[str] = []
    lines.append("\u2550" * 45)
    lines.append(" Arukellt Benchmark Scaling Curve Report")
    lines.append("\u2550" * 45)
    lines.append(f"Subject  : {report['subject']}")
    lines.append(f"Generated: {report.get('generated_at', 'n/a')}")
    lines.append(f"Compile  : {report.get('compile_status', 'n/a')}"
                 f"  ({format_ms(report.get('compile_ms'))} ms median)")
    lines.append("")
    lines.append("\u2500\u2500 Input-size vs Latency " + "\u2500" * 22)
    header = f"  {'n':>8}  {'compile_ms':>12}  {'run_ms':>10}  {'binary_bytes':>14}"
    lines.append(header)
    lines.append("  " + "\u2500" * (len(header) - 2))
    for pt in report["points"]:
        lines.append(
            f"  {pt['input_size']:>8d}"
            f"  {format_ms(pt.get('compile_ms')):>12}"
            f"  {format_ms(pt.get('run_ms')):>10}"
            f"  {format_int(pt.get('binary_bytes')):>14}"
        )
    lines.append("")
    lines.append("\u2500\u2500 Scaling Classification " + "\u2500" * 21)
    sc = report.get("scaling_class", {})
    lines.append(f"  runtime : {sc.get('run', 'unknown')}")
    lines.append(f"  compile : {sc.get('compile', 'unknown')}")
    lines.append("")
    lines.append("\u2500\u2500 Cliff Warnings " + "\u2500" * 28)
    warnings = report.get("cliff_warnings", [])
    if warnings:
        for w in warnings:
            lines.append(f"  WARNING: {w}")
    else:
        lines.append("  None detected.")
    lines.append("")
    lines.append("Done.")
    return "\n".join(lines) + "\n"


def _run_scaling(args: argparse.Namespace) -> None:
    """Execute a scaling-curve measurement and write a JSON + text report."""
    compiler = resolve_compiler(args.arukellt)
    tools = {
        "wasmtime": tool_info("wasmtime"),
        "time": tool_info("/usr/bin/time") if Path("/usr/bin/time").exists() else tool_info("time"),
    }
    sys.stdout.write("Running scaling curve measurement (3 input-size points)...\n")
    sys.stdout.flush()
    with tempfile.TemporaryDirectory(prefix="arukellt-bench-scaling-") as tmp_dir:
        report = measure_scaling(
            compiler,
            target=args.target,
            wasmtime_bin=tools["wasmtime"]["path"],
            time_bin=tools["time"]["path"],
            work_dir=Path(tmp_dir),
        )
    report["schema_version"] = SCHEMA_VERSION
    report["generated_at"] = iso_now()
    report["mode"] = "scaling"
    report["environment"] = environment_info()

    sys.stdout.write(render_scaling_text(report))

    scaling_json_path = Path(
        args.scaling_output_json
        if args.scaling_output_json
        else str(Path(args.output_json).parent / "scaling.json")
    )
    if not args.no_write_json:
        write_json(scaling_json_path, report)
        label = rel(scaling_json_path) if scaling_json_path.is_relative_to(ROOT) else str(scaling_json_path)
        sys.stdout.write(f"Scaling report written to: {label}\n")
    if args.print_json:
        sys.stdout.write(json.dumps(report, indent=2) + "\n")


def render_trend_report(
    history: list[dict[str, Any]],
    mode: str,
    target: str,
    n: int = HISTORY_N,
) -> str:
    """Render a human-readable time-series trend report from historical benchmark data.

    Shows per-benchmark time-series tables (compile_ms, run_ms, binary_bytes)
    and trend labels (improving / stable / degrading) for each metric.
    Includes a moving median summary at the end.
    """
    lines: list[str] = []
    lines.append("\u2550" * 50)
    lines.append(" Arukellt Benchmark Trend Report")
    lines.append("\u2550" * 50)

    if not history:
        lines.append(f"No history found for mode={mode}, target={target}.")
        lines.append("Run benchmarks first to build history:")
        lines.append("  python3 scripts/util/benchmark_runner.py --mode full")
        return "\n".join(lines) + "\n"

    window = history[:n]
    lines.append(f"History  : {len(window)} run(s)  mode={mode}  target={target}")
    lines.append(f"Directory: {rel(RESULTS_DIR)}")
    lines.append("")

    # Collect all benchmark names across runs
    bench_names: list[str] = []
    for run in window:
        for b in run.get("benchmarks", []):
            if b["name"] not in bench_names:
                bench_names.append(b["name"])
    bench_names.sort()

    METRIC_COLS = [
        ("compile_ms",   "compile_ms",    12),
        ("run_ms",       "run_ms",        10),
        ("binary_bytes", "binary_bytes",  14),
    ]

    for bname in bench_names:
        sep = "\u2500" * max(0, 44 - len(bname))
        lines.append(f"\u2500\u2500 {bname} {sep}")
        lines.append(
            f"  {'Timestamp':<22}"
            f"  {'compile_ms':>12}"
            f"  {'run_ms':>10}"
            f"  {'binary_bytes':>14}"
        )
        lines.append("  " + "\u2500" * 62)

        # Data rows, newest-first
        for run in window:
            ts = run.get("generated_at", "unknown")[:16]
            bmap = {b["name"]: b for b in run.get("benchmarks", [])}
            b = bmap.get(bname)
            if b is None:
                continue
            c_ms = _bench_metric_value(b, "compile_ms")
            r_ms = _bench_metric_value(b, "run_ms")
            bb = _bench_metric_value(b, "binary_bytes")
            lines.append(
                f"  {ts:<22}"
                f"  {format_ms(c_ms):>12}"
                f"  {format_ms(r_ms):>10}"
                f"  {format_int(bb):>14}"
            )

        # Trend labels (oldest-first computation)
        lines.append("")
        for metric_key, label_key, _width in METRIC_COLS:
            vals_oldest_first: list[float] = []
            for run in reversed(window):
                bmap = {b["name"]: b for b in run.get("benchmarks", [])}
                b = bmap.get(bname)
                if b is not None:
                    v = _bench_metric_value(b, metric_key)
                    if v is not None:
                        vals_oldest_first.append(v)
            label = _trend_label(vals_oldest_first)
            lines.append(f"  trend ({label_key}): {label}")
        lines.append("")

    # Moving median summary
    moving_med = compute_moving_median(history, n=n)
    if moving_med:
        lines.append(f"\u2500\u2500 Moving Median (last {min(len(history), n)} run(s)) " + "\u2500" * 18)
        lines.append(
            f"  {'Benchmark':<20}"
            f"  {'compile_ms':>12}"
            f"  {'run_ms':>10}"
            f"  {'binary_bytes':>14}"
        )
        lines.append("  " + "\u2500" * 62)
        for bname, meds in sorted(moving_med.items()):
            lines.append(
                f"  {bname:<20}"
                f"  {format_ms(meds.get('compile_ms')):>12}"
                f"  {format_ms(meds.get('run_ms')):>10}"
                f"  {format_int(meds.get('binary_bytes')):>14}"
            )
        lines.append("")

    lines.append("Done.")
    return "\n".join(lines) + "\n"


def _run_trend(args: argparse.Namespace) -> None:
    """Load benchmark history from benchmarks/results/ and print a trend report.

    Does NOT run any new benchmarks.  Scans mode prefixes in preference order
    (full → compare → quick) and uses the first non-empty history found.
    Writes a lightweight trend summary JSON to benchmarks/results/.
    """
    n = args.history_n
    target = args.target

    history: list[dict[str, Any]] = []
    mode_used = "full"
    for candidate_mode in ("full", "compare", "quick"):
        h = load_history(candidate_mode, target, limit=n)
        if h:
            history = h
            mode_used = candidate_mode
            break

    sys.stdout.write(render_trend_report(history, mode=mode_used, target=target, n=n))

    if not history:
        return

    if not args.no_write_json:
        trend_path = RESULTS_DIR / f"trend-{_target_short(target)}-latest.json"
        trend_data: dict[str, Any] = {
            "schema_version": SCHEMA_VERSION,
            "generated_at": iso_now(),
            "mode": "trend",
            "target": target,
            "history_depth": len(history),
            "history_mode": mode_used,
            "moving_median": compute_moving_median(history, n=n),
            "bench_trends": {},
        }
        bench_names_seen: list[str] = sorted({
            b["name"] for run in history for b in run.get("benchmarks", [])
        })
        for bname in bench_names_seen:
            btrends: dict[str, str] = {}
            for metric_key in ("compile_ms", "run_ms", "binary_bytes"):
                vals: list[float] = []
                for run in reversed(history[:n]):
                    bmap = {b["name"]: b for b in run.get("benchmarks", [])}
                    b = bmap.get(bname)
                    if b is not None:
                        v = _bench_metric_value(b, metric_key)
                        if v is not None:
                            vals.append(v)
                btrends[metric_key] = _trend_label(vals)
            trend_data["bench_trends"][bname] = btrends
        write_json(trend_path, trend_data)
        label = rel(trend_path) if trend_path.is_relative_to(ROOT) else str(trend_path)
        sys.stdout.write(f"Trend summary written to: {label}\n")


def variance_report_lines(current: dict[str, Any]) -> list[str]:
    """Return lines for a variance/CV section to embed in text or markdown reports."""
    lines: list[str] = []
    has_cv = any(
        bench.get("compile", {}).get("cv_pct") is not None
        or bench.get("runtime", {}).get("cv_pct") is not None
        for bench in current.get("benchmarks", [])
    )
    if not has_cv:
        return lines
    lines.append("── Variance / CV Report ─────────────────")
    for bench in current.get("benchmarks", []):
        c_cv = bench.get("compile", {}).get("cv_pct")
        r_cv = bench.get("runtime", {}).get("cv_pct")
        c_flag = f"  compile_cv={c_cv:.2f}%" if c_cv is not None else ""
        c_unstable = " [UNSTABLE]" if bench.get("compile", {}).get("variance_unstable") else ""
        r_flag = f"  runtime_cv={r_cv:.2f}%" if r_cv is not None else ""
        r_unstable = " [UNSTABLE]" if bench.get("runtime", {}).get("variance_unstable") else ""
        if c_cv is not None or r_cv is not None:
            lines.append(f"{bench['name']}:{c_flag}{c_unstable}{r_flag}{r_unstable}")
    return lines


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


def extract_preserved_cross_lang_markdown(existing: str) -> str | None:
    """Return inner Markdown between cross-lang markers, or None if absent."""
    if CROSS_LANG_COMPARE_START not in existing or CROSS_LANG_COMPARE_END not in existing:
        return None
    _, rest = existing.split(CROSS_LANG_COMPARE_START, 1)
    inner, _ = rest.split(CROSS_LANG_COMPARE_END, 1)
    return inner


def merge_preserved_cross_lang_markdown(template: str, preserved_inner: str | None) -> str:
    """Re-inject a preserved cross-lang block into freshly rendered Markdown."""
    if preserved_inner is None:
        return template
    if CROSS_LANG_COMPARE_START not in template or CROSS_LANG_COMPARE_END not in template:
        return template
    pre, rest = template.split(CROSS_LANG_COMPARE_START, 1)
    _, post = rest.split(CROSS_LANG_COMPARE_END, 1)
    body = preserved_inner if preserved_inner.endswith("\n") else preserved_inner + "\n"
    return pre + CROSS_LANG_COMPARE_START + body + CROSS_LANG_COMPARE_END + post


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
    lines.append("## Cross-language comparison")
    lines.append("")
    lines.append(
        "Native reference programs (C, Rust, Go) are built with each toolchain’s "
        "release-style flags, timed with `hyperfine` when available (otherwise a "
        "built-in shell timer), and compared to Ark wasm. "
        "`bash scripts/compare-benchmarks.sh` prints the table to **stdout** and "
        "embeds the same Markdown table in the block below (between HTML comments "
        "so `scripts/util/benchmark_runner.py` can regenerate the rest of this file "
        "without deleting it)."
    )
    lines.append("")
    lines.append("```bash")
    lines.append("bash scripts/compare-benchmarks.sh")
    lines.append("# equivalent:")
    lines.append("bash scripts/run/run-benchmarks.sh --compare-lang c,rust,go \\")
    lines.append("  --compare-write-md docs/process/benchmark-results.md --compare-c-ratio-gate")
    lines.append("```")
    lines.append("")
    lines.append(
        "Roadmap C-ratio targets (fib ≤1.5× vs C, vec_ops ≤2.0× vs C) are enforced "
        "when using `--compare-c-ratio-gate` (on by default via `compare-benchmarks.sh`). "
        "**Grain** (Wasm GC) is not in this runner yet — no `benchmarks/*.grain` "
        "sources and no `grain` CLI hook; see "
        "[`docs/process/roadmap-v4.md`](../process/roadmap-v4.md) and issue #112."
    )
    lines.append("")
    lines.append(CROSS_LANG_COMPARE_START)
    lines.append("")
    lines.append(
        "*No cross-language table embedded yet. Run `bash scripts/compare-benchmarks.sh` "
        "after a release compiler build to populate this section.*"
    )
    lines.append("")
    lines.append(CROSS_LANG_COMPARE_END)
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
    # Wasm Section Breakdown table
    key_sections = ["type", "import", "function", "code", "data", "export", "custom_total"]
    any_sections = any(
        bench["compile"].get("wasm_sections") for bench in current["benchmarks"]
    )
    if any_sections:
        present_sections = [s for s in key_sections if any(
            isinstance(bench["compile"].get("wasm_sections", {}).get(s), int)
            for bench in current["benchmarks"]
        )]
        if present_sections:
            lines.append("## Wasm Section Breakdown (bytes)")
            lines.append("")
            lines.append(
                "Section sizes parsed directly from the .wasm binary header using pure-Python "
                "LEB128 decoding (no external tools required). `symbol_attribution` is "
                "`unavailable` — the Arukellt emitter does not emit a name section so "
                "function-level attribution is not available; use `wasm-objdump` from "
                "`wabt` for deeper inspection."
            )
            lines.append("")
            header = "| Benchmark | " + " | ".join(present_sections) + " | symbol_attribution |"
            separator = "|-----------|"
            separator += "|".join("-" * (len(s) + 2) for s in present_sections)
            separator += "|-------------------|"
            lines.append(header)
            lines.append(separator)
            for bench in current["benchmarks"]:
                ws = bench["compile"].get("wasm_sections") or {}
                sym = ws.get("symbol_attribution", "unavailable")
                row = "| {} | {} | {} |".format(
                    bench["name"],
                    " | ".join(format_int(ws.get(s)) for s in present_sections),
                    sym,
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
    # Variance / CV table
    has_cv = any(
        bench.get("compile", {}).get("cv_pct") is not None
        or bench.get("runtime", {}).get("cv_pct") is not None
        for bench in current.get("benchmarks", [])
    )
    if has_cv:
        lines.append("## Variance / Coefficient of Variation")
        lines.append("")
        lines.append(
            f"Benchmarks with CV > {CV_THRESHOLD}% are flagged as **unstable** — "
            "high variance may indicate OS scheduling noise, thermal throttling, "
            "or a flaky workload. Run with more iterations or pin CPU frequency for reliable results."
        )
        lines.append("")
        lines.append("| Benchmark | Compile CV% | Compile Stable | Runtime CV% | Runtime Stable |")
        lines.append("|-----------|-------------|----------------|-------------|----------------|")
        for bench in current["benchmarks"]:
            c = bench.get("compile", {})
            r = bench.get("runtime", {})
            c_cv = c.get("cv_pct")
            r_cv = r.get("cv_pct")
            c_stable = "✗ unstable" if c.get("variance_unstable") else ("✓" if c_cv is not None else "n/a")
            r_stable = "✗ unstable" if r.get("variance_unstable") else ("✓" if r_cv is not None else "n/a")
            lines.append(
                f"| {bench['name']} | {f'{c_cv:.2f}' if c_cv is not None else 'n/a'} | {c_stable} "
                f"| {f'{r_cv:.2f}' if r_cv is not None else 'n/a'} | {r_stable} |"
            )
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
        # Section-level diff table in compare report
        benches_with_section_diff = [b for b in comparison["benchmarks"] if b.get("wasm_section_diff")]
        if benches_with_section_diff:
            all_sec_keys: list[str] = []
            for b in benches_with_section_diff:
                for k in b["wasm_section_diff"]:
                    if k not in all_sec_keys:
                        all_sec_keys.append(k)
            lines.append("### Wasm Section Δ vs Baseline (bytes)")
            lines.append("")
            lines.append("Positive delta = size grew. Negative = shrank. `n/a` = section absent in one side.")
            lines.append("")
            header = "| Benchmark | " + " | ".join(all_sec_keys) + " |"
            separator = "|-----------|"
            separator += "|".join("-" * (len(k) + 2) for k in all_sec_keys)
            separator += "|"
            lines.append(header)
            lines.append(separator)
            for bench in benches_with_section_diff:
                sd = bench["wasm_section_diff"]
                cells = []
                for k in all_sec_keys:
                    entry = sd.get(k)
                    if entry is None:
                        cells.append("n/a")
                    elif entry.get("delta_pct") is not None:
                        sign = "+" if entry["delta_pct"] >= 0 else ""
                        cells.append(f"{entry['delta']:+d} ({sign}{entry['delta_pct']:.1f}%)")
                    else:
                        cells.append(format_int(entry.get("current")))
                lines.append(f"| {bench['name']} | {' | '.join(cells)} |")
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
    lines.append("- per-benchmark `runtime` metrics (`median_ms`, `p50_ms`, `p95_ms`, `p99_ms`, `stddev_ms`, `startup_ms`, `guest_ms`, `max_rss_kb`, correctness)")
    lines.append("- per-benchmark `memory` metrics (`compiler_rss_peak_kb`, `runtime_rss_peak_kb`, `gc_pause_*` — GC fields are `\"unavailable\"` because wasmtime CLI does not expose GC pause telemetry)")
    lines.append("- workload tags for taxonomy and future grouped reporting")
    lines.append("")
    # Memory telemetry table
    lines.append("## Memory / GC Telemetry")
    lines.append("")
    lines.append("GC pause fields are `unavailable` — wasmtime CLI does not expose GC pause stats.")
    lines.append("RSS peaks are sourced from `/usr/bin/time -f %M` (null when the tool is absent).")
    lines.append("")
    lines.append("| Benchmark | Tags | Compiler RSS KB | Runtime RSS KB | GC pauses |")
    lines.append("|-----------|------|-----------------|----------------|-----------|")
    for bench in current["benchmarks"]:
        mem = bench.get("memory") or {}
        gc_field = mem.get("gc_pause_count", "unavailable")
        lines.append(
            "| {name} | {tags} | {crss} | {rrss} | {gc} |".format(
                name=bench["name"],
                tags=", ".join(bench["tags"]),
                crss=format_int(mem.get("compiler_rss_peak_kb")),
                rrss=format_int(mem.get("runtime_rss_peak_kb")),
                gc=gc_field,
            )
        )
    lines.append("")
    # Runtime latency breakdown table
    has_latency = any(
        bench["runtime"].get("p50_ms") is not None or bench["runtime"].get("startup_ms") is not None
        for bench in current["benchmarks"]
    )
    if has_latency:
        lines.append("## Runtime Latency Breakdown (ms)")
        lines.append("")
        lines.append("Startup latency is the median time to instantiate and exit the `startup.ark` no-op fixture.")
        lines.append("Guest execution time = median total − startup overhead.")
        lines.append("")
        lines.append("| Benchmark | startup | guest | p50 | p95 | p99 | stddev |")
        lines.append("|-----------|---------|-------|-----|-----|-----|--------|")
        for bench in current["benchmarks"]:
            rt = bench["runtime"]
            if rt.get("status") != "ok":
                continue
            lines.append(
                "| {name} | {startup} | {guest} | {p50} | {p95} | {p99} | {stddev} |".format(
                    name=bench["name"],
                    startup=format_ms(rt.get("startup_ms")),
                    guest=format_ms(rt.get("guest_ms")),
                    p50=format_ms(rt.get("p50_ms")),
                    p95=format_ms(rt.get("p95_ms")),
                    p99=format_ms(rt.get("p99_ms")),
                    stddev=format_ms(rt.get("stddev_ms")),
                )
            )
        lines.append("")
    return "\n".join(lines) + "\n"


def render_text(
    current: dict[str, Any],
    comparison: dict[str, Any],
    baseline_path: Path | None,
    trend_ctx: dict[str, Any] | None = None,
) -> str:
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
            wasm_sections = compile_row.get("wasm_sections")
            if wasm_sections and not wasm_sections.get("error"):
                sec_keys = ["type", "import", "function", "code", "data", "export"]
                sec_parts = [
                    f"{k}={wasm_sections[k]}"
                    for k in sec_keys
                    if isinstance(wasm_sections.get(k), int)
                ]
                if sec_parts:
                    lines.append(f"  sections: {', '.join(sec_parts)}")
        else:
            lines.append("  compile : ERROR")
            for line in compile_row.get("stderr_head", []):
                lines.append(f"    {line}")
        if runtime_row["status"] == "ok":
            lines.append(
                f"  runtime : {format_ms(runtime_row.get('median_ms'))} ms  rss={format_int(runtime_row.get('max_rss_kb'))} KB  correctness={runtime_row.get('correctness')}"
            )
            startup = runtime_row.get("startup_ms")
            guest = runtime_row.get("guest_ms")
            p50 = runtime_row.get("p50_ms")
            p95 = runtime_row.get("p95_ms")
            p99 = runtime_row.get("p99_ms")
            stddev = runtime_row.get("stddev_ms")
            if startup is not None or p50 is not None:
                latency_parts = []
                if startup is not None:
                    latency_parts.append(f"startup={format_ms(startup)}")
                if guest is not None:
                    latency_parts.append(f"guest={format_ms(guest)}")
                if p50 is not None:
                    latency_parts.append(f"p50={format_ms(p50)}")
                if p95 is not None:
                    latency_parts.append(f"p95={format_ms(p95)}")
                if p99 is not None:
                    latency_parts.append(f"p99={format_ms(p99)}")
                if stddev is not None:
                    latency_parts.append(f"stddev={format_ms(stddev)}")
                lines.append(f"  latency : {', '.join(latency_parts)}")
        elif runtime_row["status"] == "skipped":
            lines.append(f"  runtime : SKIP ({runtime_row.get('reason', 'not available')})")
        else:
            lines.append("  runtime : ERROR")
            for line in runtime_row.get("stderr_head", []):
                lines.append(f"    {line}")
        mem_row = bench.get("memory") or {}
        if mem_row:
            lines.append(
                f"  memory  : compiler_rss={format_int(mem_row.get('compiler_rss_peak_kb'))} KB"
                f"  runtime_rss={format_int(mem_row.get('runtime_rss_peak_kb'))} KB"
                f"  gc_pauses={mem_row.get('gc_pause_count', 'unavailable')}"
            )
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
            section_diff = bench.get("wasm_section_diff") or {}
            if section_diff:
                diff_parts = []
                for key, entry in sorted(section_diff.items()):
                    if entry.get("delta_pct") is not None:
                        sign = "+" if entry["delta_pct"] >= 0 else ""
                        diff_parts.append(f"{key}={entry['delta']:+d}({sign}{entry['delta_pct']:.1f}%)")
                if diff_parts:
                    lines.append(f"  sections: {', '.join(diff_parts)}")
        totals = comparison["totals"]
        lines.append(f"Summary  : pass={totals['pass']} fail={totals['fail']} skipped={totals['skipped']}")
    else:
        lines.append("")
        lines.append(f"Baseline : missing ({rel(baseline_path) if baseline_path else 'n/a'})")
    # Trend context (shown when history is available)
    if trend_ctx and trend_ctx.get("available"):
        lines.append("")
        lines.append("── Trend Context ─────────────────────────")
        depth = trend_ctx["history_depth"]
        lines.append(f"History  : {depth} prior run(s) used")
        moving_med = trend_ctx.get("moving_median", {})
        if moving_med:
            lines.append(f"Moving median (last {min(depth, HISTORY_N)} runs):")
            for bname, meds in moving_med.items():
                lines.append(
                    f"  {bname}: compile={format_ms(meds.get('compile_ms'))} ms"
                    f"  run={format_ms(meds.get('run_ms'))} ms"
                    f"  size={format_int(meds.get('binary_bytes'))} B"
                )
        bench_trends = trend_ctx.get("bench_trends", {})
        if bench_trends:
            lines.append("Trend labels (improving/stable/degrading):")
            for bname, trends in bench_trends.items():
                c_trend = trends.get("compile_ms", "stable")
                r_trend = trends.get("run_ms", "stable")
                lines.append(f"  {bname}: compile={c_trend}  run={r_trend}")
    # Variance / CV section
    var_lines = variance_report_lines(current)
    if var_lines:
        lines.append("")
        for vl in var_lines:
            lines.append(vl)
    lines.append("")
    lines.append("Done.")
    return "\n".join(lines) + "\n"


def write_json(path: Path, payload: dict[str, Any]) -> None:
    ensure_parent(path)
    path.write_text(json.dumps(payload, indent=2) + "\n")


# ── History / trend helpers ────────────────────────────────────────────────

def _target_short(target: str) -> str:
    """Extract short suffix from target: wasm32-wasi-p1 → p1."""
    return target.split("-")[-1] if "-" in target else target


def _result_filename(mode: str, target: str, ts_clean: str) -> str:
    short = _target_short(target)
    return f"bench-{mode}-{short}-{ts_clean}.json"


def save_to_history(payload: dict[str, Any]) -> Path:
    """Save a benchmark result to benchmarks/results/ with a timestamped filename.

    The filename encodes mode, target short-name, and the ISO 8601 timestamp
    from the payload so results sort correctly by name.  Returns the written path.
    """
    mode = payload.get("mode", "unknown")
    target = payload.get("target", "unknown")
    raw_ts = payload.get("generated_at", iso_now())
    # "2026-04-15T12:34:56.789Z" → "20260415T123456Z"
    ts_clean = raw_ts.replace("-", "").replace(":", "").split(".")[0] + "Z"
    filename = _result_filename(mode, target, ts_clean)
    dest = RESULTS_DIR / filename
    write_json(dest, payload)
    return dest


def load_history(mode: str, target: str, limit: int = HISTORY_N) -> list[dict[str, Any]]:
    """Load up to *limit* recent benchmark results from benchmarks/results/.

    Matches files by mode prefix and target field in the JSON payload.
    Returns results sorted newest-first.
    """
    if not RESULTS_DIR.exists():
        return []
    prefix = f"bench-{mode}-"
    candidates: list[tuple[str, dict[str, Any]]] = []
    for p in RESULTS_DIR.glob(f"{prefix}*.json"):
        data = load_json(p)
        if data is None:
            continue
        # Accept files whose stored target matches (or old files with no target)
        file_target = data.get("target", "")
        if file_target and file_target != target:
            continue
        ts_str = data.get("generated_at", "")
        candidates.append((ts_str, data))
    candidates.sort(key=lambda x: x[0], reverse=True)
    return [d for _, d in candidates[:limit]]


def _bench_metric_value(bench_data: dict[str, Any], metric_key: str) -> float | None:
    """Extract a scalar metric from a benchmark result entry."""
    if metric_key == "compile_ms":
        return bench_data.get("compile", {}).get("median_ms")
    if metric_key == "run_ms":
        return bench_data.get("runtime", {}).get("median_ms")
    if metric_key == "binary_bytes":
        return bench_data.get("compile", {}).get("binary_bytes")
    return None


def _trend_label(values: list[float]) -> str:
    """Return 'improving', 'degrading', or 'stable' over an oldest-first series.

    Requires at least 3 values; shorter series return 'stable'.
    A change of >5 % in the second half vs first half median triggers the label.
    """
    if len(values) < 3:
        return "stable"
    mid = max(1, len(values) // 2)
    first_half = statistics.median(values[:mid])
    second_half = statistics.median(values[mid:])
    if first_half == 0:
        return "stable"
    change = (second_half - first_half) / first_half
    if change < -0.05:
        return "improving"
    if change > 0.05:
        return "degrading"
    return "stable"


def compute_moving_median(
    history: list[dict[str, Any]],
    n: int = HISTORY_N,
) -> dict[str, dict[str, float | None]]:
    """Compute moving median of key metrics over the last *n* results.

    *history* is newest-first.  Returns ``{bench_name: {metric_key: value}}``.
    """
    window = history[:n]
    if not window:
        return {}
    bench_names = {b["name"] for run in window for b in run.get("benchmarks", [])}
    result: dict[str, dict[str, float | None]] = {}
    for name in sorted(bench_names):
        metrics: dict[str, float | None] = {}
        for metric_key in ("compile_ms", "run_ms", "binary_bytes"):
            vals = []
            for run in window:
                bmap = {b["name"]: b for b in run.get("benchmarks", [])}
                b = bmap.get(name)
                if b is not None:
                    v = _bench_metric_value(b, metric_key)
                    if v is not None:
                        vals.append(v)
            metrics[metric_key] = round(statistics.median(vals), 3) if vals else None
        result[name] = metrics
    return result


def compute_trend_context(
    history: list[dict[str, Any]],
    current: dict[str, Any],
    n: int = HISTORY_N,
) -> dict[str, Any]:
    """Build trend context from prior history and current results.

    *history* is newest-first and does NOT include the current run.
    Returns a dict with ``available``, ``history_depth``, ``moving_median``,
    and ``bench_trends`` keys.
    """
    if not history:
        return {"available": False, "history_depth": 0, "moving_median": {}, "bench_trends": {}}
    # Full series newest → oldest: current + history
    full_series = [current] + history
    bench_trends: dict[str, dict[str, str]] = {}
    current_benches = {b["name"]: b for b in current.get("benchmarks", [])}
    for name in sorted(current_benches):
        trends: dict[str, str] = {}
        for metric_key in ("compile_ms", "run_ms"):
            # Build oldest-first series bounded by n
            vals_oldest_first: list[float] = []
            for run in reversed(full_series[:n]):
                bmap = {b["name"]: b for b in run.get("benchmarks", [])}
                b = bmap.get(name)
                if b is not None:
                    v = _bench_metric_value(b, metric_key)
                    if v is not None:
                        vals_oldest_first.append(v)
            trends[metric_key] = _trend_label(vals_oldest_first)
        bench_trends[name] = trends
    moving_med = compute_moving_median(history, n=n)
    return {
        "available": len(history) >= 2,
        "history_depth": len(history),
        "moving_median": moving_med,
        "bench_trends": bench_trends,
    }


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
    parser.add_argument("--runtime-latency-iterations", type=int,
                        help="Iterations for runtime latency / percentile measurement (overrides mode preset).")
    parser.add_argument("--no-write-markdown", action="store_true")
    parser.add_argument("--no-write-json", action="store_true")
    parser.add_argument("--no-save-history", action="store_true",
                        help="Skip saving a timestamped copy to benchmarks/results/.")
    parser.add_argument("--history-n", type=int, default=HISTORY_N,
                        help="Number of prior runs to use for moving median and trend (default: %(default)s).")
    parser.add_argument("--fail-on-regression", action="store_true")
    parser.add_argument("--print-json", action="store_true")
    parser.add_argument("--repro-output-json",
                        help="Path to write reproducibility report JSON (--mode reproducibility only).")
    parser.add_argument("--scaling-output-json",
                        help="Path to write scaling-curve report JSON (--mode scaling only).")
    return parser.parse_args()


def _run_reproducibility(args: argparse.Namespace) -> None:
    """Execute two benchmark passes and compare for reproducibility.

    Does NOT touch the existing compare mode or baseline infrastructure.
    Writes a separate JSON report and prints a text summary.
    """
    sys.stdout.write("Running reproducibility pass 1 of 2...\n")
    sys.stdout.flush()
    run1 = collect_results(args)
    sys.stdout.write("Running reproducibility pass 2 of 2...\n")
    sys.stdout.flush()
    run2 = collect_results(args)
    report = compare_reproducibility(run1, run2)
    text = render_reproducibility_text(report)
    sys.stdout.write(text)
    repro_json_path = Path(
        args.repro_output_json
        if args.repro_output_json
        else str(Path(args.output_json).parent / "reproducibility.json")
    )
    if not args.no_write_json:
        write_json(repro_json_path, report)
        sys.stdout.write(f"Reproducibility report written to: {rel(repro_json_path) if repro_json_path.is_relative_to(ROOT) else str(repro_json_path)}\n")
    if args.print_json:
        sys.stdout.write(json.dumps(report, indent=2) + "\n")
    if not report["reproducible"]:
        raise SystemExit(1)


def main() -> None:
    args = parse_args()
    if args.mode == "reproducibility":
        _run_reproducibility(args)
        return
    if args.mode == "scaling":
        _run_scaling(args)
        return
    if args.mode == "trend":
        _run_trend(args)
        return
    current = collect_results(args)
    baseline_path = Path(args.baseline)
    baseline = load_json(baseline_path)
    comparison = compare_results(current, baseline)

    output_json = Path(args.output_json)
    output_md = Path(args.output_md)

    # Load prior history for trend context BEFORE saving so the current run is
    # not included in the moving median window.
    history_n: int = args.history_n
    prior_history = load_history(current["mode"], current["target"], limit=history_n)
    trend_ctx = compute_trend_context(prior_history, current, n=history_n)

    if not args.no_write_json:
        write_json(output_json, current)
    if not args.no_write_markdown:
        preserved_cross: str | None = None
        if output_md.exists():
            preserved_cross = extract_preserved_cross_lang_markdown(
                output_md.read_text(encoding="utf-8")
            )
        md_text = render_markdown(current, comparison, baseline_path)
        output_md.write_text(merge_preserved_cross_lang_markdown(md_text, preserved_cross))

    # Save timestamped copy to benchmarks/results/ for history tracking.
    if not args.no_save_history:
        history_path = save_to_history(current)
        sys.stdout.write(f"History  : saved to {rel(history_path)}\n")

    text = render_text(current, comparison, baseline_path, trend_ctx=trend_ctx)
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
