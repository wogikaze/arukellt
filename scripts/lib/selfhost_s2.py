"""Resolve or build the current-source selfhost compiler wasm (s2/s3).

Gates and component interop tests use this helper so CI exercises the latest
selfhost compiler artifact instead of ``target/debug/arukellt`` or the pinned
bootstrap reference wasm.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


def repo_root() -> Path:
    current = Path(__file__).resolve().parent
    while True:
        if (current / "AGENTS.md").exists() and (current / "scripts" / "manager.py").exists():
            return current
        parent = current.parent
        if parent == current:
            return Path.cwd()
        current = parent


def s2_candidates(root: Path) -> list[Path]:
    return [
        root / ".build" / "selfhost" / "arukellt-s2.wasm",
        root / ".bootstrap-build" / "arukellt-s2.wasm",
        root / ".build" / "selfhost" / "arukellt-s3.wasm",
    ]


def resolve_s2_wasm(root: Path | None = None) -> Path | None:
    root = root or repo_root()
    for candidate in s2_candidates(root):
        if candidate.is_file():
            return candidate
    return None


def is_current_selfhost_wasm(path: str | Path) -> bool:
    name = Path(path).name
    return name in {"arukellt-s2.wasm", "arukellt-s3.wasm"}


def ensure_s2_wasm(root: Path | None = None, *, build: bool = True) -> Path:
    root = root or repo_root()
    existing = resolve_s2_wasm(root)
    if existing is not None:
        return existing
    if not build:
        raise FileNotFoundError(
            "current selfhost wasm missing; build with: "
            "python3 scripts/manager.py selfhost fixpoint --build"
        )
    pinned = root / "bootstrap" / "arukellt-selfhost.wasm"
    if not pinned.is_file():
        raise FileNotFoundError(f"pinned bootstrap missing: {pinned}")
    result = subprocess.run(
        [sys.executable, str(root / "scripts" / "manager.py"), "selfhost", "fixpoint", "--build"],
        cwd=str(root),
        capture_output=True,
        text=True,
        timeout=600,
    )
    if result.returncode != 0:
        tail = (result.stderr or result.stdout)[-1200:]
        raise RuntimeError(f"failed to build s2 selfhost wasm:\n{tail}")
    built = resolve_s2_wasm(root)
    if built is None:
        raise RuntimeError("selfhost fixpoint --build finished but s2 wasm not found")
    return built


def selfhost_wrapper(root: Path | None = None) -> Path:
    root = root or repo_root()
    wrapper = root / "scripts" / "run" / "arukellt-selfhost.sh"
    if not wrapper.is_file():
        raise FileNotFoundError(f"missing selfhost wrapper: {wrapper}")
    return wrapper


def gate_env(root: Path | None = None, *, build: bool = True) -> dict[str, str]:
    import os

    root = root or repo_root()
    env = dict(os.environ)
    wasm = ensure_s2_wasm(root, build=build)
    env["ARUKELLT_SELFHOST_WASM"] = str(wasm)
    env["ARUKELLT_BIN"] = str(selfhost_wrapper(root))
    env["INTEROP_STRICT"] = "1"
    return env


def main() -> int:
    parser = argparse.ArgumentParser(description="Resolve or build s2 selfhost wasm")
    parser.add_argument("--ensure", action="store_true", help="Build s2 if missing")
    parser.add_argument("--print-path", action="store_true", help="Print resolved wasm path")
    args = parser.parse_args()
    root = repo_root()
    try:
        wasm = ensure_s2_wasm(root, build=args.ensure) if args.ensure else resolve_s2_wasm(root)
    except (FileNotFoundError, RuntimeError) as exc:
        print(str(exc), file=sys.stderr)
        return 1
    if wasm is None:
        print(
            "error: no s2/s3 selfhost wasm found "
            "(run: python3 scripts/manager.py selfhost fixpoint --build)",
            file=sys.stderr,
        )
        return 1
    if args.print_path or args.ensure:
        print(wasm)
    return 0


if __name__ == "__main__":
    sys.exit(main())
