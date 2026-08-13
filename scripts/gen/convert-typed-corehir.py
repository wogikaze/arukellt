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
from proof.typed_corehir_typed_convert import (  # noqa: E402
    ExplicitTypedCoreHirError,
    convert_typed_document,
)
from proof.verified_core_typed import TypedVerifiedCoreError  # noqa: E402


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--input", type=Path, required=True)
    result.add_argument("--output", type=Path, required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        converted = convert_typed_document(load_json(args.input.resolve()))
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
        ExplicitTypedCoreHirError,
        TypedVerifiedCoreError,
    ) as exc:
        print(f"convert-typed-corehir: FAIL: {exc}", file=sys.stderr)
        return 1
    print(
        "convert-typed-corehir: PASS: "
        f"types={len(converted['types'])} functions={len(converted['functions'])} "
        "explicit_types=validated typed_semantics=validated"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
