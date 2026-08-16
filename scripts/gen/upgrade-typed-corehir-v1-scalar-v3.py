#!/usr/bin/env python3
"""Upgrade a fail-closed scalar TypedCoreHIR v1 artifact to canonical v3."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.typed_corehir_v1_scalar_v3 import upgrade_scalar_document  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = json.loads(args.input.read_text(encoding="utf-8"))
    upgraded = upgrade_scalar_document(source)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(upgraded, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        "typed-corehir-v1-scalar-v3: PASS: "
        f"types={len(upgraded['types'])} functions={len(upgraded['functions'])} output={args.output}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as exc:
        print(f"typed-corehir-v1-scalar-v3: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
