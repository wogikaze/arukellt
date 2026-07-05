#!/usr/bin/env python3
"""compare-linear-vs-gc.py — Linear memory vs Wasm GC performance comparison.

Compares ADR-002 fixtures across two compilation targets (wasm32-wasi-p1
linear memory, wasm32-wasi-p2 Wasm GC) and three runtimes (wasmtime, Node.js,
headless Chrome browser).

Usage:
    python3 scripts/perf/compare-linear-vs-gc.py [--iterations N] [--warmups N]
        [--runtimes wasmtime,node,browser] [--output-dir DIR] [--no-compile]

Output:
    <output-dir>/linear-vs-gc-results.json   — machine-readable results
    <output-dir>/linear-vs-gc-report.md      — human-readable comparison table
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
BUILD_DIR = REPO_ROOT / ".build" / "perf-linear-vs-gc"

# ADR-002 fixtures: (name, source, expected, needs_dir)
FIXTURES = [
    ("hello", "benchmarks/legacy/hello.ark", "Hello, World!\n", False),
    ("string_concat", "benchmarks/legacy/string_concat.ark", "100\n", False),
    ("vec_pushpop", "benchmarks/legacy/vec_push_pop.ark", "100000\n0\n", False),
    ("binary_tree", "benchmarks/legacy/binary_tree.ark", "2097151\n", False),
    ("result_heavy", "benchmarks/legacy/result_heavy.ark", "28571\n21429\n1428557143\n", False),
    ("file_read", "benchmarks/legacy/file_read.ark", "470000\n", True),
]

TARGETS = {
    "linear": "wasm32-wasi-p1",
    "gc": "wasm32-wasi-p2",
}


# ─── Compilation ──────────────────────────────────────────────────────────────

def compile_fixture(name: str, source: str, target: str, out_path: Path) -> dict:
    """Compile a fixture to the given target. Returns status dict."""
    src = REPO_ROOT / source
    if not src.is_file():
        return {"status": "error", "error": f"source not found: {src}"}
    out_rel = out_path.relative_to(REPO_ROOT)
    cmd = ["arukellt", "compile", str(src.relative_to(REPO_ROOT)), "--target", target,
           "-o", str(out_rel)]
    try:
        r = subprocess.run(cmd, cwd=str(REPO_ROOT), capture_output=True, text=True, timeout=120)
        if r.returncode != 0 or not out_path.is_file():
            return {"status": "error", "error": r.stderr.strip()[-200:] or "compile failed"}
        return {"status": "ok", "wasm_bytes": out_path.stat().st_size}
    except Exception as e:
        return {"status": "error", "error": str(e)}


# ─── wasmtime runtime (via wasmtime-py) ───────────────────────────────────────

def run_wasmtime(wasm_path: Path, expected: str, iterations: int, warmups: int) -> dict:
    """Run a wasm module via wasmtime-py, instantiating once and calling _start N times."""
    try:
        import wasmtime
    except ImportError:
        return {"runtime": "wasmtime", "status": "error", "error": "wasmtime-py not installed"}

    try:
        config = wasmtime.Config()
        config.wasm_gc = True
        engine = wasmtime.Engine(config)
        module = wasmtime.Module.from_file(engine, str(wasm_path))

        linker = wasmtime.Linker(engine)
        linker.allow_shadowing = True

        stdout_parts: list[str] = []
        mem_ref: list = [None]

        def _read_mem(caller, ptr, length):
            mem = mem_ref[0]
            if mem is None:
                return ""
            raw = mem.read(caller, ptr, ptr + length)
            return bytes(raw).decode("utf-8", errors="replace")

        def _write_p1(caller, fd, iovs_ptr, iovs_len, nwritten_ptr):
            """P1 fd_write: read iovec array from memory."""
            mem = mem_ref[0]
            total = 0
            if mem is not None:
                for i in range(iovs_len):
                    # Each iovec is (ptr: u32, len: u32) = 8 bytes
                    iov_data = mem.read(caller, iovs_ptr + i * 8, iovs_ptr + i * 8 + 8)
                    ptr_val = int.from_bytes(iov_data[0:4], "little")
                    len_val = int.from_bytes(iov_data[4:8], "little")
                    stdout_parts.append(_read_mem(caller, ptr_val, len_val))
                    total += len_val
            # Write nwritten
            if mem is not None and nwritten_ptr != 0:
                mem.write(caller, total.to_bytes(4, "little"), nwritten_ptr)
            return 0  # errno success

        def _write_p2(caller, fd, ptr, length, nwritten_ptr):
            """P2 write: single buffer."""
            stdout_parts.append(_read_mem(caller, ptr, length))
            return length  # nwritten

        def _exit(caller, code):
            raise RuntimeError(f"exit({code})")

        def _now(caller):
            return 0

        def _rand(caller):
            return 0

        def _args_sizes(caller, a, b):
            return 0

        def _stub_i32(caller, *args):
            return 0

        def _stub_void(caller, *args):
            return None

        # Check if P1 or P2
        import_modules = {imp.module for imp in module.imports}
        is_p1 = "wasi_snapshot_preview1" in import_modules

        # Use custom stubs for both P1 and P2 to capture stdout consistently
        # across all runtimes (wasmtime, node, browser). This means file I/O
        # fixtures (file_read) will not have real file access — same as node/browser.
        for imp in module.imports:
            mod_name, fn_name, ftype = imp.module, imp.name, imp.type
            has_results = len(ftype.results) > 0
            if is_p1 and fn_name == "fd_write":
                linker.define_func(mod_name, fn_name, ftype, _write_p1, access_caller=True)
            elif "stdout" in mod_name and fn_name == "write":
                linker.define_func(mod_name, fn_name, ftype, _write_p2, access_caller=True)
            elif "exit" in fn_name or (is_p1 and fn_name == "proc_exit"):
                linker.define_func(mod_name, fn_name, ftype, _exit, access_caller=True)
            elif (is_p1 and fn_name == "clock_time_get") or ("monotonic" in mod_name and fn_name == "now"):
                linker.define_func(mod_name, fn_name, ftype, _now, access_caller=True)
            elif "get-random" in fn_name or (is_p1 and fn_name == "random_get"):
                linker.define_func(mod_name, fn_name, ftype, _rand, access_caller=True)
            elif "args-sizes" in fn_name or (is_p1 and fn_name == "args_sizes_get"):
                linker.define_func(mod_name, fn_name, ftype, _args_sizes, access_caller=True)
            elif has_results:
                linker.define_func(mod_name, fn_name, ftype, _stub_i32, access_caller=True)
            else:
                linker.define_func(mod_name, fn_name, ftype, _stub_void, access_caller=True)

        store = wasmtime.Store(engine)
        instance = linker.instantiate(store, module)
        exports = instance.exports(store)

        if "memory" in exports:
            mem_ref[0] = exports["memory"]
        start = exports.get("_start")
        if start is None:
            return {"runtime": "wasmtime", "target": "p1" if is_p1 else "p2",
                    "status": "error", "error": "no _start export"}

        flavor = "p1" if is_p1 else "p2"

        # Warmups
        for _ in range(warmups):
            stdout_parts.clear()
            try:
                start(store)
            except Exception:
                pass

        # Timed iterations
        timings = []
        run_ok = False
        last_err = None
        for _ in range(iterations):
            stdout_parts.clear()
            t0 = time.perf_counter()
            try:
                start(store)
                run_ok = True
            except Exception as e:
                last_err = str(e)
            t1 = time.perf_counter()
            timings.append((t1 - t0) * 1000)

        if not run_ok:
            return {"runtime": "wasmtime", "target": flavor, "status": "error",
                    "error": last_err or "all runs failed", "iterations": iterations, "warmups": warmups}

        timings.sort()
        actual = "".join(stdout_parts).strip()
        expected_clean = expected.strip()
        correctness = "pass" if actual == expected_clean else "fail"

        n = len(timings)
        return {
            "runtime": "wasmtime", "target": flavor, "status": "ok",
            "correctness": correctness, "expected": expected_clean, "actual": actual,
            "timings_ms": timings, "median_ms": timings[n // 2],
            "p50_ms": timings[min(n - 1, n // 2)],
            "p95_ms": timings[min(n - 1, int(n * 0.95))],
            "min_ms": timings[0], "max_ms": timings[-1],
            "iterations": iterations, "warmups": warmups, "error": None,
        }
    except Exception as e:
        return {"runtime": "wasmtime", "status": "error", "error": str(e)}


# ─── Node.js runtime ──────────────────────────────────────────────────────────

def run_node(wasm_path: Path, expected: str, iterations: int, warmups: int) -> dict:
    """Run a wasm module via the Node.js adapter."""
    adapter = REPO_ROOT / "scripts" / "perf" / "run-node-bench.mjs"
    cmd = ["node", str(adapter), str(wasm_path),
           "--expected", expected, "--iterations", str(iterations),
           "--warmups", str(warmups)]
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
        if r.returncode != 0:
            # Try to parse JSON error from stdout
            try:
                return json.loads(r.stdout.strip().split("\n")[-1])
            except Exception:
                return {"runtime": "node", "status": "error", "error": r.stderr[-300:]}
        return json.loads(r.stdout.strip().split("\n")[-1])
    except Exception as e:
        return {"runtime": "node", "status": "error", "error": str(e)}


# ─── Browser runtime ──────────────────────────────────────────────────────────

def run_browser(wasm_path: Path, expected: str, iterations: int, warmups: int) -> dict:
    """Run a wasm module via the browser adapter (headless Chrome)."""
    adapter = REPO_ROOT / "scripts" / "perf" / "run-browser-bench.mjs"
    cmd = ["node", str(adapter), str(wasm_path),
           "--expected", expected, "--iterations", str(iterations),
           "--warmups", str(warmups)]
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=180)
        if r.returncode != 0:
            try:
                return json.loads(r.stdout.strip().split("\n")[-1])
            except Exception:
                return {"runtime": "browser", "status": "error", "error": r.stderr[-300:]}
        return json.loads(r.stdout.strip().split("\n")[-1])
    except Exception as e:
        return {"runtime": "browser", "status": "error", "error": str(e)}


RUNTIME_FUNCS = {
    "wasmtime": run_wasmtime,
    "node": run_node,
    "browser": run_browser,
}


# ─── Report generation ────────────────────────────────────────────────────────

def generate_markdown_report(results: dict) -> str:
    """Generate a Markdown comparison report from results."""
    lines = []
    lines.append("# Linear Memory vs Wasm GC — Performance Comparison")
    lines.append("")
    lines.append(f"Generated: {results['generated_at']}")
    lines.append(f"Iterations: {results['iterations']}, Warmups: {results['warmups']}")
    lines.append("")
    lines.append("Fixtures from ADR-002 (GC vs non-GC decision benchmark).")
    lines.append("")

    for runtime in results["runtimes"]:
        lines.append(f"## {runtime}")
        lines.append("")
        lines.append("| Fixture | Linear (ms) | GC (ms) | Ratio GC/Linear | Linear status | GC status |")
        lines.append("|---------|------------|---------|-----------------|---------------|-----------|")
        for fixture in results["fixtures"]:
            fdata = results["results"][fixture][runtime]
            lin = fdata.get("linear", {})
            gc = fdata.get("gc", {})
            lin_ms = lin.get("median_ms")
            gc_ms = gc.get("median_ms")
            lin_status = lin.get("status", "?")
            gc_status = gc.get("status", "?")
            lin_str = f"{lin_ms:.3f}" if lin_ms is not None else "—"
            gc_str = f"{gc_ms:.3f}" if gc_ms is not None else "—"
            if lin_ms and gc_ms and lin_ms > 0:
                ratio = gc_ms / lin_ms
                ratio_str = f"{ratio:.2f}x" + (" ⚠️" if ratio >= 1.5 else "")
            else:
                ratio_str = "—"
            lin_corr = lin.get("correctness", "")
            gc_corr = gc.get("correctness", "")
            lin_disp = f"{lin_str}" + (f" ({lin_corr})" if lin_corr and lin_corr != "pass" else "")
            gc_disp = f"{gc_str}" + (f" ({gc_corr})" if gc_corr and gc_corr != "pass" else "")
            if lin_status != "ok":
                lin_disp = f"error: {lin.get('error', '?')[:40]}"
            if gc_status != "ok":
                gc_disp = f"error: {gc.get('error', '?')[:40]}"
            lines.append(f"| {fixture} | {lin_disp} | {gc_disp} | {ratio_str} | {lin_status} | {gc_status} |")
        lines.append("")

    # Cross-runtime summary
    lines.append("## Cross-Runtime Summary (median ms)")
    lines.append("")
    lines.append("| Fixture | Target | wasmtime | node | browser |")
    lines.append("|---------|--------|----------|------|---------|")
    for fixture in results["fixtures"]:
        for target in ["linear", "gc"]:
            vals = []
            for runtime in results["runtimes"]:
                r = results["results"][fixture][runtime].get(target, {})
                ms = r.get("median_ms")
                vals.append(f"{ms:.3f}" if ms is not None else "—")
            lines.append(f"| {fixture} | {target} | {vals[0]} | {vals[1]} | {vals[2]} |")
    lines.append("")
    lines.append("## Notes")
    lines.append("")
    lines.append("- **Linear** = `wasm32-wasi-p1` (linear memory + bump allocator)")
    lines.append("- **GC** = `wasm32-wasi-p2` (Wasm GC types, ADR-035 Phase 1 partial)")
    lines.append("- **wasmtime** = wasmtime-py (Cranelift), instantiate once + repeated _start calls")
    lines.append("- **node** = Node.js v23 (V8 12.9) native WebAssembly API")
    lines.append("- **browser** = headless Chrome 147 (V8) via puppeteer-core")
    lines.append("- GC target is ADR-035 Phase 1 partial: some fixtures may fail GC compilation/execution")
    lines.append("- Ratio >= 1.5x means GC is slower than linear (ADR-002 threshold)")
    lines.append("")

    return "\n".join(lines)


# ─── Main ─────────────────────────────────────────────────────────────────────

def main() -> int:
    parser = argparse.ArgumentParser(description="Linear vs GC performance comparison")
    parser.add_argument("--iterations", type=int, default=10, help="Timed iterations per fixture")
    parser.add_argument("--warmups", type=int, default=2, help="Warmup iterations")
    parser.add_argument("--runtimes", default="wasmtime,node,browser",
                        help="Comma-separated runtimes to use")
    parser.add_argument("--output-dir", default="docs/process",
                        help="Output directory for results")
    parser.add_argument("--no-compile", action="store_true",
                        help="Skip compilation, reuse existing wasm files")
    args = parser.parse_args()

    runtimes = args.runtimes.split(",")
    for r in runtimes:
        if r not in RUNTIME_FUNCS:
            print(f"ERROR: unknown runtime '{r}'", file=sys.stderr)
            return 1

    BUILD_DIR.mkdir(parents=True, exist_ok=True)

    results = {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "iterations": args.iterations,
        "warmups": args.warmups,
        "runtimes": runtimes,
        "fixtures": [f[0] for f in FIXTURES],
        "targets": TARGETS,
        "results": {},
    }

    print(f"Linear vs GC benchmark: {len(FIXTURES)} fixtures × {len(runtimes)} runtimes × 2 targets")
    print()

    for name, source, expected, needs_dir in FIXTURES:
        print(f"### {name}")
        results["results"][name] = {}
        wasm_paths = {}

        for target_key, target_name in TARGETS.items():
            out_path = BUILD_DIR / f"{name}-{target_key}.wasm"
            if not args.no_compile or not out_path.is_file():
                print(f"  compile {target_key} ({target_name})... ", end="", flush=True)
                cresult = compile_fixture(name, source, target_name, out_path)
                print(cresult["status"], end="")
                if cresult["status"] != "ok":
                    print(f": {cresult.get('error', '')[:60]}")
                else:
                    print(f" ({cresult['wasm_bytes']} bytes)")
            else:
                if out_path.is_file():
                    print(f"  compile {target_key}: cached ({out_path.stat().st_size} bytes)")
                else:
                    print(f"  compile {target_key}: MISSING (use without --no-compile)")

            if not out_path.is_file():
                wasm_paths[target_key] = None
            else:
                wasm_paths[target_key] = out_path

        for runtime in runtimes:
            results["results"][name][runtime] = {}
            for target_key in ("linear", "gc"):
                wp = wasm_paths.get(target_key)
                if wp is None or not wp.is_file():
                    results["results"][name][runtime][target_key] = {
                        "runtime": runtime, "target": target_key,
                        "status": "error", "error": "wasm not compiled",
                    }
                    print(f"  {runtime}/{target_key}: skipped (no wasm)")
                    continue

                print(f"  {runtime}/{target_key}... ", end="", flush=True)
                func = RUNTIME_FUNCS[runtime]
                r = func(wp, expected, args.iterations, args.warmups)
                results["results"][name][runtime][target_key] = r
                status = r.get("status", "?")
                ms = r.get("median_ms")
                corr = r.get("correctness", "")
                if status == "ok":
                    print(f"median={ms:.3f}ms correctness={corr}")
                else:
                    print(f"error: {r.get('error', '?')[:60]}")

        print()

    # Write JSON results
    out_dir = REPO_ROOT / args.output_dir
    out_dir.mkdir(parents=True, exist_ok=True)
    json_path = out_dir / "linear-vs-gc-results.json"
    json_path.write_text(json.dumps(results, indent=2), encoding="utf-8")
    print(f"JSON results: {json_path.relative_to(REPO_ROOT)}")

    # Generate Markdown report
    md_path = out_dir / "linear-vs-gc-report.md"
    report = generate_markdown_report(results)
    md_path.write_text(report, encoding="utf-8")
    print(f"Markdown report: {md_path.relative_to(REPO_ROOT)}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
