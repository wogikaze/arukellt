#!/usr/bin/env python3
"""Component WIT parse gate (issue #117).

For each ``component-wit-parse:`` manifest entry:
  1. Require a sibling ``.expected.wit`` golden beside the ``.ark`` fixture.
  2. Run ``wasm-tools component wit`` on the golden (parse round-trip).
  3. Assert option/result/tuple surface markers and kebab-case export names.
  4. Best-effort: compile ``--emit wit`` and diff when the selfhost compiler
     returns non-empty output (skipped under bootstrap component stub).
"""

from __future__ import annotations

import fcntl
import os
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
_SCRIPTS_DIR = REPO_ROOT / "scripts"
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))
MANIFEST = REPO_ROOT / "tests" / "fixtures" / "manifest.txt"
_WASM_TOOLS_LOCK = REPO_ROOT / ".build" / "wasm-tools-component.lock"

REQUIRED_MARKERS = (
    "option<",
    "result<",
    "tuple<",
    "maybe-double",
    "safe-div",
    "swap-pair",
)


def _find_tool(name: str) -> str | None:
    if name == "wasm-tools":
        cargo = Path.home() / ".cargo" / "bin" / "wasm-tools"
        if cargo.is_file():
            return str(cargo)
    return shutil.which(name)


def _compiler() -> Path | None:
    wrapper = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
    if wrapper.is_file():
        return wrapper
    for candidate in (
        REPO_ROOT / "target" / "release" / "arukellt",
        REPO_ROOT / "target" / "debug" / "arukellt",
    ):
        if candidate.is_file():
            return candidate
    return None


def _selfhost_compile_env(*, build: bool = True) -> dict[str, str]:
    from lib.selfhost_s2 import ensure_s2_wasm, gate_env

    if build:
        return gate_env(REPO_ROOT, build=True)
    env = dict(os.environ)
    wasm = ensure_s2_wasm(REPO_ROOT, build=False)
    env["ARUKELLT_SELFHOST_WASM"] = str(wasm)
    return env


def _uses_s2_selfhost(env: dict[str, str]) -> bool:
    from lib.selfhost_s2 import is_current_selfhost_wasm

    wasm = env.get("ARUKELLT_SELFHOST_WASM", "")
    return is_current_selfhost_wasm(wasm) if wasm else False


def _load_manifest_entries(kind: str) -> list[str]:
    entries: list[str] = []
    for line in MANIFEST.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        entry_kind, _, path = line.partition(":")
        if entry_kind.strip() == kind:
            entries.append(path.strip())
    return entries


def _normalize_wit(text: str) -> str:
    lines = [line.rstrip() for line in text.strip().splitlines()]
    return "\n".join(line for line in lines if line) + "\n"


def _wasm_tools_parse_wit(tool: str, wit_path: Path) -> tuple[int, str]:
    _WASM_TOOLS_LOCK.parent.mkdir(parents=True, exist_ok=True)
    with _WASM_TOOLS_LOCK.open("w", encoding="utf-8") as lock_file:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        result = subprocess.run(
            [tool, "component", "wit", str(wit_path)],
            capture_output=True,
            text=True,
            timeout=60,
        )
    if result.returncode != 0:
        return result.returncode, (result.stderr or result.stdout)[-800:]
    return 0, result.stdout


def _try_emit_wit(
    compiler: Path,
    fixture_rel: str,
    out_rel: str,
    env: dict[str, str] | None = None,
) -> tuple[int, str]:
    cmd = [
        str(compiler),
        "compile",
        fixture_rel,
        "--target",
        "wasm32-gc",
        "--emit",
        "wit",
        "-o",
        out_rel,
    ]
    if compiler.name == "arukellt-selfhost.sh":
        cmd = ["bash", str(compiler), *cmd[1:]]
    result = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
        env=_selfhost_compile_env(build=False) if env is None else env,
    )
    if result.returncode != 0:
        tail = (result.stderr or result.stdout)[-800:]
        return 1, f"compile failed: {tail}"
    out = REPO_ROOT / out_rel
    if not out.is_file() or out.stat().st_size == 0:
        return 2, "bootstrap component stub returned empty WIT (golden-only gate)"
    return 0, out.read_text(encoding="utf-8")


def main() -> int:
    if not MANIFEST.is_file():
        print(f"error: missing {MANIFEST}", file=sys.stderr)
        return 1

    entries = _load_manifest_entries("component-wit-parse")
    if not entries:
        print("error: no component-wit-parse: entries in manifest", file=sys.stderr)
        return 1

    wasm_tools = _find_tool("wasm-tools")
    if wasm_tools is None:
        print("error: wasm-tools not in PATH", file=sys.stderr)
        return 1

    compiler = _compiler()
    try:
        compile_env = _selfhost_compile_env(build=True)
    except (FileNotFoundError, RuntimeError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    strict_emit = _uses_s2_selfhost(compile_env)
    failures = 0

    for fixture_rel in entries:
        fixture = REPO_ROOT / "tests" / "fixtures" / fixture_rel
        golden = fixture.with_name(fixture.stem + ".expected.wit")
        if not fixture.is_file():
            print(f"FAIL {fixture_rel}: missing fixture source", file=sys.stderr)
            failures += 1
            continue
        if not golden.is_file():
            print(f"FAIL {fixture_rel}: missing golden {golden.relative_to(REPO_ROOT)}",
                  file=sys.stderr)
            failures += 1
            continue

        golden_text = golden.read_text(encoding="utf-8")
        for marker in REQUIRED_MARKERS:
            if marker not in golden_text:
                print(f"FAIL {fixture_rel}: golden missing marker {marker!r}",
                      file=sys.stderr)
                failures += 1
                break
        else:
            rc, msg = _wasm_tools_parse_wit(wasm_tools, golden)
            if rc != 0:
                print(f"FAIL {fixture_rel}: wasm-tools component wit parse: {msg}",
                      file=sys.stderr)
                failures += 1
                continue
            if "maybe_double" in golden_text or "safe_div" in golden_text:
                print(f"FAIL {fixture_rel}: golden uses snake_case export names",
                      file=sys.stderr)
                failures += 1
                continue
            print(f"pass: {fixture_rel} (wasm-tools parse)")

        if compiler is None:
            continue
        out_rel = f".build/component-wit-parse-{fixture.stem}.wit"
        out_path = REPO_ROOT / out_rel
        out_path.parent.mkdir(parents=True, exist_ok=True)
        rc, msg = _try_emit_wit(
            compiler, str(Path("tests/fixtures") / fixture_rel), out_rel, compile_env,
        )
        try:
            if rc == 2:
                if strict_emit:
                    print(f"FAIL {fixture_rel}: empty WIT emit under s2 selfhost ({msg})",
                          file=sys.stderr)
                    failures += 1
                else:
                    print(f"  note: {fixture_rel} emit skipped ({msg})")
                continue
            if rc != 0:
                print(f"FAIL {fixture_rel}: {msg}", file=sys.stderr)
                failures += 1
                continue
            if _normalize_wit(msg) != _normalize_wit(golden_text):
                print(f"FAIL {fixture_rel}: --emit wit diverges from golden",
                      file=sys.stderr)
                failures += 1
        finally:
            if out_path.is_file():
                out_path.unlink()

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
