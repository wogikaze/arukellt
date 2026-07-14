#!/usr/bin/env python3
"""Validate quality rules, tooling ownership, commands, ADRs, and CI wiring."""

from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from quality.checks import check_quality_contract  # noqa: E402


if __name__ == "__main__":
    raise SystemExit(check_quality_contract(ROOT))
