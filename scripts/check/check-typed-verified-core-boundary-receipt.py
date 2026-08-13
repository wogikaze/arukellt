#!/usr/bin/env python3
"""Validate typed VerifiedCore boundary receipt and all bound files."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.typed_verified_core_receipt import (  # noqa: E402
    TypedVerifiedCoreReceiptError,
    validate_boundary_receipt,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("receipt", type=Path)
    args = parser.parse_args()
    try:
        value = json.loads(args.receipt.read_text(encoding="utf-8"))
        document = validate_boundary_receipt(value, root=ROOT)
    except (
        OSError,
        json.JSONDecodeError,
        ValueError,
        TypeError,
        TypedVerifiedCoreReceiptError,
    ) as exc:
        print(f"typed-verified-core-boundary-receipt: FAIL: {exc}", file=sys.stderr)
        return 1
    print(
        "typed-verified-core-boundary-receipt: PASS: "
        f"files={len(document['files'])} semantic_checks={len(document['semantic_checks'])}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
