#!/usr/bin/env python3
"""Executable smoke contracts for CLI commands advertised as guaranteed."""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CLI = ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
COMPILE_GUARD_DIR = ROOT / ".build" / "cli-guarantees"

# Two modules that both declare `pub struct Item` with different layouts. The
# backend still keys layouts by the bare type name, so the emitted module is
# invalid Wasm. The CLI must reject that artifact rather than report success.
SAMENAME_SOURCES = {
    "a.ark": """pub struct Item {
    name: String,
    count: i32,
}

pub fn make(n: i32) -> Item {
    Item { name: String_from("alpha"), count: n }
}

pub fn describe(it: Item) -> String {
    it.name
}
""",
    "b.ark": """pub struct Item {
    weight: f64,
    id: i64,
    tag: i32,
}

pub fn make(t: i32) -> Item {
    Item { weight: 2.5, id: 99, tag: t }
}

pub fn tag_of(it: Item) -> i32 {
    it.tag
}
""",
    "main.ark": """use std::host::stdio
import a
import b

fn main() {
    let x = a::make(3)
    let y = b::make(7)
    stdio::println(a::describe(x))
    stdio::println(b::tag_of(y))
    stdio::println(x.count)
}
""",
}

VALID_SOURCE = """use std::host::stdio

fn main() {
    stdio::println("ok")
}
"""


def run_cli(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run([str(CLI), *args], cwd=ROOT, text=True, capture_output=True)


def write_sources(directory: Path, sources: dict[str, str]) -> Path:
    directory.mkdir(parents=True, exist_ok=True)
    for name, source in sources.items():
        (directory / name).write_text(source, encoding="utf-8")
    return directory


def compiles_to(directory: Path, input_name: str, output_name: str) -> subprocess.CompletedProcess[str]:
    return run_cli(
        "compile",
        "--target",
        "wasm32",
        str((directory / input_name).relative_to(ROOT)),
        "-o",
        str((directory / output_name).relative_to(ROOT)),
    )


def module_validates(path: Path) -> bool:
    tool = shutil.which("wasm-tools")
    if tool is None:
        return False
    return subprocess.run([tool, "validate", str(path)], capture_output=True).returncode == 0


def assert_rejected(result: subprocess.CompletedProcess[str], artifact: Path) -> int:
    output = result.stdout + result.stderr
    if "W0004" not in output:
        print("cli compile guarantee failed: rejected module without W0004", file=sys.stderr)
        return 1
    if "compilation succeeded" in output:
        print("cli compile guarantee failed: rejected module reported success", file=sys.stderr)
        return 1
    if artifact.exists():
        print("cli compile guarantee failed: rejected module left an artifact", file=sys.stderr)
        return 1
    return 0


def check_compile_validation() -> int:
    samename = write_sources(COMPILE_GUARD_DIR / "samename", SAMENAME_SOURCES)
    invalid_artifact = samename / "main.wasm"
    invalid_artifact.unlink(missing_ok=True)
    result = compiles_to(samename, "main.ark", "main.wasm")
    if result.returncode == 0:
        # The cross-module collision is fixed: the module is then legitimate
        # and has to validate. This keeps the check meaningful once #940 is
        # closed instead of pinning it to a bug.
        if not invalid_artifact.exists() or not module_validates(invalid_artifact):
            print(
                "cli compile guarantee failed: success reported for an invalid module",
                file=sys.stderr,
            )
            return 1
    else:
        failed = assert_rejected(result, invalid_artifact)
        if failed:
            return failed

    valid = write_sources(COMPILE_GUARD_DIR / "valid", {"main.ark": VALID_SOURCE})
    valid_artifact = valid / "main.wasm"
    valid_artifact.unlink(missing_ok=True)
    result = compiles_to(valid, "main.ark", "main.wasm")
    if result.returncode != 0 or not valid_artifact.exists():
        print("cli compile guarantee failed: valid module rejected", file=sys.stderr)
        return 1
    return 0


def run(command: str) -> int:
    if command == "check":
        result = subprocess.run(
            [str(CLI), "check", "tests/fixtures/diagnostics/type_mismatch.ark"],
            cwd=ROOT,
            text=True,
            capture_output=True,
        )
        output = result.stdout + result.stderr
        if result.returncode == 0 or "E0200" not in output:
            print("cli check guarantee failed: expected non-zero E0200 diagnostic", file=sys.stderr)
            return 1
        return 0
    if command == "help":
        result = subprocess.run([str(CLI), "help"], cwd=ROOT, text=True, capture_output=True)
        if result.returncode != 0 or "Usage: arukellt <COMMAND>" not in result.stdout:
            print("cli help guarantee failed: canonical usage text missing", file=sys.stderr)
            return 1
        return 0
    if command == "compile-validates-output":
        return check_compile_validation()
    print(f"unknown CLI guarantee: {command}", file=sys.stderr)
    return 2


def main() -> int:
    default_commands = ["check", "help", "compile-validates-output"]
    commands = default_commands if len(sys.argv) == 1 or sys.argv[1] == "all" else [sys.argv[1]]
    return 1 if any(run(command) for command in commands) else 0


if __name__ == "__main__":
    raise SystemExit(main())
