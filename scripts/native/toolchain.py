"""Shared clang discovery for native-cpp (public runner + selfhost executor)."""

from __future__ import annotations

import os
import re
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

MINIMUM_CLANG_VERSION = 14

_CLANG_CANDIDATES = (
    "clang",
    "clang-18",
    "clang-17",
    "clang-16",
    "clang-15",
    "clang-14",
)


@dataclass(frozen=True)
class ClangToolchain:
    path: str
    version_line: str
    major_version: int

    @property
    def identity(self) -> str:
        return f"{self.path}|{self.version_line}"


def clang_candidates(*, honor_override: bool = True) -> list[str]:
    override = os.environ.get("ARUKELLT_CC", "").strip() if honor_override else ""
    if override:
        return [override]
    return list(_CLANG_CANDIDATES)


def resolve_clang(*, honor_override: bool = True) -> tuple[ClangToolchain | None, str]:
    """Return (toolchain, diagnostic). diagnostic is empty on success."""
    candidates = clang_candidates(honor_override=honor_override)
    searched: list[str] = []
    selected_path: str | None = None
    for candidate in candidates:
        found = shutil.which(candidate) if os.path.sep not in candidate else (
            candidate if Path(candidate).is_file() and os.access(candidate, os.X_OK) else None
        )
        searched.append(candidate if found is None else found)
        if found is not None:
            selected_path = found
            break
    if selected_path is None:
        requested = candidates[0] if candidates else "clang 14+"
        return None, (
            f"toolchain diagnostic: C compiler `{requested}` was not found "
            f"(searched: {', '.join(searched)}); install clang {MINIMUM_CLANG_VERSION}+ "
            f"or set ARUKELLT_CC"
        )

    result = subprocess.run(
        [selected_path, "--version"],
        capture_output=True,
        text=True,
        check=False,
    )
    version_line = (result.stdout or result.stderr).splitlines()[0] if result.returncode == 0 else ""
    if "clang" not in version_line.lower() and "clang" not in Path(selected_path).name.lower():
        return None, (
            f"toolchain diagnostic: clang {MINIMUM_CLANG_VERSION}+ is required; "
            f"`{selected_path}` does not look like clang ({version_line or 'no --version output'})"
        )
    match = re.search(r"clang version (\d+)", version_line)
    if match is None or int(match.group(1)) < MINIMUM_CLANG_VERSION:
        return None, (
            f"toolchain diagnostic: clang {MINIMUM_CLANG_VERSION}+ is required; "
            f"detected `{version_line or selected_path}` "
            f"(searched: {', '.join(searched)})"
        )
    canonical = str(Path(selected_path).resolve())
    return ClangToolchain(canonical, version_line, int(match.group(1))), ""
