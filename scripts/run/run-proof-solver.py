#!/usr/bin/env python3
"""Run a configured solver and emit SHA-bound proof artifacts."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.solver_driver import run_solver_and_generate_receipts  # noqa: E402


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--subject", type=Path, required=True)
    result.add_argument("--solver-input", type=Path, required=True)
    result.add_argument("--toolchain", type=Path, required=True)
    result.add_argument("--solver-output", type=Path, required=True)
    result.add_argument("--trust-manifest-output", type=Path, required=True)
    result.add_argument("--proof-receipt-output", type=Path, required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        result = run_solver_and_generate_receipts(
            args.subject.resolve(),
            args.solver_input.resolve(),
            args.toolchain.resolve(),
            args.solver_output.resolve(),
            args.trust_manifest_output.resolve(),
            args.proof_receipt_output.resolve(),
        )
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(f"run-proof-solver: FAIL: {exc}", file=sys.stderr)
        return 1
    print(
        "run-proof-solver: "
        f"status={result.proof_status} obligations={result.obligation_count} "
        f"solver_exit={result.process_returncode}"
    )
    if result.process_returncode != 0 or result.proof_status != "proved":
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
