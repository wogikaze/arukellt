#!/usr/bin/env python3
"""Validate a Proof IR, TrustManifest, and ProofReceipt as one bound result."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.trust import ValidationError, validate_bound_receipt  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("proof_ir", type=Path)
    parser.add_argument("trust_manifest", type=Path)
    parser.add_argument("receipt", type=Path)
    args = parser.parse_args()

    try:
        _, manifest, receipt = validate_bound_receipt(
            args.proof_ir, args.trust_manifest, args.receipt
        )
    except (OSError, ValueError, ValidationError) as exc:
        print(f"proof-receipt: FAIL: {exc}", file=sys.stderr)
        return 1

    obligations = receipt["obligations"]
    print(
        "proof-receipt: PASS: "
        f"status={receipt['status']} obligations={obligations['proved']}/{obligations['total']} "
        f"solver={manifest['solver']['name']}@{manifest['solver']['version']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
