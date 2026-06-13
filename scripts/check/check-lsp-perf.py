#!/usr/bin/env python3
"""LSP performance smoke tests (issue #463).

Measures hover / definition / completion / open+diagnose / incremental-change
turnaround on large fixtures. Records timings to target/lsp-perf-results.json
and warns when any case exceeds WARN_MULTIPLIER × baseline.

Not a hard CI gate unless PERF_GATE=strict is set.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path


WARN_MULTIPLIER = 5.0
BASELINE_HOVER_MS = 50.0
BASELINE_DEFINITION_MS = 30.0
BASELINE_COMPLETION_MS = 100.0
BASELINE_OPEN_LARGE_MS = 200.0
BASELINE_INCREMENTAL_MS = 150.0


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _find_wasmtime() -> str | None:
    return shutil.which("wasmtime")


def _resolve_compiler(root: Path) -> Path | None:
    sys.path.insert(0, str(root))
    from scripts.selfhost.checks import resolve_ide_gate_compiler_wasm

    compiler = resolve_ide_gate_compiler_wasm(root)
    if compiler is not None:
        return compiler
    pinned = root / "bootstrap" / "arukellt-selfhost.wasm"
    return pinned if pinned.is_file() else None


def _baseline(name: str, default: float) -> float:
    env_key = f"LSP_PERF_BASELINE_{name.upper()}_MS"
    raw = os.environ.get(env_key)
    if raw is None:
        return default
    try:
        return float(raw)
    except ValueError:
        return default


def _record_perf(name: str, elapsed_ms: float, results_path: Path) -> None:
    entry = {
        "test": name,
        "elapsed_ms": elapsed_ms,
        "timestamp": int(time.time()),
    }
    results_path.parent.mkdir(parents=True, exist_ok=True)
    with results_path.open("a", encoding="utf-8") as f:
        f.write(json.dumps(entry) + "\n")


def _warn(name: str, elapsed_ms: float, baseline_ms: float) -> None:
    ratio = elapsed_ms / baseline_ms if baseline_ms > 0 else 0.0
    if ratio > WARN_MULTIPLIER:
        print(
            f"PERF WARNING: {name} took {elapsed_ms:.1f}ms "
            f"({ratio:.1f}x baseline of {baseline_ms:.1f}ms)",
            file=sys.stderr,
        )
        if os.environ.get("PERF_GATE") == "strict":
            raise SystemExit(1)


def _frame(msg: str) -> bytes:
    body = msg.encode("utf-8")
    return f"Content-Length: {len(body)}\r\n\r\n".encode("ascii") + body


def _rpc(id_: int, method: str, params: dict) -> bytes:
    payload = json.dumps({"jsonrpc": "2.0", "id": id_, "method": method, "params": params})
    return _frame(payload)


def _notify(method: str, params: dict) -> bytes:
    payload = json.dumps({"jsonrpc": "2.0", "method": method, "params": params})
    return _frame(payload)


def _lifecycle_prefix(uri: str, text: str) -> bytes:
    return (
        _rpc(1, "initialize", {"processId": None, "rootUri": None, "capabilities": {}})
        + _notify("initialized", {})
        + _notify(
            "textDocument/didOpen",
            {
                "textDocument": {
                    "uri": uri,
                    "languageId": "arukellt",
                    "version": 1,
                    "text": text,
                }
            },
        )
    )


def _timed_request(script: bytes, wasmtime: str, compiler: Path, root: Path) -> float:
    start = time.perf_counter()
    rc = subprocess.run(
        [wasmtime, "run", "--dir", str(root), str(compiler), "--", "lsp"],
        cwd=str(root),
        input=script,
        capture_output=True,
        timeout=120,
    ).returncode
    elapsed_ms = (time.perf_counter() - start) * 1000.0
    if rc != 0:
        raise RuntimeError(f"lsp exited {rc}")
    return elapsed_ms


def main() -> int:
    root = _repo_root()
    fixture = root / "tests" / "fixtures" / "lsp_perf" / "large_module.ark"
    if not fixture.is_file():
        print(f"error: missing fixture {fixture}", file=sys.stderr)
        return 1

    wasmtime = _find_wasmtime()
    if not wasmtime:
        print("error: wasmtime not found", file=sys.stderr)
        return 1

    compiler = _resolve_compiler(root)
    if compiler is None:
        print("error: no selfhost compiler wasm", file=sys.stderr)
        return 1

    source = fixture.read_text(encoding="utf-8")
    uri = "file:///large_module.ark"
    results_path = Path(
        os.environ.get("LSP_PERF_OUTPUT", str(root / "target" / "lsp-perf-results.json"))
    )
    if results_path.exists():
        results_path.unlink()

    prefix = _lifecycle_prefix(uri, source)
    cases: list[tuple[str, bytes, float]] = [
        (
            "hover_large_file",
            prefix
            + _rpc(2, "textDocument/hover", {"textDocument": {"uri": uri}, "position": {"line": 250, "character": 10}}),
            _baseline("HOVER", BASELINE_HOVER_MS),
        ),
        (
            "definition_large_file",
            prefix
            + _rpc(
                3,
                "textDocument/definition",
                {"textDocument": {"uri": uri}, "position": {"line": 250, "character": 10}},
            ),
            _baseline("DEFINITION", BASELINE_DEFINITION_MS),
        ),
        (
            "completion_large_file",
            prefix
            + _rpc(
                4,
                "textDocument/completion",
                {"textDocument": {"uri": uri}, "position": {"line": 250, "character": 10}},
            ),
            _baseline("COMPLETION", BASELINE_COMPLETION_MS),
        ),
        (
            "open_and_diagnose_large",
            _lifecycle_prefix(uri, source)
            + _rpc(5, "textDocument/hover", {"textDocument": {"uri": uri}, "position": {"line": 1, "character": 0}}),
            _baseline("OPEN_LARGE", BASELINE_OPEN_LARGE_MS),
        ),
    ]

    lines = source.splitlines()
    if len(lines) > 100:
        lines[100] = "    let modified = helper_001(99)"
        changed = "\n".join(lines) + "\n"
        cases.append(
            (
                "incremental_change_diagnose",
                _lifecycle_prefix(uri, source)
                + _notify(
                    "textDocument/didChange",
                    {
                        "textDocument": {"uri": uri, "version": 2},
                        "contentChanges": [{"text": changed}],
                    },
                )
                + _rpc(6, "textDocument/hover", {"textDocument": {"uri": uri}, "position": {"line": 1, "character": 0}}),
                _baseline("INCREMENTAL", BASELINE_INCREMENTAL_MS),
            )
        )

    failures = 0
    for name, script, baseline in cases:
        try:
            elapsed_ms = _timed_request(script, wasmtime, compiler, root)
        except Exception as exc:  # noqa: BLE001
            print(f"FAIL {name}: {exc}", file=sys.stderr)
            failures += 1
            continue
        _record_perf(name, elapsed_ms, results_path)
        _warn(name, elapsed_ms, baseline)
        print(f"  pass: {name} ({elapsed_ms:.1f}ms)")

    if failures:
        print(f"lsp-perf: {failures} failure(s)", file=sys.stderr)
        return 1

    print(f"lsp-perf: {len(cases)} case(s) recorded → {results_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
