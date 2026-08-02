#!/usr/bin/env python3
"""Convert TypedCoreHIR v1 into semantically typed VerifiedCore v1."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.common import load_json  # noqa: E402
from proof.typed_corehir_convert import UnsupportedTypedCoreHir, convert_document  # noqa: E402
from proof.verified_core_typed import (  # noqa: E402
    TypedVerifiedCoreError,
    validate_typed_document,
)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--input", type=Path, required=True)
    result.add_argument("--output", type=Path, required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        converted = convert_document(load_json(args.input.resolve()))
        validate_typed_document(converted)
        output = args.output.resolve()
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(
            json.dumps(converted, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    except (
        OSError,
        ValueError,
        TypeError,
        KeyError,
        UnsupportedTypedCoreHir,
        TypedVerifiedCoreError,
    ) as exc:
        print(f"convert-typed-corehir: FAIL: {exc}", file=sys.stderr)
        return 1
    print(
        "convert-typed-corehir: PASS: "
        f"types={len(converted['types'])} functions={len(converted['functions'])} "
        "typed_semantics=validated"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
