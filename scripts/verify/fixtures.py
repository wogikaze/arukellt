"""Fixture manifest loading helpers."""

from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 on supported development hosts
    import tomli as tomllib  # type: ignore[no-redef]


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


def _nearest_package_root(path: Path, fixtures_root: Path) -> Path | None:
    """Return the closest ancestor package (ark.toml) below ``fixtures_root``."""
    for ancestor in path.parents:
        if ancestor == fixtures_root:
            return None
        if (ancestor / "ark.toml").is_file():
            return ancestor
    return None


def _package_dependency_roots(package_root: Path) -> tuple[Path, ...]:
    """Return local dependency directories declared by the package's ark.toml.

    A nested module dir such as a mock registry (``file://./mock_reg``) holds
    package entry points, not helper modules of the package under test.
    """
    manifest = package_root / "ark.toml"
    if not manifest.is_file():
        return ()
    try:
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError):
        return ()
    roots: list[Path] = []
    registry = data.get("registry")
    if isinstance(registry, dict):
        url = registry.get("url")
        if isinstance(url, str) and url.startswith("file://"):
            roots.append((package_root / url[len("file://"):]).resolve())
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        entries = data.get(section)
        if not isinstance(entries, dict):
            continue
        for spec in entries.values():
            relative = spec.get("path") if isinstance(spec, dict) else None
            if isinstance(relative, str):
                roots.append((package_root / relative).resolve())
    return tuple(roots)


def disk_fixture_paths(fixtures_root: Path) -> list[str]:
    """Return sorted list of relative .ark paths from disk.

    Rules (matching verify-harness.sh logic):
    - Skip lsp_perf/ subtree.
    - Skip non-main.ark files when main.ark exists in the same directory.
    - Skip non-main.ark support modules nested inside a package whose root has
      main.ark, except under a locally declared dependency/registry directory.
    """
    dependency_roots: dict[Path, tuple[Path, ...]] = {}
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
        package_root = _nearest_package_root(ark, fixtures_root)
        if (
            ark.name != "main.ark"
            and package_root is not None
            and (package_root / "main.ark").is_file()
        ):
            if package_root not in dependency_roots:
                dependency_roots[package_root] = _package_dependency_roots(package_root)
            resolved = ark.resolve()
            if not any(root == resolved.parent or root in resolved.parents for root in dependency_roots[package_root]):
                continue
        entries.append(rel_str)
    return entries
