#!/usr/bin/env python3
"""Independently validate a complete manifest-carrying solver result."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.solver_result import (  # noqa: E402
    SolverResultError,
    validate_solver_result_file,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result", type=Path)
    parser.add_argument("--subject", type=Path, required=True)
    parser.add_argument("--solver-input", type=Path, required=True)
    parser.add_argument("--toolchain", type=Path, required=True)
    parser.add_argument("--solver-output", type=Path, required=True)
    parser.add_argument("--trust-manifest", type=Path, required=True)
    parser.add_argument("--proof-receipt", type=Path, required=True)
    args = parser.parse_args()
    try:
        document = validate_solver_result_file(
            args.result.resolve(),
            subject_path=args.subject.resolve(),
            solver_input_path=args.solver_input.resolve(),
            toolchain_path=args.toolchain.resolve(),
            solver_output_path=args.solver_output.resolve(),
            trust_manifest_path=args.trust_manifest.resolve(),
            proof_receipt_path=args.proof_receipt.resolve(),
        )
    except (OSError, ValueError, KeyError, TypeError, SolverResultError) as exc:
        print(f"solver-result: FAIL: {exc}", file=sys.stderr)
        return 1
    print(
        "solver-result: PASS: "
        f"status={document['status']} obligations={document['obligations']['total']} "
        f"manifest=embedded receipt=embedded"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
