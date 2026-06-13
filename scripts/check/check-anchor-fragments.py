#!/usr/bin/env python3
"""Validate markdown anchor fragments in docs and issues.

Checks relative links of the form ``path.md#anchor`` and same-file ``#anchor``
references against GFM heading slugs (ADR-019 §1.1) and explicit ``<a id="">``
anchors (ADR-019 §1.3).

Skips external URLs (http/https/mailto), Docsify router paths (#/...), and
pseudo placeholder references handled by check-links.sh.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
ALLOWLIST_PATH = Path(__file__).resolve().parent / "anchor-allowlist.txt"

SCAN_DIRS = ("docs", "issues")
SCAN_FILES = ("README.md", "AGENTS.md")

LINK_PATTERN = re.compile(r"\]\(([^)]+)\)")
HEADING_PATTERN = re.compile(r"^(#{2,6})\s+(.+?)\s*$", re.MULTILINE)
EXPLICIT_ID_PATTERN = re.compile(r"""<a\s+id=["']([^"']+)["']""", re.IGNORECASE)

SKIP_PATH_SUBSTRINGS = ('"', ": ", "NNN", "...")

errors: list[str] = []


def gfm_slug(text: str) -> str:
    """Generate a GFM/Docsify heading anchor slug (ADR-019 §1.1)."""
    slug = text.lower().replace(" ", "-")
    slug = re.sub(r"[^a-z0-9-]", "", slug)
    return slug


def build_anchor_set(content: str) -> set[str]:
    """Collect valid anchor ids from explicit tags and ##-###### headings."""
    anchors: set[str] = set()

    for match in EXPLICIT_ID_PATTERN.finditer(content):
        anchors.add(match.group(1))

    slug_counts: dict[str, int] = {}
    for match in HEADING_PATTERN.finditer(content):
        heading = match.group(2).strip()
        heading = re.sub(r"<!--.*?-->", "", heading).strip()
        heading = re.sub(r"\s+#+\s*$", "", heading).strip()
        base = gfm_slug(heading)
        if not base:
            continue
        count = slug_counts.get(base, 0)
        slug = base if count == 0 else f"{base}-{count}"
        slug_counts[base] = count + 1
        anchors.add(slug)

    return anchors


def load_allowlist() -> set[tuple[str, str]]:
    """Return allowlisted (relative-md-path, anchor) pairs."""
    allowed: set[tuple[str, str]] = set()
    if not ALLOWLIST_PATH.exists():
        return allowed
    for line in ALLOWLIST_PATH.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if ":" not in line:
            continue
        path_part, anchor = line.split(":", 1)
        allowed.add((path_part.strip(), anchor.strip()))
    return allowed


def should_skip_ref(ref: str) -> bool:
    ref = ref.strip()
    if not ref:
        return True
    lowered = ref.lower()
    if lowered.startswith(("http://", "https://", "mailto:", "data:")):
        return True
    if ref.startswith("#/"):
        return True
    if "#" not in ref:
        return True
    path_part = ref.split("#", 1)[0]
    if path_part and any(token in path_part for token in SKIP_PATH_SUBSTRINGS):
        return True
    return False


def normalize_ref(ref: str) -> str:
    ref = ref.strip()
    if " " in ref:
        ref = ref.split(" ", 1)[0]
    if "?" in ref:
        ref = ref.split("?", 1)[0]
    return ref


def resolve_target(md_file: Path, path_part: str) -> Path | None:
    if not path_part:
        return md_file
    candidate = (md_file.parent / path_part).resolve()
    if candidate.exists():
        return candidate
    root_candidate = (ROOT / path_part).resolve()
    if root_candidate.exists():
        return root_candidate
    return None


def iter_markdown_files() -> list[Path]:
    files: list[Path] = []
    for directory in SCAN_DIRS:
        root_dir = ROOT / directory
        if root_dir.is_dir():
            files.extend(sorted(root_dir.rglob("*.md")))
    for name in SCAN_FILES:
        path = ROOT / name
        if path.is_file():
            files.append(path)
    return files


def check_file(md_file: Path, allowlist: set[tuple[str, str]]) -> None:
    rel_md = md_file.relative_to(ROOT).as_posix()
    content = md_file.read_text(encoding="utf-8")
    local_anchors = build_anchor_set(content)
    anchor_cache: dict[Path, set[str]] = {md_file: local_anchors}

    for match in LINK_PATTERN.finditer(content):
        ref = normalize_ref(match.group(1))
        if should_skip_ref(ref):
            continue

        if ref.startswith("#"):
            fragment = ref[1:]
            if not fragment:
                continue
            if fragment not in local_anchors:
                if (rel_md, fragment) not in allowlist:
                    errors.append(f"  BROKEN ANCHOR: {rel_md} -> #{fragment}")
            continue

        path_part, fragment = ref.split("#", 1)
        if not fragment:
            continue

        target = resolve_target(md_file, path_part)
        if target is None:
            continue

        if target not in anchor_cache:
            anchor_cache[target] = build_anchor_set(
                target.read_text(encoding="utf-8")
            )

        rel_target = target.relative_to(ROOT).as_posix()
        if fragment not in anchor_cache[target]:
            if (rel_target, fragment) not in allowlist and (rel_md, fragment) not in allowlist:
                errors.append(
                    f"  BROKEN ANCHOR: {rel_md} -> {path_part}#{fragment}"
                )


def main() -> int:
    allowlist = load_allowlist()
    files = iter_markdown_files()

    print("=== Anchor Fragment Checker ===")
    print("")
    for scan_dir in SCAN_DIRS:
        print(f"Checking {scan_dir}/**/*.md ...")
    for name in SCAN_FILES:
        if (ROOT / name).is_file():
            print(f"Checking {name} ...")
    print("")

    for md_file in files:
        check_file(md_file, allowlist)

    if errors:
        print(f"=== FAILED: {len(errors)} broken anchor(s) in {len(files)} files ===")
        for err in errors:
            print(err, file=sys.stderr)
        return 1

    print(f"=== OK: {len(files)} files checked, no broken anchors ===")
    return 0


if __name__ == "__main__":
    sys.exit(main())
