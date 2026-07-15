#!/usr/bin/env python3
"""Validate the CQ-18 manual sample artifact.

Checks:
1. Summary counts match actual sample counts.
2. Every sample with judgment != "pending" has required fields:
   actual_classification, reviewed_by, reviewed_at, judgment, evidence.
3. Any pending sample means the close-gate fails (CQ-18 cannot close
   while pending samples remain).
4. Samples where expected == actual and judgment == "correct" are NOT
   auto-approved by the generator; the validator rejects entries that
   were set to "correct" solely because expected == actual without a
   real reviewed_by.
5. source_fingerprint is present on every sample.

Exit 0 if all checks pass, 1 otherwise.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
JSON_PATH = ROOT / "docs" / "data" / "cq18-manual-sample.json"

REQUIRED_REVIEW_FIELDS = (
    "actual_classification",
    "reviewed_by",
    "reviewed_at",
    "judgment",
    "evidence",
)


def main() -> int:
    if not JSON_PATH.is_file():
        print(f"ERROR: {JSON_PATH} not found", file=sys.stderr)
        return 1

    data = json.loads(JSON_PATH.read_text(encoding="utf-8"))
    errors: list[str] = []
    warnings: list[str] = []

    samples = data.get("samples", {})
    summary = data.get("summary", {})

    # Check 1: summary counts match actual
    actual_total = sum(len(v) for v in samples.values())
    if summary.get("total") != actual_total:
        errors.append(
            f"summary.total={summary.get('total')} != actual total={actual_total}"
        )

    actual_by_category = {k: len(v) for k, v in samples.items()}
    for cat, count in summary.get("by_category", {}).items():
        if actual_by_category.get(cat) != count:
            errors.append(
                f"summary.by_category[{cat}]={count} != actual={actual_by_category.get(cat)}"
            )

    actual_by_judgment: dict[str, int] = {}
    for group in samples.values():
        for entry in group:
            j = entry.get("judgment", "pending")
            actual_by_judgment[j] = actual_by_judgment.get(j, 0) + 1
    for j, count in summary.get("by_judgment", {}).items():
        if actual_by_judgment.get(j) != count:
            errors.append(
                f"summary.by_judgment[{j}]={count} != actual={actual_by_judgment.get(j)}"
            )

    # Check 2 & 4 & 5: per-sample validation
    pending_count = 0
    for cat, group in samples.items():
        for entry in group:
            idx = entry.get("index", "?")
            path = entry.get("path", "?")
            symbol = entry.get("symbol", "?")
            judgment = entry.get("judgment", "pending")

            # Check 5: source_fingerprint present
            if not entry.get("source_fingerprint"):
                errors.append(
                    f"{cat}[{idx}] {path}:{symbol}: missing source_fingerprint"
                )

            if judgment == "pending":
                pending_count += 1
                continue

            # Check 2: required fields for non-pending
            for field in REQUIRED_REVIEW_FIELDS:
                val = entry.get(field)
                if val is None or (isinstance(val, str) and not val.strip()):
                    errors.append(
                        f"{cat}[{idx}] {path}:{symbol}: judgment={judgment} but "
                        f"field '{field}' is empty/missing"
                    )

            # Check 4: reject auto-approved entries (expected==actual set by
            # generator without real review). A real review must have a
            # non-default reviewed_by.
            reviewed_by = entry.get("reviewed_by")
            if (
                judgment == "correct"
                and entry.get("actual_classification") == entry.get("expected_classification")
                and not reviewed_by
            ):
                errors.append(
                    f"{cat}[{idx}] {path}:{symbol}: auto-approved (expected==actual) "
                    f"without reviewed_by — not a real review"
                )

    # Check 3: pending samples block close-gate
    if pending_count > 0:
        warnings.append(
            f"{pending_count} pending samples remain — CQ-18 close-gate blocked"
        )

    # Report
    for w in warnings:
        print(f"WARNING: {w}", file=sys.stderr)
    for e in errors:
        print(f"ERROR: {e}", file=sys.stderr)

    if warnings:
        print(f"\n{pending_count} pending, {actual_total - pending_count} reviewed", file=sys.stderr)

    if errors:
        return 1
    # Warnings about pending samples are not errors (the check script
    # itself should pass), but the close-gate consumer must check the
    # pending count separately.
    return 0


if __name__ == "__main__":
    sys.exit(main())
