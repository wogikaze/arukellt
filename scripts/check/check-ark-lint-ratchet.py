#!/usr/bin/env python3
"""Reject increases in local Ark W0011 findings relative to a git base."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from quality.checks import run_lint_ratchet  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="HEAD")
    parser.add_argument("paths", nargs="+")
    args = parser.parse_args()
    return run_lint_ratchet(ROOT, args.paths, args.base, False, False)


if __name__ == "__main__":
    raise SystemExit(main())
