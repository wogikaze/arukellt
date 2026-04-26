"""Generic file operations."""

from pathlib import Path


def repo_root() -> Path:
    """Walk up from this file to find the directory containing pyproject.toml or Cargo.toml."""
    current = Path(__file__).resolve().parent
    while True:
        if (current / "pyproject.toml").exists() or (current / "Cargo.toml").exists():
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
