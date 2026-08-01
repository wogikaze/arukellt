#!/usr/bin/env python3
"""Validate Arukellt formal-verification artifacts and release policy."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.common import ValidationError  # noqa: E402
from proof.trust import check_release_policy  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("policy", type=Path)
    args = parser.parse_args()
    try:
        mode, count = check_release_policy(args.policy)
    except (OSError, ValueError, ValidationError) as exc:
        print(f"formal-verification-gates: FAIL: {exc}", file=sys.stderr)
        return 1
    print(f"formal-verification-gates: PASS: mode={mode} artifacts={count}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
