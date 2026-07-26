#!/usr/bin/env python3
"""Run native_cpp_public fixtures and write a coverage receipt."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PUBLIC_DIR = REPO_ROOT / "tests" / "fixtures" / "native_cpp_public"
WRAPPER = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
DEFAULT_OUTPUT = REPO_ROOT / "docs" / "data" / "native-cpp-public-coverage-receipt.json"

# Compile-only / trap negative fixtures: expect non-zero status + diagnostic.
NEGATIVE = {
    "main_with_param.ark": "requires `fn main()`",
    "panic_message.ark": "panic",
    "trap_div_zero.ark": "divide by zero",
}

# Positive fixtures that intentionally exit non-zero.
EXPECTED_EXIT = {
    "process_exit_7.ark": 7,
}


def _build_dir() -> Path:
    env = os.environ.get("ARUKELLT_BUILD_DIR", "").strip()
    if env:
        return Path(env)
    return REPO_ROOT / ".build"


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
    return env


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()

    build_dir = _build_dir()
    fixtures = sorted(PUBLIC_DIR.glob("*.ark"))
    if not fixtures:
        print("no native_cpp_public fixtures found", file=sys.stderr)
        return 1

    results: list[dict[str, object]] = []
    failures = 0
    for fixture in fixtures:
        name = fixture.name
        result = subprocess.run(
            [str(WRAPPER), "run", str(fixture.relative_to(REPO_ROOT)), "--target", "native-cpp"],
            cwd=REPO_ROOT,
            env=_env(build_dir),
            capture_output=True,
            text=True,
            check=False,
        )
        combined = result.stdout + result.stderr
        expect_negative = name in NEGATIVE
        needle = NEGATIVE.get(name, "")
        expected_exit = EXPECTED_EXIT.get(name)
        if expect_negative:
            ok = result.returncode != 0 and needle in combined
            kind = "negative"
        elif expected_exit is not None:
            ok = result.returncode == expected_exit
            kind = "positive_exit"
        else:
            ok = result.returncode == 0
            kind = "positive"
        if not ok:
            failures += 1
        results.append(
            {
                "fixture": str(fixture.relative_to(REPO_ROOT)),
                "kind": kind,
                "returncode": result.returncode,
                "ok": ok,
                "expected_diagnostic": needle if expect_negative else "",
                "expected_exit": expected_exit if expected_exit is not None else 0,
            }
        )

    receipt = {
        "schema": "native-cpp-public-coverage-receipt/v1",
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "fixture_count": len(results),
        "passed": sum(1 for item in results if item["ok"]),
        "failed": failures,
        "hof": {
            "zero_capture": "supported",
            "capture_closures": "rejected_known_limitation",
        },
        "phi": {
            "edge_parallel_copy": "supported",
            "note": "structured if/else joins use shared locals; block.phis use edge copies",
        },
        "results": results,
    }

    text = json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    if args.write:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text, encoding="utf-8")
        print(f"wrote {args.output}")
    else:
        sys.stdout.write(text)

    if failures:
        print(f"public corpus failures: {failures}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
