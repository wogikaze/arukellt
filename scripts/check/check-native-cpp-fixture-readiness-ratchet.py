#!/usr/bin/env python3
"""Ratchet: native-cpp fixture readiness must not regress vs baseline."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_RECEIPT = REPO_ROOT / "docs" / "data" / "native-cpp-fixture-coverage-receipt.json"
DEFAULT_BASELINE = REPO_ROOT / "docs" / "data" / "native-cpp-fixture-coverage-baseline.json"


def _load(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise SystemExit(f"{path}: expected object")
    return data


def _metric(data: dict, key: str) -> float | int | None:
    counts = data.get("counts") or {}
    rates = data.get("rates") or {}
    if key in counts:
        return counts[key]
    if key in rates:
        return rates[key]
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", type=Path, default=DEFAULT_RECEIPT)
    parser.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE)
    parser.add_argument(
        "--write-baseline",
        action="store_true",
        help="Copy current complete receipt counts/rates into baseline",
    )
    args = parser.parse_args()

    if not args.receipt.is_file():
        print(f"missing receipt: {args.receipt}", file=sys.stderr)
        return 1
    receipt = _load(args.receipt)
    if receipt.get("schema") != "native-cpp-fixture-coverage-receipt/v2":
        print("receipt schema must be native-cpp-fixture-coverage-receipt/v2", file=sys.stderr)
        return 1
    if not receipt.get("complete"):
        print("receipt is not marked complete", file=sys.stderr)
        return 1

    if args.write_baseline:
        baseline = {
            "schema": "native-cpp-fixture-coverage-baseline/v1",
            "source_receipt_generated_at": receipt.get("generated_at"),
            "source_commit": (receipt.get("environment") or {}).get("source_commit"),
            "counts": receipt.get("counts"),
            "rates": receipt.get("rates"),
            "hard_zero": {
                "ice_total": bool((receipt.get("counts") or {}).get("ice_total") == 0),
                "unexpected_crash": bool(
                    (receipt.get("counts") or {}).get("unexpected_crash") == 0
                ),
            },
            "notes": [
                "Worsening is forbidden relative to these floors/ceilings.",
                "When ice_total or unexpected_crash reaches 0, hard_zero locks that metric at 0.",
            ],
        }
        args.baseline.parent.mkdir(parents=True, exist_ok=True)
        args.baseline.write_text(json.dumps(baseline, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(f"wrote baseline {args.baseline}")
        return 0

    if not args.baseline.is_file():
        print(f"missing baseline: {args.baseline}", file=sys.stderr)
        return 1
    baseline = _load(args.baseline)

    failures: list[str] = []

    def require_le(name: str) -> None:
        cur = _metric(receipt, name)
        base = _metric(baseline, name)
        if cur is None or base is None:
            failures.append(f"{name}: missing metric")
            return
        hard = (baseline.get("hard_zero") or {}).get(name)
        limit = 0 if hard else base
        if cur > limit:
            failures.append(f"{name}: {cur} > {limit} (baseline {base}, hard_zero={bool(hard)})")

    def require_ge_rate(name: str) -> None:
        cur = _metric(receipt, name)
        base = _metric(baseline, name)
        if cur is None or base is None:
            failures.append(f"{name}: missing metric")
            return
        if float(cur) + 1e-12 < float(base):
            failures.append(f"{name}: {cur} < baseline {base}")

    require_le("ice_total")
    require_le("unexpected_crash")
    require_ge_rate("positive_compile_pass_rate")
    require_ge_rate("compiled_positive_semantic_run_pass_rate")

    # Promote hard-zero locks when current hits 0.
    hard_zero = dict(baseline.get("hard_zero") or {})
    counts = receipt.get("counts") or {}
    updated = False
    for key in ("ice_total", "unexpected_crash"):
        if counts.get(key) == 0 and not hard_zero.get(key):
            hard_zero[key] = True
            updated = True
    if updated:
        baseline["hard_zero"] = hard_zero
        args.baseline.write_text(
            json.dumps(baseline, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        print(f"updated hard_zero locks in {args.baseline}")

    if failures:
        print("native-cpp fixture readiness ratchet FAILED:", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1

    print("native-cpp fixture readiness ratchet OK")
    print(
        "ice_total={ice} unexpected_crash={crash} "
        "positive_compile={pc} compiled_semantic={cs}".format(
            ice=counts.get("ice_total"),
            crash=counts.get("unexpected_crash"),
            pc=(receipt.get("rates") or {}).get("positive_compile_pass_rate"),
            cs=(receipt.get("rates") or {}).get("compiled_positive_semantic_run_pass_rate"),
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
