#!/usr/bin/env python3
"""Generate SHA-bound TrustManifest and ProofReceipt artifacts."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.solver_receipts import generate_solver_receipts  # noqa: E402


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--subject", type=Path, required=True)
    result.add_argument("--solver-output", type=Path, required=True)
    result.add_argument("--toolchain", type=Path, required=True)
    result.add_argument("--trust-manifest-output", type=Path, required=True)
    result.add_argument("--proof-receipt-output", type=Path, required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        _, receipt = generate_solver_receipts(
            args.subject.resolve(),
            args.solver_output.resolve(),
            args.toolchain.resolve(),
            args.trust_manifest_output.resolve(),
            args.proof_receipt_output.resolve(),
        )
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(f"write-solver-receipts: FAIL: {exc}", file=sys.stderr)
        return 1
    print(
        "write-solver-receipts: PASS: "
        f"status={receipt['status']} obligations={receipt['obligations']['total']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
