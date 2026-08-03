#!/usr/bin/env python3
"""Write hash-bound evidence for the manifest-carrying solver result boundary."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.common import sha256_file  # noqa: E402
from proof.solver_trust_boundary_receipt import (  # noqa: E402
    REQUIRED_CAPABILITIES,
    REQUIRED_PRODUCERS,
    validate_solver_trust_boundary_receipt,
)

BOUNDARY_FILES = (
    "scripts/proof/solver_result.py",
    "scripts/proof/solver_driver.py",
    "scripts/proof/solver_receipts.py",
    "scripts/proof/solver_trust_boundary_receipt.py",
    "scripts/proof/trust.py",
    "scripts/run/run-proof-solver.py",
    "scripts/gen/write-solver-receipts.py",
    "scripts/gen/write-solver-trust-boundary-receipt.py",
    "scripts/check/check-solver-result.py",
    "scripts/check/check-solver-trust-manifest-boundary.py",
    "scripts/check/check-solver-trust-boundary-receipt.py",
    "scripts/tests/test_solver_receipts.py",
    "scripts/tests/test_solver_result.py",
    "scripts/tests/test_solver_trust_boundary_receipt.py",
    "scripts/tests/test_solver_driver.py",
    ".github/workflows/solver-bound-proof-receipts.yml",
    ".github/workflows/proof-solver-driver.yml",
    "tests/proof/toolchain/toolchain.json",
    "release/proof-policy.json",
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / ".build" / "proof" / "solver-trust-boundary.json",
    )
    args = parser.parse_args()

    files: list[dict[str, str]] = []
    for relative in BOUNDARY_FILES:
        path = ROOT / relative
        if not path.is_file():
            raise ValueError(f"solver trust boundary file missing: {relative}")
        files.append({"path": relative, "sha256": sha256_file(path)})

    document = {
        "schema": "arukellt-solver-trust-boundary",
        "schema_version": 1,
        "status": "enforced",
        "primary_result": "arukellt-solver-result@1",
        "raw_solver_output_role": "evidence-only",
        "trust_manifest_policy": "embedded-and-file-bound",
        "proof_receipt_policy": "embedded-and-file-bound",
        "capabilities": sorted(REQUIRED_CAPABILITIES),
        "public_producers": sorted(REQUIRED_PRODUCERS),
        "failure_action": "no-valid-solver-result",
        "files": files,
    }
    validate_solver_trust_boundary_receipt(document, root=ROOT)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        "solver-trust-boundary-receipt: PASS: "
        f"files={len(files)} output={args.output}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"solver-trust-boundary-receipt: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
