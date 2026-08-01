#!/usr/bin/env python3
"""Fail closed when a proof-required release lacks valid proved receipts."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.trust import ValidationError, check_release_policy  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("policy", type=Path)
    args = parser.parse_args()

    try:
        mode, artifact_count = check_release_policy(args.policy)
    except (OSError, ValueError, ValidationError) as exc:
        print(f"proof-release: FAIL: {exc}", file=sys.stderr)
        return 1

    print(f"proof-release: PASS: mode={mode} receipts={artifact_count}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
