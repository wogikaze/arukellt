#!/usr/bin/env python3
"""Check that a release git tag matches the project release version."""

from __future__ import annotations

import argparse
import os
import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python < 3.11 fallback.
    import tomli as tomllib  # type: ignore[no-redef]


ROOT = Path(__file__).resolve().parents[2]
TAG_PREFIX = "refs/tags/"
VERSION_RE = re.compile(r"^v?([0-9]+[.][0-9]+[.][0-9]+(?:[-+][0-9A-Za-z.-]+)?)$")


def load_project_version() -> str:
    project_state = tomllib.loads((ROOT / "docs/data/project-state.toml").read_text(encoding="utf-8"))
    try:
        version = project_state["project"]["version"]
    except KeyError as exc:
        raise SystemExit(f"docs/data/project-state.toml missing project.version: {exc}") from exc
    if not isinstance(version, str) or not version:
        raise SystemExit("docs/data/project-state.toml project.version must be a non-empty string")
    return version


def ref_to_tag(ref: str) -> str | None:
    if not ref.startswith(TAG_PREFIX):
        return None
    return ref[len(TAG_PREFIX) :]


def normalize_tag(tag: str) -> str:
    match = VERSION_RE.fullmatch(tag)
    if not match:
        raise SystemExit(
            f"release tag '{tag}' must be a semantic version tag like v0.1.0 or 0.1.0"
        )
    return match.group(1)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate that the release tag matches docs/data/project-state.toml project.version."
    )
    parser.add_argument(
        "--ref",
        default=os.environ.get("GITHUB_REF", ""),
        help="Git ref to check; defaults to GITHUB_REF.",
    )
    parser.add_argument(
        "--require-tag",
        action="store_true",
        help="Fail when the supplied ref is not a tag ref.",
    )
    args = parser.parse_args()

    tag = ref_to_tag(args.ref)
    if tag is None:
        if args.require_tag:
            raise SystemExit(f"expected a tag ref, got '{args.ref or '<empty>'}'")
        print(f"release tag/version check skipped for non-tag ref: {args.ref or '<empty>'}")
        return 0

    project_version = load_project_version()
    tag_version = normalize_tag(tag)
    if tag_version != project_version:
        raise SystemExit(
            f"release tag version mismatch: tag '{tag}' resolves to '{tag_version}', "
            f"but docs/data/project-state.toml project.version is '{project_version}'"
        )

    print(f"release tag/version check passed: {tag} matches project version {project_version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
