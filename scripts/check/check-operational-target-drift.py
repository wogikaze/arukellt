#!/usr/bin/env python3
"""Check for deprecated target names in operational code.

Detects old target spellings (wasm32-wasi-p1, wasm32-wasi-p2,
wasm32-wasi-p3, wasm32-freestanding) in operational scripts, tests, and
fixture files. Old names are allowed only in:
- alias contract definitions
- compatibility tests
- migration/history text
- ADR/RFC/spec/research/design/archive docs

Usage:
    check-operational-target-drift.py [--fix]
"""

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

# Use string constants to avoid --fix corrupting the patterns
_DEPRECATED = [
    "wasm32-wasi-p1",
    "wasm32-wasi-p2",
    "wasm32-wasi-p3",
    "wasm32-freestanding",
]
DEPRECATED_PATTERNS = [re.compile(r"\b" + re.escape(name) + r"\b") for name in _DEPRECATED]

# Directories where deprecated names are allowed (alias, migration, history)
ALLOWED_DIRS = {
    "docs/adr",
    "docs/rfc",
    "docs/spec",
    "docs/research",
    "docs/design",
    "docs/process",
    "docs/platform",
    "docs/history",
    "docs/language",
    "docs/stdlib",
    "docs/state",
    "docs/compiler",
    "docs/testing",
    "docs/plans",
    "docs/migration",
    "docs/benchmarks",
    "docs/data",
    "docs/playground",
    "docs/release-criteria.md",
    "extensions/arukellt-all-in-one/CHANGELOG.md",
    "tmp",
    "issues",
    "playground",
}

# Files where deprecated names are allowed (alias contract, compat test, detection)
ALLOWED_FILES = {
    "scripts/tests/test_target_contract.py",
    "scripts/check/check-docs-consistency.py",
    "scripts/check/gate-765-docs-ci-hard-gates.py",
    "scripts/check/check-operational-target-drift.py",
    "scripts/gen/generate-docs.py",
    "scripts/selfhost/checks_broken.py",
    "docs/data/project-state.toml",
    "docs/current-state.md",
    "docs/overview.html",
    "docs/debug-support.md",
    "docs/cli-reference.md",
    "docs/playground/dist/compiler-types.d.ts",
    "docs/playground/dist/t2-runner.js",
    "docs/data/release-guarantees.toml",
    "docs/data/release-guarantees.md",
    "src/compiler/main/target_contract_generated.ark",
    "extensions/arukellt-all-in-one/src/target-contract.generated.js",
}

# File extensions to scan
SCAN_EXTENSIONS = {".py", ".sh", ".ark", ".flags", ".toml", ".json", ".md", ".txt", ".diag", ".mir", ".mjs", ".ts"}


def is_allowed(rel_path: str) -> bool:
    for allowed_dir in ALLOWED_DIRS:
        if rel_path.startswith(allowed_dir):
            return True
    if rel_path in ALLOWED_FILES:
        return True
    return False


def check_drift(fix: bool) -> int:
    errors = 0
    fixed = 0

    for path in REPO_ROOT.rglob("*"):
        if not path.is_file():
            continue
        if any(s in str(path) for s in [
            ".git", ".worktrees", ".build", "__pycache__", "node_modules",
            "/target/",  # Rust build artifacts
        ]):
            continue
        if path.suffix not in SCAN_EXTENSIONS:
            continue

        try:
            rel = str(path.relative_to(REPO_ROOT))
        except ValueError:
            continue

        if is_allowed(rel):
            continue

        try:
            text = path.read_text(encoding="utf-8")
        except Exception:
            continue

        found_deprecated = []
        for pattern in DEPRECATED_PATTERNS:
            matches = pattern.findall(text)
            if matches:
                found_deprecated.extend(matches)

        if found_deprecated:
            if fix:
                # Apply replacements using constants to avoid self-corruption
                new_text = text
                new_text = new_text.replace("wasm32-wasi-p2", "wasm32-gc")
                new_text = new_text.replace("wasm32-wasi-p1", "wasm32")
                new_text = new_text.replace("wasm32-wasi-p3", "wasm32-gc")
                new_text = new_text.replace("wasm32-freestanding", "wasm32")
                if new_text != text:
                    path.write_text(new_text, encoding="utf-8")
                    fixed += 1
                    print(f"FIXED: {rel}")
            else:
                for m in set(found_deprecated):
                    print(f"DEPRECATED_TARGET: {rel} uses '{m}'")
                errors += 1

    if fix and fixed > 0:
        print(f"OPERATIONAL_TARGET_DRIFT: fixed {fixed} file(s)")
        return 0
    if errors == 0:
        print("OPERATIONAL_TARGET_DRIFT: PASS")
        return 0
    else:
        print(f"OPERATIONAL_TARGET_DRIFT: {errors} file(s) with deprecated targets")
        return 1


def main() -> int:
    parser = argparse.ArgumentParser(description="Check for deprecated target names in operational code")
    parser.add_argument("--fix", action="store_true", help="Auto-fix deprecated names")
    args = parser.parse_args()
    return check_drift(args.fix)


if __name__ == "__main__":
    sys.exit(main())
