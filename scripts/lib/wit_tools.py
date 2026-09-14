"""Helpers for parsing WIT fixtures with the repository's official P2 inputs."""

from __future__ import annotations

import fcntl
import shutil
import subprocess
import tempfile
from pathlib import Path


_WASI_P2_WIT_DIR = (
    Path(__file__).resolve().parents[1]
    / "selfhost"
    / "wit"
    / "deps"
    / "wasi-cli-0.2.0"
    / "wit"
)
_WASI_P2_DEPS = ("clocks", "filesystem", "io", "random", "sockets")


def parse_wit_package(
    tool: str,
    wit_path: Path,
    lock_path: Path,
    *,
    timeout: int = 60,
) -> tuple[int, str]:
    """Parse one WIT file together with the checked-in WASI P2 packages."""
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="wit-package-") as temp_dir:
        package = Path(temp_dir)
        dependency_root = package / "deps"
        dependency_root.mkdir()
        shutil.copy2(wit_path, package / "main.wit")
        shutil.copytree(_WASI_P2_WIT_DIR, dependency_root / "wasi-cli")
        for dependency in _WASI_P2_DEPS:
            shutil.copytree(
                _WASI_P2_WIT_DIR / "deps" / dependency,
                dependency_root / dependency,
            )
        with lock_path.open("w", encoding="utf-8") as lock_file:
            fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
            result = subprocess.run(
                [tool, "component", "wit", str(package)],
                capture_output=True,
                text=True,
                timeout=timeout,
            )
    return result.returncode, result.stdout if result.returncode == 0 else (
        result.stderr or result.stdout
    )
