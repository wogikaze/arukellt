"""Fixture manifest loading helpers."""

from pathlib import Path


def load_manifest(manifest_path: Path) -> list[dict]:
    """Parse manifest.txt and return list of {"kind": str, "path": str}.

    Each non-comment, non-blank line is expected to be "kind:path".
    """
    entries = []
    for line in manifest_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        kind, _, path = line.partition(":")
        entries.append({"kind": kind.strip(), "path": path.strip()})
    return entries


def count_fixtures(manifest_path: Path) -> int:
    """Count non-bench entries in the manifest."""
    return sum(
        1
        for entry in load_manifest(manifest_path)
        if entry["kind"] != "bench"
    )


def disk_fixture_paths(fixtures_root: Path) -> list[str]:
    """Return sorted list of relative .ark paths from disk.

    Rules (matching verify-harness.sh logic):
    - Skip lsp_perf/ subtree.
    - Skip non-main.ark files when main.ark exists in the same directory.
    """
    entries = []
    for ark in sorted(fixtures_root.rglob("*.ark")):
        rel = ark.relative_to(fixtures_root)
        rel_str = str(rel)
        # Skip lsp_perf/ subtree
        if rel_str.startswith("lsp_perf/") or rel_str.startswith("lsp_perf" + "\\"):
            continue
        # Skip non-main.ark when main.ark exists in same dir
        if ark.name != "main.ark" and (ark.parent / "main.ark").exists():
            continue
        entries.append(rel_str)
    return entries
