#!/usr/bin/env python3
"""Validate an Arukellt Proof IR JSON artifact."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[2]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from scripts.proof.ir import ValidationError, validate_document  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifact", type=Path)
    args = parser.parse_args()
    try:
        document = json.loads(args.artifact.read_text(encoding="utf-8"))
        validate_document(document)
    except (OSError, json.JSONDecodeError, ValidationError) as error:
        print(f"proof-ir: FAIL: {error}", file=sys.stderr)
        return 1
    print(
        "proof-ir: PASS: "
        f"{args.artifact} ({len(document['functions'])} function(s), schema v1)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
