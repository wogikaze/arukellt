#!/usr/bin/env python3
"""Run the current-only fixture gate or explicit bootstrap reference parity."""
from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
ROOT = SCRIPTS.parent
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from selfhost.fixture_parity import (  # noqa: E402
    run_fixture_parity,
    run_reference_fixture_parity,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--reference",
        action="store_true",
        help="run retired pinned-vs-current parity; bootstrap refresh only",
    )
    parser.add_argument("--workers", type=int, help="parallel fixture workers")
    parser.add_argument(
        "--execute-all",
        action="store_true",
        help="execute every current fixture after compile+validate",
    )
    parser.add_argument("--filter-dir", action="append", default=[])
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    if args.workers is not None:
        if args.workers < 1:
            parser.error("--workers must be >= 1")
        os.environ["ARUKELLT_FIXTURE_WORKERS"] = str(args.workers)
    if args.execute_all:
        os.environ["ARUKELLT_FIXTURE_EXECUTE_ALL"] = "1"

    runner = run_reference_fixture_parity if args.reference else run_fixture_parity
    rc, output = runner(ROOT, args.dry_run, filter_dirs=args.filter_dir or None)
    if output:
        print(output, end="" if output.endswith("\n") else "\n")
    return rc


if __name__ == "__main__":
    raise SystemExit(main())
