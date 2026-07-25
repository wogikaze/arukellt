#!/usr/bin/env python3
"""Measure native-cpp compile/run outcomes across tests/fixtures/**."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
WRAPPER = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
FIXTURES = REPO_ROOT / "tests" / "fixtures"
DEFAULT_OUTPUT = REPO_ROOT / "docs" / "data" / "native-cpp-fixture-coverage-receipt.json"

CAPABILITY_RE = re.compile(
    r"does not support MIR opcode|does not support CoreOp|target `native-cpp`",
    re.I,
)
ICE_RE = re.compile(r"compiler ICE|wasm trap|unreachable instruction", re.I)
MAIN_PARAM_RE = re.compile(r"requires `fn main\(\)` with no parameters", re.I)
NO_MAIN_RE = re.compile(r"requires a `fn main\(\)` entry point", re.I)


def _build_dir() -> Path:
    env = os.environ.get("ARUKELLT_BUILD_DIR", "").strip()
    return Path(env) if env else REPO_ROOT / ".build"


def _env(build_dir: Path) -> dict[str, str]:
    env = os.environ.copy()
    env["ARUKELLT_BUILD_DIR"] = str(build_dir)
    for candidate in (
        build_dir / "selfhost" / "arukellt-s2-runtime.wasm",
        build_dir / "selfhost" / "arukellt-s2.wasm",
        REPO_ROOT / ".build" / "selfhost" / "arukellt-s2-runtime.wasm",
    ):
        if candidate.is_file():
            env["ARUKELLT_SELFHOST_WASM"] = str(candidate)
            break
    # Do not set ARUKELLT_NATIVE_CPP_INTERNAL_COMPILE here: that flag disables the
    # public `run --target native-cpp` launcher and falls through to emit wasm.
    return env


def _has_main(path: Path) -> bool:
    text = path.read_text(encoding="utf-8", errors="ignore")
    return "fn main(" in text


def _classify_compile(returncode: int, combined: str) -> str:
    if returncode == 0:
        return "compile_pass"
    if MAIN_PARAM_RE.search(combined) or NO_MAIN_RE.search(combined):
        return "entry_reject"
    if ICE_RE.search(combined):
        return "ice"
    if CAPABILITY_RE.search(combined) and (
        "does not support" in combined or "planned phase" in combined or "unsupported" in combined
    ):
        return "capability_reject"
    if "error[" in combined or "failed to" in combined.lower():
        return "frontend_error"
    return "other_fail"


def _measure_one(
    fixture: Path,
    *,
    build_dir: Path,
    out_root: Path,
    do_run: bool,
    timeout: float,
) -> dict[str, object]:
    rel = fixture.relative_to(REPO_ROOT)
    digest = re.sub(r"[^A-Za-z0-9._-]+", "_", str(rel))
    c_path = out_root / f"{digest}.c"
    c_path.parent.mkdir(parents=True, exist_ok=True)
    started = time.time()
    compile = subprocess.run(
        [
            str(WRAPPER),
            "compile",
            str(rel),
            "--target",
            "native-cpp",
            "--emit",
            "c",
            "-o",
            str(c_path.relative_to(REPO_ROOT)),
        ],
        cwd=REPO_ROOT,
        env=_env(build_dir),
        capture_output=True,
        text=True,
        check=False,
        timeout=timeout,
    )
    combined = compile.stdout + compile.stderr
    kind = _classify_compile(compile.returncode, combined)
    result: dict[str, object] = {
        "fixture": str(rel),
        "compile_kind": kind,
        "compile_returncode": compile.returncode,
        "compile_seconds": round(time.time() - started, 3),
    }
    if kind != "compile_pass" or not do_run:
        result["run_kind"] = "skipped"
        return result

    run_started = time.time()
    run = subprocess.run(
        [str(WRAPPER), "run", str(rel), "--target", "native-cpp"],
        cwd=REPO_ROOT,
        env=_env(build_dir),
        capture_output=True,
        text=True,
        check=False,
        timeout=timeout,
    )
    run_combined = run.stdout + run.stderr
    if run.returncode == 0:
        run_kind = "run_pass"
    elif ICE_RE.search(run_combined) or run.returncode < 0:
        run_kind = "run_crash"
    else:
        run_kind = "run_nonzero"
    result["run_kind"] = run_kind
    result["run_returncode"] = run.returncode
    result["run_seconds"] = round(time.time() - run_started, 3)
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--timeout", type=float, default=60.0)
    parser.add_argument("--limit", type=int, default=0, help="0 = all fixtures with main")
    parser.add_argument("--compile-only", action="store_true")
    parser.add_argument(
        "--run-pass-compiles",
        action="store_true",
        help="Also run fixtures that compile successfully (default with --write)",
    )
    args = parser.parse_args()
    do_run = args.run_pass_compiles or (args.write and not args.compile_only)

    build_dir = _build_dir()
    out_root = build_dir / "native-cpp-fixture-measure" / "c"
    out_root.mkdir(parents=True, exist_ok=True)

    fixtures = sorted(p for p in FIXTURES.rglob("*.ark") if _has_main(p))
    if args.limit > 0:
        fixtures = fixtures[: args.limit]

    results: list[dict[str, object]] = []
    errors = 0
    started = time.time()
    with ThreadPoolExecutor(max_workers=max(1, args.workers)) as pool:
        futures = {
            pool.submit(
                _measure_one,
                fixture,
                build_dir=build_dir,
                out_root=out_root,
                do_run=do_run,
                timeout=args.timeout,
            ): fixture
            for fixture in fixtures
        }
        done = 0
        for future in as_completed(futures):
            done += 1
            fixture = futures[future]
            try:
                results.append(future.result())
            except subprocess.TimeoutExpired:
                errors += 1
                results.append(
                    {
                        "fixture": str(fixture.relative_to(REPO_ROOT)),
                        "compile_kind": "timeout",
                        "compile_returncode": -1,
                        "run_kind": "skipped",
                    }
                )
            except Exception as exc:  # noqa: BLE001
                errors += 1
                results.append(
                    {
                        "fixture": str(fixture.relative_to(REPO_ROOT)),
                        "compile_kind": "harness_error",
                        "compile_returncode": -1,
                        "run_kind": "skipped",
                        "error": str(exc),
                    }
                )
            if done % 50 == 0 or done == len(fixtures):
                print(f"progress {done}/{len(fixtures)}", file=sys.stderr)

    results.sort(key=lambda item: str(item["fixture"]))
    compile_counts: dict[str, int] = {}
    run_counts: dict[str, int] = {}
    by_dir: dict[str, dict[str, int]] = {}
    for item in results:
        compile_counts[str(item["compile_kind"])] = compile_counts.get(str(item["compile_kind"]), 0) + 1
        run_counts[str(item["run_kind"])] = run_counts.get(str(item["run_kind"]), 0) + 1
        parts = str(item["fixture"]).split("/")
        top = parts[2] if len(parts) > 2 else "?"
        bucket = by_dir.setdefault(top, {"total": 0})
        bucket["total"] += 1
        bucket[str(item["compile_kind"])] = bucket.get(str(item["compile_kind"]), 0) + 1
        bucket[str(item["run_kind"])] = bucket.get(str(item["run_kind"]), 0) + 1

    directory_summary = []
    for top in sorted(by_dir):
        bucket = by_dir[top]
        total_dir = bucket["total"]
        compile_pass_dir = bucket.get("compile_pass", 0)
        run_pass_dir = bucket.get("run_pass", 0)
        directory_summary.append(
            {
                "directory": top,
                "total": total_dir,
                "compile_pass": compile_pass_dir,
                "run_pass": run_pass_dir,
                "capability_reject": bucket.get("capability_reject", 0),
                "ice": bucket.get("ice", 0),
                "frontend_error": bucket.get("frontend_error", 0),
                "run_nonzero": bucket.get("run_nonzero", 0),
                "run_crash": bucket.get("run_crash", 0),
                "compile_pass_rate": round(compile_pass_dir / total_dir, 4) if total_dir else 0.0,
                "run_pass_rate": round(run_pass_dir / total_dir, 4) if total_dir else 0.0,
            }
        )

    total = len(results)
    compile_pass = compile_counts.get("compile_pass", 0)
    run_pass = run_counts.get("run_pass", 0)
    receipt = {
        "schema": "native-cpp-fixture-coverage-receipt/v1",
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "fixture_root": "tests/fixtures",
        "selection": "files containing `fn main(`",
        "total": total,
        "elapsed_seconds": round(time.time() - started, 1),
        "workers": args.workers,
        "ran_executables": do_run,
        "compile_counts": compile_counts,
        "run_counts": run_counts,
        "directory_summary": directory_summary,
        "rates": {
            "compile_pass_rate": round(compile_pass / total, 4) if total else 0.0,
            "run_pass_rate_of_all": round(run_pass / total, 4) if total and do_run else None,
            "run_pass_rate_of_compile_pass": (
                round(run_pass / compile_pass, 4) if compile_pass and do_run else None
            ),
        },
        "notes": [
            "Selection: tests/fixtures/**/*.ark containing `fn main(`.",
            "compile_pass: `compile --target native-cpp --emit c` exit 0.",
            "run_pass: `run --target native-cpp` exit 0 (intentional non-zero exits count as run_nonzero).",
            "capability_reject is an honest unsupported MIR/CoreOp diagnostic, not a harness bug.",
        ],
        "harness_errors": errors,
        "results": results,
    }

    text = json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    if args.write:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text, encoding="utf-8")
        print(f"wrote {args.output}", file=sys.stderr)
    else:
        # summary only to stdout when not writing full results to keep terminals usable
        summary = {
            k: receipt[k]
            for k in (
                "schema",
                "generated_at",
                "total",
                "elapsed_seconds",
                "compile_counts",
                "run_counts",
                "rates",
            )
        }
        print(json.dumps(summary, indent=2, sort_keys=True))
    print(
        f"compile_pass={compile_pass}/{total} ({100 * compile_pass / total:.1f}%) "
        f"run_pass={run_pass}/{total if do_run else 0}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
