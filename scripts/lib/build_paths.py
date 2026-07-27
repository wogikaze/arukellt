"""Resolve the repository build directory (default ``<root>/.build``).

``ARUKELLT_BUILD_DIR`` overrides the build root so parallel agents that share a
checkout (or need isolated artifact trees) do not serialize on one lock or
stomp shared wasm outputs. Linked git worktrees already get a per-worktree
``.build`` when the env var is unset — callers must still pass the worktree
``root``, not the primary checkout.
"""

from __future__ import annotations

import os
from pathlib import Path

_BUILD_DIR_ENV = "ARUKELLT_BUILD_DIR"


def build_dir(root: Path) -> Path:
    """Return the absolute build directory for ``root``."""
    raw = os.environ.get(_BUILD_DIR_ENV, "").strip()
    if not raw:
        return (root / ".build").resolve()
    candidate = Path(raw).expanduser()
    if candidate.is_absolute():
        return candidate.resolve()
    return (root / candidate).resolve()


def selfhost_dir(root: Path) -> Path:
    """Return ``<build_dir>/selfhost``."""
    return build_dir(root) / "selfhost"


def runtime_lock_path(root: Path) -> Path:
    """Exclusive flock path for selfhost compile / parity serialization."""
    return build_dir(root) / "selfhost-runtime.lock"


def path_rel_to_root(root: Path, path: Path) -> str:
    """Return a path string usable as ``cwd=root`` relative when possible."""
    resolved = path.resolve()
    try:
        return resolved.relative_to(root.resolve()).as_posix()
    except ValueError:
        return str(resolved)


def selfhost_rel(root: Path, *parts: str) -> str:
    """Relative (or absolute) path under ``selfhost_dir(root)``."""
    return path_rel_to_root(root, selfhost_dir(root).joinpath(*parts))
