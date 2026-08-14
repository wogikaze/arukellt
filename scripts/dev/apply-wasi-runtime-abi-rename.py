#!/usr/bin/env python3
"""Mechanical ownership migration for #819/#841.

This script is intentionally idempotent and is removed before the PR is marked
ready. It renames host-operation emitter modules to the runtime ABI namespace,
renames the network-routing flag, and moves internal host-operation spellings
from __intrinsic_* to __runtime_* without touching GC/compiler intrinsics.
"""
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WASM = ROOT / "src" / "compiler" / "wasm"
FAMILIES = ("http", "sockets", "fs", "env", "process", "stdio", "time", "random")


def rename_files() -> None:
    for path in sorted(WASM.glob("call_host*.ark")):
        target = path.with_name(path.name.replace("call_host", "call_runtime", 1))
        if target.exists():
            path.unlink()
        else:
            path.rename(target)
    for family in FAMILIES:
        for path in sorted(WASM.glob(f"intrinsic_{family}*.ark")):
            target = path.with_name(path.name.replace("intrinsic_", "runtime_abi_", 1))
            if target.exists():
                path.unlink()
            else:
                path.rename(target)


def editable_files() -> list[Path]:
    roots = [
        ROOT / "src",
        ROOT / "std",
        ROOT / "data",
        ROOT / "tests",
        ROOT / "scripts",
        ROOT / "docs" / "plans",
    ]
    suffixes = {".ark", ".py", ".sh", ".toml", ".md", ".txt", ".json", ".yml", ".yaml"}
    out: list[Path] = []
    for base in roots:
        if not base.exists():
            continue
        for path in base.rglob("*"):
            if path.is_file() and path.suffix in suffixes and path != Path(__file__).resolve():
                out.append(path)
    return out


def rewrite_text() -> None:
    replacements: list[tuple[str, str]] = [
        ("needs_arukellt_host", "needs_network_runtime"),
        ("wasm::call_host", "wasm::call_runtime"),
        ("call_host::", "call_runtime::"),
        ("call_host_", "call_runtime_"),
    ]
    for family in FAMILIES:
        replacements.extend(
            [
                (f"wasm::intrinsic_{family}", f"wasm::runtime_abi_{family}"),
                (f"intrinsic_{family}::", f"runtime_abi_{family}::"),
                (f"intrinsic_{family}_", f"runtime_abi_{family}_"),
                (f"__intrinsic_{family}", f"__runtime_{family}"),
            ]
        )
    for path in editable_files():
        try:
            old = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        new = old
        for before, after in replacements:
            new = new.replace(before, after)
        if new != old:
            path.write_text(new, encoding="utf-8")


def main() -> None:
    rename_files()
    rewrite_text()


if __name__ == "__main__":
    main()
