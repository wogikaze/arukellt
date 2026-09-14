"""Resolve external tool binaries without accepting same-name shadow scripts."""

from __future__ import annotations

import os
import shutil
import subprocess


def _is_bytecodealliance_wasm_tools(candidate: str) -> bool:
    """Return whether ``candidate`` is the Component Model wasm-tools CLI."""
    try:
        result = subprocess.run(
            [candidate, "validate", "--help"],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return False
    help_text = (result.stdout or "") + (result.stderr or "")
    return result.returncode == 0 and "--features" in help_text


def find_wasm_tools() -> str | None:
    """Return a usable Bytecode Alliance ``wasm-tools`` executable.

    Some environments expose an unrelated Python program with the same
    executable name ahead of the official CLI. Check the validation help
    surface before selecting a PATH entry so component gates do not silently
    run the wrong tool.
    """
    candidates: list[str] = []
    requested = os.environ.get("ARUKELLT_WASM_TOOLS_BIN", "").strip()
    if requested:
        resolved = shutil.which(requested)
        candidates.append(resolved or requested)

    discovered = shutil.which("wasm-tools")
    if discovered:
        candidates.append(discovered)

    for entry in os.environ.get("PATH", "").split(os.pathsep):
        if entry:
            candidates.append(os.path.join(entry, "wasm-tools"))

    seen: set[str] = set()
    for candidate in candidates:
        if candidate in seen:
            continue
        seen.add(candidate)
        if _is_bytecodealliance_wasm_tools(candidate):
            return candidate
    return None
