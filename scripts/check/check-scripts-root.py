#!/usr/bin/env python3
"""Check that only manager.py and README.md are in scripts/ root directory."""

import sys
from pathlib import Path

REPO_ROOT = Path.cwd()
SCRIPTS_DIR = REPO_ROOT / "scripts"

ALLOWED_FILES = {"manager.py", "README.md"}

def check_scripts_root():
    """Check that only allowed files exist in scripts/ root."""
    files = set(f.name for f in SCRIPTS_DIR.iterdir() if f.is_file())
    files.discard(".generated-files")  # Ignore this file
    
    unexpected = files - ALLOWED_FILES
    
    if unexpected:
        print(f"ERROR: Unexpected files in scripts/ root directory:", file=sys.stderr)
        for f in sorted(unexpected):
            print(f"  - {f}", file=sys.stderr)
        print(f"\nAllowed files in scripts/ root: {', '.join(sorted(ALLOWED_FILES))}", file=sys.stderr)
        print("Move utility scripts to scripts/util/, benchmark scripts to scripts/perf/, etc.", file=sys.stderr)
        return False
    
    return True

if __name__ == "__main__":
    if check_scripts_root():
        sys.exit(0)
    else:
        sys.exit(1)
