#!/usr/bin/env python3
"""Independently validate the solver TrustManifest boundary receipt."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.solver_trust_boundary_receipt import (  # noqa: E402
    SolverTrustBoundaryReceiptError,
    validate_solver_trust_boundary_receipt,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("receipt", type=Path)
    args = parser.parse_args()
    try:
        document = json.loads(args.receipt.read_text(encoding="utf-8"))
        validated = validate_solver_trust_boundary_receipt(document, root=ROOT)
    except (OSError, ValueError, json.JSONDecodeError, SolverTrustBoundaryReceiptError) as exc:
        print(f"solver-trust-boundary-receipt: FAIL: {exc}", file=sys.stderr)
        return 1
    print(
        "solver-trust-boundary-receipt: PASS: "
        f"files={len(validated['files'])} primary={validated['primary_result']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
