"""Generic file operations."""

from pathlib import Path


def repo_root() -> Path:
    """Walk up from this file to find the Arukellt repository root."""
    current = Path(__file__).resolve().parent
    while True:
        if (current / "AGENTS.md").exists() and (current / "scripts/manager.py").exists():
            return current
        parent = current.parent
        if parent == current:
            # Reached filesystem root without finding marker — fall back to cwd
            return Path.cwd()
        current = parent


def find_files(root: Path, glob: str) -> list[Path]:
    """Return a sorted list of files matching glob under root."""
    return sorted(root.rglob(glob))


def read_text(path: Path) -> str:
    """Read a file as UTF-8 text."""
    return path.read_text(encoding="utf-8")
