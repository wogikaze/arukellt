#!/usr/bin/env python3
"""Extract and check all ```ark code blocks in docs/.

Usage:
    python3 scripts/check/check-doc-examples.py [docs-dir]

- Extracts ```ark ... ``` fenced code blocks (language tag exactly "ark").
- Skips blocks immediately preceded by an HTML comment <!-- skip-doc-check -->.
- Runs `arukellt check <snippet> --target wasm32` on each block.
- Reports failures as "FAIL: <file> block <N>: <error>".
- Exits 0 if all non-skipped blocks pass and every marker is valid.

Marker contract (issue #877 — normative example discipline):

- `<!-- skip-doc-check reason="..." owner="..." expires="YYYY-MM-DD" -->`
  is a deferral: it MUST carry a non-empty `reason`, an `owner`, and
  an exit condition — `expires="YYYY-MM-DD"` or `exit="..."`.
- `<!-- skip-doc-check kind="grammar-fragment" reason="..." -->`
  (also `kind="invalid-example"` / `kind="notation"`) marks permanent
  notation rather than deferred work: `reason` is required, owner and
  exit condition are not.
- `<!-- fixture: path.ark -->` declares the example is verified by the
  fixture harness: the path MUST exist under tests/fixtures/ and be
  registered in tests/fixtures/manifest.txt.
- An expired `expires` fails under docs/language/ (the normative
  surface owned by #877) and warns elsewhere.
"""
from __future__ import annotations

import argparse
import datetime
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # python < 3.11
    import tomli as tomllib

ROOT = Path(__file__).resolve().parent.parent.parent
TMP_DIR = ROOT / ".build" / "doc-examples"
CURRENT_SELFHOST = ROOT / ".build" / "selfhost" / "arukellt-s2.wasm"
BOOTSTRAP_SELFHOST = ROOT / "bootstrap" / "arukellt-selfhost.wasm"

BLOCK_PATTERN = re.compile(r"```ark\n(.*?)```", re.DOTALL)
SKIP_COMMENT_PATTERN = re.compile(
    r"<!--\s*skip-doc-check(?:\s+[^>]*)?\s*-->", re.IGNORECASE
)
# Fixture-backed examples are verified by the fixture harness, not the doc
# example checker. A <!-- fixture: path.ark --> comment preceding a block
# marks it as fixture-backed and skips doc-example compilation.
FIXTURE_COMMENT_PATTERN = re.compile(
    r"<!--\s*fixture:\s*[\w/.-]+\s*-->", re.IGNORECASE
)
HTML_COMMENT_PATTERN = re.compile(r"<!--.*?-->", re.DOTALL)
ATTR_PATTERN = re.compile(r'(\w+)="([^"]*)"')
DATE_PATTERN = re.compile(r"^\d{4}-\d{2}-\d{2}$")
# Skip kinds that denote permanent notation rather than deferred work.
# They are exempt from the owner/exit requirements (issue #877).
NOTATION_KINDS = {"grammar-fragment", "invalid-example", "notation"}

FIXTURE_MANIFEST = ROOT / "tests" / "fixtures" / "manifest.txt"
FIXTURES_ROOT = ROOT / "tests" / "fixtures"
NORMATIVE_DOC_PREFIX = "docs/language/"
CLASSIFICATIONS_TOML = ROOT / "docs" / "data" / "language-doc-classifications.toml"


def load_normative_docs() -> set[str]:
    """docs/language/<file> paths classified normative by ADR-018 registry.

    Expired skips hard-fail only inside the normative surface (#877);
    explanatory/transitional docs warn instead while their owners catch up.
    """
    result: set[str] = set()
    if not CLASSIFICATIONS_TOML.exists():
        return result
    data = tomllib.loads(CLASSIFICATIONS_TOML.read_text(encoding="utf-8"))
    for entry in data.get("docs", []):
        if entry.get("class") == "normative":
            result.add(NORMATIVE_DOC_PREFIX + str(entry.get("file", "")))
    return result

# Files listed in .generated-files are auto-generated and must not have
# <!-- skip-doc-check --> comments added by hand (they'd be overwritten).
# We read the manifest and skip those files entirely.
_GENERATED_FILES_MANIFEST = ROOT / ".generated-files"


def load_manifest_paths() -> set[str]:
    """Fixture paths registered in tests/fixtures/manifest.txt (any kind)."""
    paths: set[str] = set()
    if not FIXTURE_MANIFEST.exists():
        return paths
    for line in FIXTURE_MANIFEST.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or ":" not in line:
            continue
        paths.add(line.split(":", 1)[1].strip())
    return paths


