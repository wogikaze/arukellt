#!/usr/bin/env python3
"""Advisory in-file test adoption report for issue #715 (always exits 0)."""
from __future__ import annotations

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

TARGETS = {
    "std": 180,
    "src/compiler": 60,
}


def count_in_file_tests(root_rel: str) -> tuple[int, int]:
    root = os.path.join(ROOT, root_rel)
    files = 0
    tests = 0
    for dirpath, _, names in os.walk(root):
        for name in names:
            if not name.endswith(".ark"):
                continue
            path = os.path.join(dirpath, name)
            with open(path, encoding="utf-8") as f:
                text = f.read()
            n = len(re.findall(r"^\s*test ", text, re.M))
            if n == 0:
                continue
            files += 1
            tests += n
    return files, tests


def main() -> int:
    print("in-file test adoption (advisory, #715)")
    for root_rel, goal in TARGETS.items():
        files, tests = count_in_file_tests(root_rel)
        status = "ok" if tests >= goal else "below-target"
        print(f"  {root_rel}: {tests} tests in {files} files (target {goal}) [{status}]")
    return 0


if __name__ == "__main__":
    sys.exit(main())
