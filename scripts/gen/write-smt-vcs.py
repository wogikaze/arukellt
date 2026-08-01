#!/usr/bin/env python3
"""Generate fail-closed SMT-LIB verification conditions from VerifiedCore v1."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.smtlib_v1 import UnsupportedVerifiedCore, generate_smtlib_file  # noqa: E402


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--subject", type=Path, required=True)
    result.add_argument("--output", type=Path, required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        count = generate_smtlib_file(args.subject.resolve(), args.output.resolve())
    except (OSError, ValueError, KeyError, TypeError, UnsupportedVerifiedCore) as exc:
        print(f"write-smt-vcs: FAIL: {exc}", file=sys.stderr)
        return 1
    print(f"write-smt-vcs: PASS: obligations={count}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
