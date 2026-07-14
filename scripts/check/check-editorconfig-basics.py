#!/usr/bin/env python3
"""Check repository whitespace rules owned by .editorconfig."""

from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from quality.checks import check_editorconfig_basics  # noqa: E402


if __name__ == "__main__":
    raise SystemExit(check_editorconfig_basics(ROOT))