def validate_marker(
    comment: str,
    rel: Path | str,
    manifest_paths: set[str],
    today: str,
    normative_docs: set[str],
    failures: list[str],
    warnings: list[str],
) -> None:
    """Validate one skip-doc-check / fixture marker against the #877 contract."""
    if "skip-doc-check" in comment:
        attrs = dict(ATTR_PATTERN.findall(comment))
        kind = attrs.get("kind", "")
        if not attrs.get("reason", "").strip():
            failures.append(f"{rel}: skip-doc-check missing non-empty reason=")
        if kind not in NOTATION_KINDS:
            if not attrs.get("owner", "").strip():
                failures.append(f"{rel}: skip-doc-check missing owner=")
            expires = attrs.get("expires", "").strip()
            if not expires and not attrs.get("exit", "").strip():
                failures.append(
                    f"{rel}: skip-doc-check missing exit condition "
                    "(expires= or exit=)"
                )
            elif expires:
                if not DATE_PATTERN.match(expires):
                    failures.append(
                        f"{rel}: skip-doc-check expires= must be YYYY-MM-DD"
                    )
                elif expires < today:
                    msg = f"{rel}: skip-doc-check expired {expires}"
                    if str(rel) in normative_docs:
                        failures.append(msg)
                    else:
                        warnings.append(msg)
    for fixture_match in re.finditer(r"fixture:\s*([\w/.-]+)", comment):
        fixture_rel = fixture_match.group(1)
        if fixture_rel not in manifest_paths:
            failures.append(
                f"{rel}: fixture: {fixture_rel} not registered in "
                "tests/fixtures/manifest.txt"
            )
        elif not (FIXTURES_ROOT / fixture_rel).exists():
            failures.append(f"{rel}: fixture: {fixture_rel} does not exist")


def validate_file_markers(
    md_path: Path,
    rel: Path | str,
    manifest_paths: set[str],
    today: str,
    normative_docs: set[str],
    failures: list[str],
    warnings: list[str],
    seen: set[tuple[str, int]],
) -> None:
    """Validate every marker comment that precedes an ```ark block.

    The marker zone mirrors the skip heuristic (last 800 chars before
    the block); each marker comment is validated once via `seen` (keyed by file and position).
    """
    text = md_path.read_text(encoding="utf-8")
    for m in BLOCK_PATTERN.finditer(text):
        zone = text[: m.start()].rstrip()[-800:]
        for cm in HTML_COMMENT_PATTERN.finditer(zone):
            key = (str(md_path), m.start() + cm.start())
            comment = cm.group(0)
            if key in seen or (
                "skip-doc-check" not in comment and "fixture:" not in comment
            ):
                continue
            seen.add(key)
            validate_marker(
                comment, rel, manifest_paths, today,
                normative_docs, failures, warnings,
            )


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
    """Locate the arukellt binary.

    Prefer the selfhost wrapper after compiler bootstrap retirement (ADR-029).
    A separately installed compiler must not shadow the repository wrapper.
    """
    import shutil

    if configured := os.environ.get("ARUKELLT_BIN"):
        return configured
    selfhost_wrapper = ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
    if selfhost_wrapper.is_file() and selfhost_wrapper.stat().st_mode & 0o111:
        return str(selfhost_wrapper)
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
        # Structured skip attrs can exceed 200 chars; scan enough preceding text.
        should_skip = bool(
            SKIP_COMMENT_PATTERN.search(preceding_stripped[-800:])
            or FIXTURE_COMMENT_PATTERN.search(preceding_stripped[-800:])
        )

        results.append((i, code, should_skip))

    return results


def check_block(
    arukellt: str, code: str, md_path: Path, block_idx: int, target: str
) -> tuple[bool, str]:
    """Run arukellt check on a code snippet. Returns (passed, error_output)."""
    TMP_DIR.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        suffix=".ark", mode="w", encoding="utf-8", delete=False, dir=TMP_DIR
    ) as f:
        f.write(code)
        tmp_path = Path(f.name)

    try:
        arg_path = str(tmp_path.relative_to(ROOT))
        env = os.environ.copy()
        if "ARUKELLT_SELFHOST_WASM" not in env:
            if CURRENT_SELFHOST.exists():
                env["ARUKELLT_SELFHOST_WASM"] = str(CURRENT_SELFHOST)
            elif BOOTSTRAP_SELFHOST.exists():
                env["ARUKELLT_SELFHOST_WASM"] = str(BOOTSTRAP_SELFHOST)
        result = subprocess.run(
            [arukellt, "check", arg_path, "--target", target],
            capture_output=True,
            text=True,
            cwd=ROOT,
            env=env,
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
        default="wasm32",
        help="Compile target passed to arukellt check (default: wasm32)",
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
    marker_failures: list[str] = []
    marker_warnings: list[str] = []
    manifest_paths = load_manifest_paths()
    today = datetime.date.today().isoformat()
    normative_docs = load_normative_docs()
    seen_markers: set[tuple[str, int]] = set()

    for md in sorted(docs_dir.rglob("*.md")):
        # Skip auto-generated files — their code blocks can't have
        # <!-- skip-doc-check --> comments added without being overwritten.
        if md.resolve() in generated_files:
            skip_count += sum(1 for _ in BLOCK_PATTERN.finditer(md.read_text(encoding="utf-8")))
            continue

        rel = md.relative_to(ROOT) if md.is_relative_to(ROOT) else md
        validate_file_markers(
            md, rel, manifest_paths, today, normative_docs,
            marker_failures, marker_warnings, seen_markers,
        )

        blocks = extract_blocks(md)
        if not blocks:
            continue

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

    for msg in marker_failures:
        print(f"MARKER-FAIL: {msg}")
    for msg in marker_warnings:
        print(f"MARKER-WARN: {msg}")
    print(
        f"\nDoc example check: {pass_count} pass, {fail_count} fail, "
        f"{skip_count} skip, {len(marker_failures)} marker-fail, "
        f"{len(marker_warnings)} marker-warn"
    )
    return 0 if fail_count == 0 and not marker_failures else 1


if __name__ == "__main__":
    sys.exit(main())
