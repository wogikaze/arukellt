#!/usr/bin/env python3
"""Generate Phase 7 SMT-LIB VCs from semantically admitted VerifiedCore."""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.smtlib_typed_v7 import (  # noqa: E402
    UnsupportedTypedVerifiedCore,
    generate_typed_smtlib_file,
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--subject", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        count = generate_typed_smtlib_file(args.subject.resolve(), args.output.resolve())
    except (OSError, ValueError, KeyError, TypeError, UnsupportedTypedVerifiedCore) as exc:
        print(f"write-smt-vcs-v7: FAIL: {exc}", file=sys.stderr)
        return 1
    print(f"write-smt-vcs-v7: PASS: obligations={count} phase7_semantics=validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
