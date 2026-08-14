#!/usr/bin/env python3
"""Validate hash-bound typed VerifiedCore Phase 5 boundary evidence."""
from __future__ import annotations
import json, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]; SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path: sys.path.insert(0, str(SCRIPTS))
from proof.typed_verified_core_receipt_v5 import validate_boundary_receipt

def main() -> int:
    if len(sys.argv) != 2: raise ValueError("usage: check-typed-verified-core-boundary-receipt-v5.py RECEIPT")
    path = Path(sys.argv[1]); value = json.loads(path.read_text(encoding="utf-8")); validate_boundary_receipt(value, ROOT)
    if value.get("proof_phase") != 5 or value.get("aggregate_encoding") != "arukellt-smt-datatype-v1": raise ValueError("receipt does not bind Phase 5 datatype semantics")
    print(f"typed-verified-core-boundary-receipt-v5: PASS: files={len(value['files'])}")
    return 0

if __name__ == "__main__":
    try: raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"typed-verified-core-boundary-receipt-v5: FAIL: {exc}", file=sys.stderr); raise SystemExit(1)
