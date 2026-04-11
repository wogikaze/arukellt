#!/usr/bin/env python3
"""Extract and check all ```ark code blocks in docs/.

Usage:
    python3 scripts/check/check-doc-examples.py [docs-dir]

- Extracts ```ark ... ``` fenced code blocks (language tag exactly "ark").
- Skips blocks immediately preceded by an HTML comment <!-- skip-doc-check -->.
- Runs `arukellt check <snippet> --target wasm32-wasi-p1` on each block.
- Reports failures as "FAIL: <file> block <N>: <error>".
- Exits 0 if all non-skipped blocks pass, non-zero otherwise.
"""
from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent

BLOCK_PATTERN = re.compile(r"```ark\n(.*?)```", re.DOTALL)
SKIP_COMMENT_PATTERN = re.compile(r"<!--\s*skip-doc-check\s*-->")

# Files listed in .generated-files are auto-generated and must not have
# <!-- skip-doc-check --> comments added by hand (they'd be overwritten).
# We read the manifest and skip those files entirely.
_GENERATED_FILES_MANIFEST = ROOT / ".generated-files"


def load_generated_files() -> set[Path]:
    """Return the set of generated file paths (absolute) from .generated-files."""
    result: set[Path] = set()
    if not _GENERATED_FILES_MANIFEST.exists():
        return result
    for line in _GENERATED_FILES_MANIFEST.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        rel_path = line.split("|")[0].strip()
        result.add((ROOT / rel_path).resolve())
    return result


def find_arukellt() -> str:
    """Locate the arukellt binary."""
    import shutil

    if configured := os.environ.get("ARUKELLT_BIN"):
        return configured
    for candidate in [
        ROOT / "target" / "debug" / "arukellt",
        ROOT / "target" / "release" / "arukellt",
    ]:
        if candidate.is_file() and candidate.stat().st_mode & 0o111:
            return str(candidate)
    if found := shutil.which("arukellt"):
        return found
    return "arukellt"  # fall back; will produce a clear error


def extract_blocks(md_path: Path) -> list[tuple[int, str, bool]]:
    """Return list of (block_index, code, should_skip) for each ```ark block."""
    text = md_path.read_text(encoding="utf-8")
    results: list[tuple[int, str, bool]] = []

    for i, m in enumerate(BLOCK_PATTERN.finditer(text)):
        code = m.group(1)

        # Check whether the immediately preceding non-whitespace content is
        # a skip-doc-check HTML comment.
        preceding = text[: m.start()]
        # Strip trailing whitespace/newlines to find the last meaningful token.
        preceding_stripped = preceding.rstrip()
        should_skip = bool(SKIP_COMMENT_PATTERN.search(preceding_stripped[-200:]))

        results.append((i, code, should_skip))

    return results


def check_block(
    arukellt: str, code: str, md_path: Path, block_idx: int, target: str
) -> tuple[bool, str]:
    """Run arukellt check on a code snippet. Returns (passed, error_output)."""
    with tempfile.NamedTemporaryFile(
        suffix=".ark", mode="w", encoding="utf-8", delete=False
    ) as f:
        f.write(code)
        tmp_path = Path(f.name)

    try:
        result = subprocess.run(
            [arukellt, "check", str(tmp_path), "--target", target],
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            return True, ""
        error_output = (result.stderr or result.stdout).strip()
        return False, error_output
    finally:
        tmp_path.unlink(missing_ok=True)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Check ```ark code blocks in docs/ with arukellt check."
    )
    parser.add_argument(
        "docs_dir",
        nargs="?",
        default=str(ROOT / "docs"),
        help="Directory to scan for .md files (default: docs/)",
    )
    parser.add_argument(
        "--target",
        default="wasm32-wasi-p1",
        help="Compile target passed to arukellt check (default: wasm32-wasi-p1)",
    )
    args = parser.parse_args()

    docs_dir = Path(args.docs_dir)
    if not docs_dir.is_dir():
        print(f"error: docs directory not found: {docs_dir}", file=sys.stderr)
        return 2

    arukellt = find_arukellt()
    generated_files = load_generated_files()

    pass_count = 0
    fail_count = 0
    skip_count = 0

    for md in sorted(docs_dir.rglob("*.md")):
        # Skip auto-generated files — their code blocks can't have
        # <!-- skip-doc-check --> comments added without being overwritten.
        if md.resolve() in generated_files:
            skip_count += sum(1 for _ in BLOCK_PATTERN.finditer(md.read_text(encoding="utf-8")))
            continue

        blocks = extract_blocks(md)
        if not blocks:
            continue

        rel = md.relative_to(ROOT) if md.is_relative_to(ROOT) else md

        for idx, code, should_skip in blocks:
            if should_skip:
                skip_count += 1
                continue

            passed, error_output = check_block(arukellt, code, md, idx, args.target)
            if passed:
                pass_count += 1
            else:
                fail_count += 1
                # Report first line of error as the inline summary, full output below
                first_line = error_output.splitlines()[0] if error_output else "(no output)"
                print(f"FAIL: {rel} block {idx}: {first_line}")
                if error_output:
                    # Indent continuation lines for readability
                    for line in error_output.splitlines():
                        print(f"  {line}")

    print(
        f"\nDoc example check: {pass_count} pass, {fail_count} fail, {skip_count} skip"
    )
    return 0 if fail_count == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
