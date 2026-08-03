#!/usr/bin/env python3
"""Convert captured solver output into a complete manifest-carrying result."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.solver_receipts import generate_solver_receipts  # noqa: E402
from proof.solver_result import (  # noqa: E402
    create_solver_result,
    validate_solver_result_file,
    write_solver_result,
)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--subject", type=Path, required=True)
    result.add_argument("--solver-input", type=Path, required=True)
    result.add_argument("--solver-output", type=Path, required=True)
    result.add_argument("--toolchain", type=Path, required=True)
    result.add_argument("--trust-manifest-output", type=Path, required=True)
    result.add_argument("--proof-receipt-output", type=Path, required=True)
    result.add_argument("--solver-result-output", type=Path, required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    subject = args.subject.resolve()
    solver_input = args.solver_input.resolve()
    solver_output = args.solver_output.resolve()
    toolchain = args.toolchain.resolve()
    manifest_output = args.trust_manifest_output.resolve()
    receipt_output = args.proof_receipt_output.resolve()
    result_output = args.solver_result_output.resolve()
    try:
        _, receipt = generate_solver_receipts(
            subject,
            solver_output,
            toolchain,
            manifest_output,
            receipt_output,
        )
        document = create_solver_result(
            subject_path=subject,
            solver_input_path=solver_input,
            toolchain_path=toolchain,
            solver_output_path=solver_output,
            trust_manifest_path=manifest_output,
            proof_receipt_path=receipt_output,
            execution_mode="captured-output",
            process_returncode=None,
            timed_out=False,
        )
        write_solver_result(document, result_output)
        validate_solver_result_file(
            result_output,
            subject_path=subject,
            solver_input_path=solver_input,
            toolchain_path=toolchain,
            solver_output_path=solver_output,
            trust_manifest_path=manifest_output,
            proof_receipt_path=receipt_output,
        )
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(f"write-solver-receipts: FAIL: {exc}", file=sys.stderr)
        return 1
    print(
        "write-solver-receipts: PASS: "
        f"status={receipt['status']} obligations={receipt['obligations']['total']} "
        f"result={result_output}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
