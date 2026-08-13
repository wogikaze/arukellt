#!/usr/bin/env python3
"""Validate structural and semantic typing of a VerifiedCore v1 artifact."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.verified_core_typed import (  # noqa: E402
    TypedVerifiedCoreError,
    validate_typed_document,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifact", type=Path)
    args = parser.parse_args()
    try:
        value = json.loads(args.artifact.read_text(encoding="utf-8"))
        document = validate_typed_document(value)
    except (OSError, json.JSONDecodeError, ValueError, TypeError, KeyError, TypedVerifiedCoreError) as exc:
        print(f"typed-verified-core: FAIL: {exc}", file=sys.stderr)
        return 1
    print(
        "typed-verified-core: PASS: "
        f"types={len(document['types'])} functions={len(document['functions'])}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
