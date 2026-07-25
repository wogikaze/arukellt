#!/usr/bin/env python3
"""Validate docs/data/native-cpp-run-promotion-receipt.json freshness/shape."""

from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RECEIPT = ROOT / "docs" / "data" / "native-cpp-run-promotion-receipt.json"
PROJECT_STATE = ROOT / "docs" / "data" / "project-state.toml"
MAX_AGE_DAYS = 30


def main() -> int:
    if not RECEIPT.is_file():
        print(f"FAIL: missing {RECEIPT}", file=sys.stderr)
        return 1
    data = json.loads(RECEIPT.read_text(encoding="utf-8"))
    required = [
        "receipt_kind",
        "created_utc",
        "run_supported",
        "public_coverage_receipt",
        "gates",
    ]
    for key in required:
        if key not in data:
            print(f"FAIL: receipt missing key {key}", file=sys.stderr)
            return 1
    if data.get("receipt_kind") != "native-cpp-public-run-experimental-promotion":
        print("FAIL: unexpected receipt_kind", file=sys.stderr)
        return 1
    if data.get("run_supported") is not True:
        print("FAIL: receipt run_supported must be true for promotion", file=sys.stderr)
        return 1
    created = datetime.strptime(data["created_utc"], "%Y-%m-%dT%H:%M:%SZ").replace(
        tzinfo=timezone.utc
    )
    age = datetime.now(timezone.utc) - created
    if age.days > MAX_AGE_DAYS:
        print(f"FAIL: promotion receipt stale ({age.days} days)", file=sys.stderr)
        return 1
    state = PROJECT_STATE.read_text(encoding="utf-8")
    if 'id = "native-cpp"' not in state or "run_supported = true" not in state:
        # Narrow check: native-cpp block must claim run_supported true.
        in_native = False
        native_run = None
        for line in state.splitlines():
            if line.strip() == 'id = "native-cpp"':
                in_native = True
            elif in_native and line.startswith("[["):
                break
            elif in_native and line.strip().startswith("run_supported"):
                native_run = line.strip().endswith("true")
        if native_run is not True:
            print("FAIL: project-state native-cpp run_supported is not true", file=sys.stderr)
            return 1
    coverage = ROOT / data["public_coverage_receipt"]
    if not coverage.is_file():
        print(f"FAIL: missing coverage receipt {coverage}", file=sys.stderr)
        return 1
    gates = data["gates"]
    for key, expected in (
        ("public_corpus", "PASS"),
        ("parity", "PASS"),
        ("ubsan_werror", "PASS"),
        ("installed_layout", "PASS"),
    ):
        if gates.get(key) != expected:
            print(f"FAIL: gate {key}={gates.get(key)!r} expected {expected!r}", file=sys.stderr)
            return 1
    print("native-cpp-run-promotion-receipt: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
