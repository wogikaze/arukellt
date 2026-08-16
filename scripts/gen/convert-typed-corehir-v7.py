#!/usr/bin/env python3
"""Convert canonical TypedCoreHIR v3 into Phase 7 VerifiedCore."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.typed_corehir_v3_convert import convert_document  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    converted = convert_document(json.loads(args.input.read_text(encoding="utf-8")))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(converted, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        "convert-typed-corehir-v7: PASS: "
        f"types={len(converted['types'])} functions={len(converted['functions'])} output={args.output}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as exc:
        print(f"convert-typed-corehir-v7: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
