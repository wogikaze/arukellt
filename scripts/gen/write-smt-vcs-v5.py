#!/usr/bin/env python3
"""Generate Phase 5 SMT verification conditions from typed VerifiedCore."""
from __future__ import annotations
import argparse, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]; SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path: sys.path.insert(0, str(SCRIPTS))
from proof.smtlib_typed_v5_final import generate_typed_smtlib_file

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument("--subject", type=Path, required=True); parser.add_argument("--output", type=Path, required=True); args = parser.parse_args()
    count = generate_typed_smtlib_file(args.subject, args.output)
    print(f"phase5-smt-vcs: PASS: obligations={count} output={args.output}"); return 0

if __name__ == "__main__":
    try: raise SystemExit(main())
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(f"phase5-smt-vcs: FAIL: {exc}", file=sys.stderr); raise SystemExit(1)
