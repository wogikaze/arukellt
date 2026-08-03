#!/usr/bin/env python3
"""Normalize comparison-only VerifiedCore emitted from source contracts."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.normalize_source_contract_profile import (  # noqa: E402
    UnsupportedSourceContractProfile,
    normalize_file,
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    normalize_file(args.input, args.output)
    print(f"normalize-source-contract-profile: PASS: output={args.output}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, UnsupportedSourceContractProfile) as exc:
        print(f"normalize-source-contract-profile: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
