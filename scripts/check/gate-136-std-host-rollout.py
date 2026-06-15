#!/usr/bin/env python3
"""Close gate for issue #136 — ADR-011 std::host::* rollout consistency."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
OPEN_DIR = REPO_ROOT / "issues" / "open"
DONE_DIR = REPO_ROOT / "issues" / "done"

# ADR-011 §2 planned host modules (module stem → manifest/doc name)
ADR011_MODULES = (
    "stdio",
    "fs",
    "env",
    "process",
    "clock",
    "random",
    "http",
    "sockets",
)

DONE_UPSTREAM = ("137", "138")
SLICED_PARENTS = {
    "077": ("655", "656"),
    "139": ("657", "658"),
}
SLICE_DONE_MIN = ("655", "657")


def _issue_path(issue_id: str) -> Path | None:
    for directory in (DONE_DIR, OPEN_DIR):
        for path in directory.glob(f"{issue_id}-*.md"):
            return path
    return None


def _issue_in_done(issue_id: str) -> bool:
    path = _issue_path(issue_id)
    return path is not None and path.parent == DONE_DIR


def main() -> int:
    failures: list[str] = []

    manifest_path = REPO_ROOT / "std" / "manifest.toml"
    if not manifest_path.is_file():
        failures.append("missing std/manifest.toml")
        manifest_text = ""
    else:
        manifest_text = manifest_path.read_text(encoding="utf-8")

    for stem in ADR011_MODULES:
        mod = f"std::host::{stem}"
        ark = REPO_ROOT / "std" / "host" / f"{stem}.ark"
        if not ark.is_file():
            failures.append(f"missing std/host/{stem}.ark")
        if mod not in manifest_text:
            failures.append(f"manifest.toml missing {mod}")

    cap_surface = REPO_ROOT / "docs" / "capability-surface.md"
    if not cap_surface.is_file():
        failures.append("missing docs/capability-surface.md")
    else:
        cap_text = cap_surface.read_text(encoding="utf-8")
        for stem in ADR011_MODULES:
            if f"std::host::{stem}" not in cap_text:
                failures.append(f"capability-surface.md missing std::host::{stem}")
        if "## Runtime verification" not in cap_text:
            failures.append("capability-surface.md missing Runtime verification section")

    for page in ("io.md", "http.md", "sockets.md"):
        doc = REPO_ROOT / "docs" / "stdlib" / "modules" / page
        if not doc.is_file():
            failures.append(f"missing docs/stdlib/modules/{page}")

    for issue_id in DONE_UPSTREAM:
        if not _issue_in_done(issue_id):
            failures.append(f"upstream #{issue_id} not in issues/done/")

    for parent_id, (slice_a, slice_b) in SLICED_PARENTS.items():
        both_slices_done = _issue_in_done(slice_a) and _issue_in_done(slice_b)
        parent = _issue_path(parent_id)
        if parent is None:
            failures.append(f"missing parent issue #{parent_id}")
        elif both_slices_done:
            if parent.parent != DONE_DIR:
                failures.append(
                    f"parent #{parent_id} expected done umbrella in issues/done/ "
                    f"when #{slice_a} and #{slice_b} are closed"
                )
        elif parent.parent != OPEN_DIR:
            failures.append(f"parent #{parent_id} expected open umbrella in issues/open/")
        if not _issue_in_done(slice_a):
            failures.append(f"slice #{slice_a} (child of #{parent_id}) not in issues/done/")

    for slice_id in SLICE_DONE_MIN:
        path = _issue_path(slice_id)
        if path is None:
            continue
        text = path.read_text(encoding="utf-8")
        if "Status: done" not in text.split("---", 2)[1]:
            failures.append(f"slice #{slice_id} frontmatter not Status: done")

    adr = REPO_ROOT / "docs" / "adr" / "ADR-011-wasi-host-layering.md"
    if adr.is_file():
        adr_text = adr.read_text(encoding="utf-8")
        for stem in ADR011_MODULES:
            if f"std::host::{stem}" not in adr_text:
                failures.append(f"ADR-011 missing std::host::{stem}")

    gate138 = REPO_ROOT / "scripts" / "check" / "gate-138-shared-capabilities-t1-t3.py"
    if gate138.is_file():
        result = subprocess.run(
            [sys.executable, str(gate138)],
            cwd=str(REPO_ROOT),
            capture_output=True,
            text=True,
            timeout=60,
        )
        if result.returncode != 0:
            failures.append("gate-138-shared-capabilities-t1-t3.py failed")
    else:
        failures.append("missing gate-138-shared-capabilities-t1-t3.py")

    open_index = REPO_ROOT / "issues" / "open" / "index.md"
    if open_index.is_file():
        index_text = open_index.read_text(encoding="utf-8")
        if re.search(r"\b136\b.*std.*host", index_text, re.IGNORECASE):
            failures.append("issues/open/index.md still lists #136 as open")

    if failures:
        print("gate-136-std-host-rollout: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1

    print("gate-136-std-host-rollout: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
