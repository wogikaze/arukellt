#!/usr/bin/env python3
"""Close-gate enforcement for audit-reopened false-done issues.

When an issue tracked here lives under ``issues/done/``, this script runs the
acceptance gate defined in ``docs/process/false-done-prevention.md``. While the
issue remains in ``issues/open/``, gates are not enforced (verify stays green).

Issue IDs: 074, 510, 472, 500, 051, 123
"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import inspect
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import fcntl
from collections.abc import Callable
from pathlib import Path
from typing import TypeVar

REPO_ROOT = Path(__file__).resolve().parents[2]
OPEN_DIR = REPO_ROOT / "issues" / "open"
DONE_DIR = REPO_ROOT / "issues" / "done"
MANIFEST = REPO_ROOT / "tests" / "fixtures" / "manifest.txt"
PLAYGROUND = REPO_ROOT / "playground"
_GATE074_LOCK = REPO_ROOT / ".build" / "close-gate-074.lock"
_GATE076_LOCK = REPO_ROOT / ".build" / "close-gate-076.lock"
_PLAYGROUND_LOCK = REPO_ROOT / ".build" / "close-gate-playground.lock"
_CACHE_DIR = REPO_ROOT / ".build" / "close-gate-cache"
_CACHE_VERSION = "close-gate-v1"


def _playground_env() -> dict[str, str]:
    """Prefer a modern Node (nvm) so global/local tsc can run."""
    env = dict(os.environ)
    nvm_root = Path.home() / ".nvm" / "versions" / "node"
    candidates: list[Path] = []
    if nvm_root.is_dir():
        for child in sorted(nvm_root.iterdir(), reverse=True):
            node_bin = child / "bin" / "node"
            if node_bin.is_file():
                candidates.append(child / "bin")
    for bin_dir in candidates:
        env["PATH"] = f"{bin_dir}{os.pathsep}{env.get('PATH', '')}"
        break
    return env

# Issues that invoke pinned-bootstrap selfhost compile or wasmtime.
# Previously serialized via runtime_lock to prevent parallel wasm state stomping,
# but audit (2026-07) confirmed all close-gates are read-only w.r.t. s2/s3 wasm:
# each gate writes only to its own .build/gate-<id>-* temp dir and invokes
# wasmtime as a fresh process. The runtime_lock is only needed for fixpoint
# builds (run_fixpoint in scripts/selfhost/checks.py) which rewrite s2/s3.
# Removing the lock allows #654/#651/#473 (~17s each) to run in parallel,
# cutting close-gate wall time from ~72s to ~17s on cache miss.
_SELFHOST_LOCKED_ISSUES: frozenset[str] = frozenset()

# Playground npm build mutates playground/dist; keep #472/#500 exclusive.
_PLAYGROUND_LOCKED_ISSUES = frozenset({"472", "500"})

T = TypeVar("T")
_FILE_DIGESTS: dict[str, str] | None = None
_FILE_DIGESTS_LOCK = threading.Lock()
_CACHE_ENABLED = True

ISSUE_ID_RE = re.compile(r"^(\d{3})")

# issue_id -> list of human-readable gate names (for error messages)
TRACKED: dict[str, list[str]] = {
    "074": ["P2 component validate + wasmtime run (wasi_p2_native/hello.ark)"],
    "076": ["P2 filesystem fixture validate + wasmtime run + p2_fs_out.txt (wasi_fs_p2.ark)"],
    "510": ["P2 component wasm-tools validate"],
    "472": ["playground typecheck distinguishes parse vs type errors"],
    "500": ["playground wasm typecheck export gate"],
    "051": ["std::time + std::random umbrella (gate-051-std-time-random.py)"],
    "648": ["general canonical ABI umbrella (gate-648-component-export-general-abi.py)"],
    "123": ["Layer C import string syntax component fixture"],
    "641": ["T4 native scaffold compile (t4/native_scaffold.ark)"],
    "639": ["HTTP registry fixtures + gate-639-registry-http.py"],
    "643": ["Grain benchmark hook (compare-benchmarks --compare-lang grain)"],
    "657": ["TCP connect/read/write host-linker smoke (gate-657-sockets-connect-read-write.py)"],
    "658": ["TCP listen/accept host-linker smoke (gate-658-sockets-listen-accept.py)"],
    "139": ["WASI P2 sockets umbrella (gate-139-wasi-p2-sockets-umbrella.py)"],
    "655": ["HTTP outgoing client gate-655-http-outgoing.py"],
    "656": ["HTTP incoming server gate-656-http-incoming.py"],
    "077": ["WASI P2 HTTP umbrella (gate-077-wasi-p2-http-umbrella.py)"],
    "138": ["std::host six-module T1/T3 smoke matrix (gate-138-shared-capabilities-t1-t3.py)"],
    "136": ["ADR-011 std::host rollout consistency (gate-136-std-host-rollout.py)"],
    "652": ["WIT import parser grammar gate-652-wit-import-parser.py"],
    "653": ["WIT import resolver MIR gate-653-wit-import-resolver-mir.py"],
    "654": ["WIT import component emit gate-654-wit-import-component-emit.py"],
    "663": ["ark.toml WIT package resolution gate-663-ark-toml-wit-package.py"],
    "664": ["WIT record/enum import bindings gate-664-wit-import-record-enum-bindings.py"],
    "443": ["Component composition Phase 3 gate-443-component-composition-phase3.py"],
    "665": ["WIT import compose round-trip E2E gate-665-wit-import-compose-roundtrip-e2e.py"],
    "651": ["WIT flags type support gate-651-wit-flags-type-support.py"],
    "473": ["WIT resource handles gate-473-wit-resource-handles.py"],
    "034": ["callable --wit import binding gate-034-wit-cli-integration.py"],
    "679": ["docs-runtime contract audit (gate-679-docs-runtime-contract-audit.py)"],
}


def _issue_location(issue_id: str) -> Path | None:
    for directory in (DONE_DIR, OPEN_DIR):
        for path in directory.glob(f"{issue_id}-*.md"):
            return path
    return None


def _issue_in_done(issue_id: str) -> bool:
    path = _issue_location(issue_id)
    if path is None:
        return False
    return path.parent == DONE_DIR


def _with_file_lock(lock_path: Path, fn: Callable[[], T]) -> T:
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    with lock_path.open("w", encoding="utf-8") as lock_file:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        return fn()


def _with_selfhost_runtime_lock(fn: Callable[[], T]) -> T:
    import importlib.util

    spec = importlib.util.spec_from_file_location(
        "runtime_lock",
        REPO_ROOT / "scripts" / "selfhost" / "runtime_lock.py",
    )
    if spec is None or spec.loader is None:
        raise RuntimeError("missing scripts/selfhost/runtime_lock.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.with_selfhost_runtime_lock(fn)


def _default_jobs() -> int:
    cpu = os.cpu_count() or 4
    return max(1, min(8, cpu))


def _git_files() -> list[str]:
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=str(REPO_ROOT),
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        return []
    files: list[str] = []
    for raw in result.stdout.split(b"\0"):
        if not raw:
            continue
        rel = raw.decode("utf-8", errors="surrogateescape")
        if not rel.startswith(".build/"):
            files.append(rel)
    return sorted(files)


def _file_digest(rel: str) -> str:
    path = REPO_ROOT / rel
    digest = hashlib.sha256()
    try:
        with path.open("rb") as f:
            for chunk in iter(lambda: f.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError:
        digest.update(b"<missing>")
    return digest.hexdigest()


def _prime_file_digests() -> None:
    """Build a rel-path -> content-hash map for tracked files.

    Uses ``git ls-files -s`` to get blob OIDs for tracked files (fast — no
    file I/O).  Untracked files are excluded because their content can change
    between gate runs (e.g. gate output files), making cache keys unstable.
    Tracked files are immutable within a single git state, so the cache key
    is deterministic across processes.

    Thread-safe: uses a lock to prevent concurrent priming from multiple
    ThreadPoolExecutor workers.

    This cuts digest priming from ~13s (reading 7000 files) to ~0.05s
    (single git command, no file I/O).
    """
    global _FILE_DIGESTS

    if _FILE_DIGESTS is not None:
        return
    with _FILE_DIGESTS_LOCK:
        if _FILE_DIGESTS is not None:
            return
        digests: dict[str, str] = {}
        tracked = subprocess.run(
            ["git", "ls-files", "-s", "-z"],
            cwd=str(REPO_ROOT),
            capture_output=True,
            check=False,
        )
        if tracked.returncode != 0:
            _FILE_DIGESTS = digests
            return
        for raw in tracked.stdout.split(b"\0"):
            if not raw:
                continue
            # Format: "<mode> <oid> <stage>\t<path>"
            try:
                meta, rel_bytes = raw.split(b"\t", 1)
            except ValueError:
                continue
            parts = meta.split(b" ")
            if len(parts) < 2:
                continue
            oid = parts[1].decode("ascii", errors="replace")
            rel = rel_bytes.decode("utf-8", errors="surrogateescape")
            if rel.startswith(".build/"):
                continue
            digests[rel] = f"git:{oid}"
        _FILE_DIGESTS = digests


def _matches_prefix(rel: str, prefix: str) -> bool:
    prefix = prefix.rstrip("/")
    return rel == prefix or rel.startswith(prefix + "/")


def _gate_prefixes(issue_id: str) -> tuple[str, ...]:
    """Return the file prefixes whose content changes impact this gate.

    Finer-grained prefixes mean fewer cache invalidations: a change to
    tests/fixtures/component/* invalidates #651/#473 but not #654/#653.
    All gates also depend on the compiler/stdlib (via wasmtime selfhost)
    and their own gate script.
    """
    # Common base: compiler source + stdlib + selfhost runtime + gate script itself.
    # The gate script is auto-included via _gate_cache_key (inspect.getsource).
    base = ("src/compiler", "std", "scripts/selfhost", "scripts/run", "bootstrap")
    script_prefix = f"scripts/check/gate-{issue_id}-"
    if issue_id in {"077", "655", "656", "139", "657", "658"}:
        return base + ("scripts/check", "tools/host-linker", "std/host", "tests/fixtures/host", "tests/fixtures/wasi_http_p2.ark")
    if issue_id == "654":
        return base + (script_prefix, "scripts/check", "tests/fixtures/wit_import", "tests/fixtures/manifest.txt")
    if issue_id == "651":
        return base + (script_prefix, "scripts/check", "tests/fixtures/component", "tests/fixtures/manifest.txt")
    if issue_id == "473":
        return base + (script_prefix, "scripts/check", "tests/fixtures/component", "tests/fixtures/manifest.txt")
    if issue_id == "653":
        return base + (script_prefix, "scripts/check", "tests/fixtures/wit_import", "tests/fixtures/manifest.txt")
    if issue_id == "663":
        return base + (script_prefix, "scripts/check", "tests/fixtures/wit_import/ark_manifest", "tests/fixtures/manifest.txt")
    if issue_id == "664":
        return base + (script_prefix, "scripts/check", "tests/fixtures/wit_import", "tests/fixtures/manifest.txt")
    if issue_id == "665":
        return base + (script_prefix, "scripts/check", "tests/fixtures/wit_import/compose_roundtrip", "tests/fixtures/manifest.txt")
    if issue_id == "443":
        return base + (script_prefix, "scripts/check", "tests/fixtures/wit_import/compose_roundtrip", "src/compiler/main", "tests/fixtures/manifest.txt")
    if issue_id in {"034", "123", "510", "074", "076"}:
        return base + (script_prefix, "scripts/check", "tests/fixtures", "tests/fixtures/manifest.txt")
    if issue_id in {"472", "500"}:
        return (script_prefix, "scripts/check", "playground")
    if issue_id in {"679", "136"}:
        return (script_prefix, "scripts/check", "scripts/gen", "docs", "issues", "README.md", "AGENTS.md")
    if issue_id in {"051", "138", "648", "639", "641", "643"}:
        return base + (script_prefix, "scripts/check", "tests", "docs", "tools")
    return (script_prefix, "scripts/check", "src", "std", "tests", "docs", "tools", "issues")


def _gate_fingerprint(issue_id: str) -> str:
    _prime_file_digests()
    digest = hashlib.sha256()
    digest.update(_CACHE_VERSION.encode("utf-8"))
    digest.update(issue_id.encode("utf-8"))
    prefixes = _gate_prefixes(issue_id)
    for rel, content_hash in (_FILE_DIGESTS or {}).items():
        if any(_matches_prefix(rel, prefix) for prefix in prefixes):
            digest.update(rel.encode("utf-8", errors="surrogateescape"))
            digest.update(b"\0")
            digest.update(content_hash.encode("ascii"))
            digest.update(b"\0")
    return digest.hexdigest()


def _gate_cache_key(issue_id: str) -> str:
    gate_fn = GATES[issue_id]
    digest = hashlib.sha256()
    digest.update(_gate_fingerprint(issue_id).encode("utf-8"))
    digest.update(issue_id.encode("utf-8"))
    digest.update("\n".join(TRACKED[issue_id]).encode("utf-8"))
    try:
        digest.update(inspect.getsource(gate_fn).encode("utf-8"))
    except OSError:
        digest.update(repr(gate_fn).encode("utf-8"))
    return digest.hexdigest()


def _cache_file(issue_id: str) -> Path:
    return _CACHE_DIR / f"{issue_id}.json"


def _cache_lookup(issue_id: str, key: str) -> tuple[int, str] | None:
    if not _CACHE_ENABLED:
        return None
    path = _cache_file(issue_id)
    if not path.is_file():
        return None
    try:
        entry = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if entry.get("key") != key:
        return None
    try:
        return int(entry.get("rc", 1)), str(entry.get("msg", "cached"))
    except (TypeError, ValueError):
        return None


def _cache_store(issue_id: str, key: str, rc: int, msg: str) -> None:
    if not _CACHE_ENABLED:
        return
    try:
        _CACHE_DIR.mkdir(parents=True, exist_ok=True)
        _cache_file(issue_id).write_text(
            json.dumps({"key": key, "rc": rc, "msg": msg}, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    except OSError:
        pass


def _run_gate(issue_id: str) -> tuple[str, list[str], int, str]:
    gate_names = TRACKED[issue_id]
    gate_fn = GATES[issue_id]
    cache_key = _gate_cache_key(issue_id)

    cached = _cache_lookup(issue_id, cache_key)
    if cached is not None:
        rc, msg = cached
        return issue_id, gate_names, rc, f"cached: {msg}" if msg else "cached"

    def invoke() -> tuple[int, str]:
        return gate_fn()

    if issue_id in _PLAYGROUND_LOCKED_ISSUES:
        rc, msg = _with_file_lock(_PLAYGROUND_LOCK, invoke)
    elif issue_id in _SELFHOST_LOCKED_ISSUES:
        rc, msg = _with_selfhost_runtime_lock(invoke)
    else:
        rc, msg = invoke()
    _cache_store(issue_id, cache_key, rc, msg)
    return issue_id, gate_names, rc, msg


def _find_tool(name: str) -> str | None:
    if name == "wasm-tools":
        cargo = Path.home() / ".cargo" / "bin" / "wasm-tools"
        if cargo.is_file():
            return str(cargo)
    return shutil.which(name)


def _compiler() -> Path | None:
    release = REPO_ROOT / "target" / "release" / "arukellt"
    if release.is_file():
        return release
    debug = REPO_ROOT / "target" / "debug" / "arukellt"
    if debug.is_file():
        return debug
    wrapper = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
    if wrapper.is_file():
        return wrapper
    return None


def _selfhost_compile_env() -> dict[str, str]:
    """Prefer s2-runtime selfhost wasm over stale pinned bootstrap."""
    env = dict(os.environ)
    if "ARUKELLT_SELFHOST_WASM" in env:
        return env
    for candidate in (
        REPO_ROOT / ".build" / "selfhost" / "arukellt-s2-runtime.wasm",
        REPO_ROOT / ".build" / "selfhost" / "arukellt-s2.wasm",
        REPO_ROOT / ".build" / "selfhost" / "arukellt-s3.wasm",
        REPO_ROOT / ".build" / "selfhost" / "arukellt-pinned-bootstrap.wasm",
        REPO_ROOT / "bootstrap" / "arukellt-selfhost.wasm",
    ):
        if candidate.is_file():
            env["ARUKELLT_SELFHOST_WASM"] = str(candidate)
            break
    return env


def _manifest_contains(entry: str) -> bool:
    if not MANIFEST.is_file():
        return False
    return entry in MANIFEST.read_text(encoding="utf-8")


def _compile_p2_component(fixture_rel: str, out: Path) -> tuple[int, str]:
    """Core wasm + post-wrap for gate 074 (run export + stdio bridge path)."""
    return _compile_p2_component_wrapped(fixture_rel, out)


def _compile_p2_component_direct(fixture_rel: str, out: Path) -> tuple[int, str]:
    """Pinned bootstrap `--emit component` (validate-only gates)."""
    compiler = _compiler()
    if compiler is None:
        return 2, "arukellt compiler binary not found (build release/debug first)"
    fixture = REPO_ROOT / fixture_rel
    if not fixture.is_file():
        return 1, f"missing fixture {fixture_rel}"
    fixture_arg = str(fixture_rel)
    try:
        out_arg = str(out.relative_to(REPO_ROOT))
    except ValueError:
        out_arg = str(out)
    cmd = [
        str(compiler),
        "compile",
        fixture_arg,
        "--target",
        "wasm32-wasi-p2",
        "--wasi-version",
        "p2",
        "--emit",
        "component",
        "-o",
        out_arg,
    ]
    if compiler.name == "arukellt-selfhost.sh":
        cmd = ["bash", str(compiler), *cmd[1:]]
    result = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
        env=_selfhost_compile_env(),
    )
    if result.returncode != 0:
        tail = (result.stderr or result.stdout)[-800:]
        return 1, f"compile failed: {tail}"
    return 0, ""


def _compile_p2_component_wrapped(fixture_rel: str, out: Path) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "arukellt compiler binary not found (build release/debug first)"
    fixture = REPO_ROOT / fixture_rel
    if not fixture.is_file():
        return 1, f"missing fixture {fixture_rel}"
    fixture_arg = str(fixture_rel)
    out_dir = out.parent
    out_dir.mkdir(parents=True, exist_ok=True)
    core_out = out_dir / f"{out.stem}.core.wasm"
    try:
        core_arg = str(core_out.relative_to(REPO_ROOT))
    except ValueError:
        core_arg = str(core_out)
    try:
        out_arg = str(out.relative_to(REPO_ROOT))
    except ValueError:
        out_arg = str(out)
    cmd = [
        str(compiler),
        "compile",
        fixture_arg,
        "--target",
        "wasm32-wasi-p2",
        "--wasi-version",
        "p2",
        "--emit",
        "core-wasm",
        "-o",
        core_arg,
    ]
    if compiler.name == "arukellt-selfhost.sh":
        cmd = ["bash", str(compiler), *cmd[1:]]
    result = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
        env=_selfhost_compile_env(),
    )
    if result.returncode != 0:
        tail = (result.stderr or result.stdout)[-800:]
        return 1, f"compile failed: {tail}"
    if not core_out.is_file():
        return 1, f"missing core wasm output {core_out}"
    try:
        import importlib.util

        wrap_spec = importlib.util.spec_from_file_location(
            "p2_component_wrap",
            REPO_ROOT / "scripts" / "selfhost" / "p2_component_wrap.py",
        )
        if wrap_spec is None or wrap_spec.loader is None:
            return 1, "missing scripts/selfhost/p2_component_wrap.py"
        wrap_mod = importlib.util.module_from_spec(wrap_spec)
        wrap_spec.loader.exec_module(wrap_mod)
        out.write_bytes(wrap_mod.wrap_p2_command_component(core_out.read_bytes()))
    except Exception as exc:  # noqa: BLE001
        return 1, f"p2_component_wrap failed: {exc}"
    if not out.is_file():
        return 1, f"missing component output {out}"
    return 0, ""


def _wasm_tools_validate(path: Path) -> tuple[int, str]:
    tool = _find_tool("wasm-tools")
    if not tool:
        return 2, "wasm-tools not in PATH"
    result = subprocess.run(
        [tool, "validate", str(path)],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:]
    return 0, ""


def _wasmtime_run(component: Path, expect_stdout: str) -> tuple[int, str]:
    wasmtime = _find_tool("wasmtime")
    if not wasmtime:
        return 2, "wasmtime not in PATH"
    result = subprocess.run(
        [wasmtime, "run", "-W", "gc=y", "-W", "gc-support=y", str(component)],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, f"wasmtime exit {result.returncode}: {(result.stderr or '')[-400:]}"
    if expect_stdout not in result.stdout:
        return 1, f"expected stdout containing {expect_stdout!r}, got {result.stdout!r}"
    return 0, ""



def _wasmtime_run_dir(component: Path, expect_stdout: str) -> tuple[int, str]:
    wasmtime = _find_tool("wasmtime")
    if not wasmtime:
        return 2, "wasmtime not in PATH"
    result = subprocess.run(
        [wasmtime, "run", "-W", "gc=y", "-W", "gc-support=y", "--dir", str(REPO_ROOT), str(component)],
        capture_output=True,
        timeout=60,
        cwd=str(REPO_ROOT),
    )
    if result.returncode != 0:
        stderr = (result.stderr or b"").decode("utf-8", errors="replace")
        return 1, f"wasmtime exit {result.returncode}: {stderr[-400:]}"
    try:
        stdout = result.stdout.decode("utf-8")
    except UnicodeDecodeError:
        return 1, f"stdout is not valid UTF-8: {result.stdout!r}"
    if expect_stdout not in stdout:
        return 1, f"expected stdout containing {expect_stdout!r}, got {stdout!r}"
    return 0, ""


def gate_076() -> tuple[int, str]:
    entry = "component-compile:wasi_fs_p2.ark"
    if not _manifest_contains(entry):
        return 1, f"manifest missing {entry}"
    _GATE076_LOCK.parent.mkdir(parents=True, exist_ok=True)
    with _GATE076_LOCK.open("w", encoding="utf-8") as lock_file:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        return _gate_076_locked()


def _gate_076_locked() -> tuple[int, str]:
    import importlib.util
    spec = importlib.util.spec_from_file_location("runtime_lock", REPO_ROOT / "scripts" / "selfhost" / "runtime_lock.py")
    if spec is None or spec.loader is None:
        return 1, "missing scripts/selfhost/runtime_lock.py"
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.with_selfhost_runtime_lock(_gate_076_body)


def _gate_076_body() -> tuple[int, str]:
    last_rc = 1
    last_msg = ""
    out_file = REPO_ROOT / "p2_fs_out.txt"
    for attempt in range(3):
        out_dir = Path(tempfile.mkdtemp(prefix="close-gate-076-", dir=REPO_ROOT / ".build"))
        try:
            if out_file.is_file():
                out_file.unlink()
            out = out_dir / "wasi_fs_p2.component.wasm"
            last_rc, last_msg = _compile_p2_component("tests/fixtures/wasi_fs_p2.ark", out)
            if last_rc != 0:
                continue
            last_rc, last_msg = _wasm_tools_validate(out)
            if last_rc != 0:
                continue
            last_rc, last_msg = _wasmtime_run_dir(out, "hello p2 fs")
            if last_rc != 0:
                continue
            if not out_file.is_file():
                return 1, "p2_fs_out.txt missing after wasmtime run"
            content = out_file.read_text(encoding="utf-8")
            if content != "hello p2 fs":
                return 1, f"p2_fs_out.txt expected 'hello p2 fs', got {content!r}"
            return 0, ""
        finally:
            shutil.rmtree(out_dir, ignore_errors=True)
        if attempt < 2:
            time.sleep(0.1 * (attempt + 1))
    return last_rc, last_msg



def gate_074() -> tuple[int, str]:
    entry = "component-compile:wasi_p2_native/hello.ark"
    if not _manifest_contains(entry):
        return 1, f"manifest missing {entry}"
    _GATE074_LOCK.parent.mkdir(parents=True, exist_ok=True)
    with _GATE074_LOCK.open("w", encoding="utf-8") as lock_file:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        return _gate_074_locked()


def _gate_074_locked() -> tuple[int, str]:
    import importlib.util

    spec = importlib.util.spec_from_file_location(
        "runtime_lock",
        REPO_ROOT / "scripts" / "selfhost" / "runtime_lock.py",
    )
    if spec is None or spec.loader is None:
        return 1, "missing scripts/selfhost/runtime_lock.py"
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.with_selfhost_runtime_lock(_gate_074_body)


def _gate_074_body() -> tuple[int, str]:
    last_rc = 1
    last_msg = ""
    for attempt in range(3):
        out_dir = Path(
            tempfile.mkdtemp(prefix="close-gate-074-", dir=REPO_ROOT / ".build")
        )
        try:
            out = out_dir / "hello.component.wasm"
            last_rc, last_msg = _compile_p2_component(
                "tests/fixtures/wasi_p2_native/hello.ark", out
            )
            if last_rc != 0:
                continue
            last_rc, last_msg = _wasm_tools_validate(out)
            if last_rc != 0:
                continue
            last_rc, last_msg = _wasmtime_run(out, "hello p2")
            if last_rc == 0:
                return 0, ""
        finally:
            shutil.rmtree(out_dir, ignore_errors=True)
        if attempt < 2:
            time.sleep(0.1 * (attempt + 1))
    return last_rc, last_msg


def gate_510() -> tuple[int, str]:
    out_dir = REPO_ROOT / ".build" / "close-gate-510"
    out_dir.mkdir(parents=True, exist_ok=True)
    out = out_dir / "p2.component.wasm"
    rc, msg = _compile_p2_component_direct("tests/fixtures/wasi_p2_native/hello.ark", out)
    if rc != 0:
        return rc, msg
    return _wasm_tools_validate(out)


def gate_472() -> tuple[int, str]:
    if not (PLAYGROUND / "package.json").is_file():
        return 1, "playground package missing"
    env = _playground_env()
    # Ensure local deps (@types/node) exist for tsc.
    if not (PLAYGROUND / "node_modules").is_dir():
        install = subprocess.run(
            ["npm", "install"],
            cwd=str(PLAYGROUND),
            capture_output=True,
            text=True,
            timeout=180,
            env=env,
        )
        if install.returncode != 0:
            return 1, f"playground npm install failed: {(install.stderr or '')[-400:]}"
    build = subprocess.run(
        ["npm", "run", "build"],
        cwd=str(PLAYGROUND),
        capture_output=True,
        text=True,
        timeout=120,
        env=env,
    )
    if build.returncode != 0:
        return 1, f"playground build failed: {(build.stderr or '')[-400:]}"
    test = subprocess.run(
        ["node", "--test", "dist/tests/typecheck-close-gate.test.js"],
        cwd=str(PLAYGROUND),
        capture_output=True,
        text=True,
        timeout=60,
        env=env,
    )
    if test.returncode != 0:
        return 1, (test.stderr or test.stdout)[-800:]
    return 0, ""


def gate_500() -> tuple[int, str]:
    return gate_472()


def gate_051() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-051-std-time-random.py"
    if not script.is_file():
        return 1, "missing scripts/check/gate-051-std-time-random.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_648() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-648-component-export-general-abi.py"
    if not script.is_file():
        return 1, "missing scripts/check/gate-648-component-export-general-abi.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_139() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-139-wasi-p2-sockets-umbrella.py"
    if not script.is_file():
        return 1, "missing scripts/check/gate-139-wasi-p2-sockets-umbrella.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_077() -> tuple[int, str]:
    if _issue_in_done("655") and _issue_in_done("656"):
        return 0, ""
    script = REPO_ROOT / "scripts" / "check" / "gate-077-wasi-p2-http-umbrella.py"
    if not script.is_file():
        return 1, "missing scripts/check/gate-077-wasi-p2-http-umbrella.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_123() -> tuple[int, str]:
    entry = "component-compile:wit_import/main.ark"
    if not _manifest_contains(entry):
        return 1, f"manifest missing {entry} (Layer C / WIT import close gate)"
    fixture = REPO_ROOT / "tests" / "fixtures" / "wit_import" / "main.ark"
    if not fixture.is_file():
        return 1, "missing tests/fixtures/wit_import/main.ark"
    text = fixture.read_text(encoding="utf-8")
    if 'import "' not in text:
        return 1, "wit_import fixture lacks Layer C string import syntax"
    return 0, ""


def gate_639() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-639-registry-http.py"
    if not script.is_file():
        return 1, "missing scripts/check/gate-639-registry-http.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode == 2:
        return 2, (result.stdout + result.stderr)[-400:]
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_641() -> tuple[int, str]:
    entry = "t4-compile:t4/native_scaffold.ark"
    if not _manifest_contains(entry):
        return 1, f"manifest missing {entry}"
    fixture = REPO_ROOT / "tests" / "fixtures" / "t4" / "native_scaffold.ark"
    if not fixture.is_file():
        return 1, "missing tests/fixtures/t4/native_scaffold.ark"
    target_ark = REPO_ROOT / "src" / "compiler" / "driver" / "target.ark"
    native_ark = REPO_ROOT / "src" / "compiler" / "driver" / "native.ark"
    if not target_ark.is_file() or not native_ark.is_file():
        return 1, "missing src/compiler/driver/target.ark or native.ark"
    target_text = target_ark.read_text(encoding="utf-8")
    native_text = native_ark.read_text(encoding="utf-8")
    if "is_native_target" not in target_text:
        return 1, "target.ark lacks native registration"
    if "emit_native_scaffold" not in native_text or "T4 native scaffold" not in native_text:
        return 1, "native.ark lacks scaffold emitter"
    contract = (REPO_ROOT / "docs" / "adr" / "ADR-007-targets.md").read_text(encoding="utf-8")
    if "native_scaffold" not in contract or "scaffold" not in contract:
        return 1, "ADR-007-targets.md T4 native section not scaffold"
    return 0, ""


def gate_643() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-643-grain-benchmark.py"
    if not script.is_file():
        return 1, "missing scripts/check/gate-643-grain-benchmark.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_652() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-652-wit-import-parser.py"
    if not script.is_file():
        return 1, "missing gate-652-wit-import-parser.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_653() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-653-wit-import-resolver-mir.py"
    if not script.is_file():
        return 1, "missing gate-653-wit-import-resolver-mir.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_663() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-663-ark-toml-wit-package.py"
    if not script.is_file():
        return 1, "missing gate-663-ark-toml-wit-package.py"
    result = subprocess.run([sys.executable, str(script)], cwd=str(REPO_ROOT), capture_output=True, text=True)
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-400:]
    return 0, ""

def gate_664() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-664-wit-import-record-enum-bindings.py"
    if not script.is_file():
        return 1, "missing gate-664-wit-import-record-enum-bindings.py"
    result = subprocess.run([sys.executable, str(script)], cwd=str(REPO_ROOT), capture_output=True, text=True)
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-400:]
    return 0, ""


def gate_443() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-443-component-composition-phase3.py"
    if not script.is_file():
        return 1, "missing gate-443-component-composition-phase3.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=300,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_665() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-665-wit-import-compose-roundtrip-e2e.py"
    if not script.is_file():
        return 1, "missing gate-665-wit-import-compose-roundtrip-e2e.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=300,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_654() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-654-wit-import-component-emit.py"
    if not script.is_file():
        return 1, "missing gate-654-wit-import-component-emit.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_651() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-651-wit-flags-type-support.py"
    if not script.is_file():
        return 1, "missing gate-651-wit-flags-type-support.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_473() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-473-wit-resource-handles.py"
    if not script.is_file():
        return 1, "missing gate-473-wit-resource-handles.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_034() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-034-wit-cli-integration.py"
    if not script.is_file():
        return 1, "missing gate-034-wit-cli-integration.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_655() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-655-http-outgoing.py"
    if not script.is_file():
        return 1, "missing gate-655-http-outgoing.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_656() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-656-http-incoming.py"
    if not script.is_file():
        return 1, "missing gate-656-http-incoming.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_657() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-657-sockets-connect-read-write.py"
    if not script.is_file():
        return 1, "missing scripts/check/gate-657-sockets-connect-read-write.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_658() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-658-sockets-listen-accept.py"
    if not script.is_file():
        return 1, "missing scripts/check/gate-658-sockets-listen-accept.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_138() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-138-shared-capabilities-t1-t3.py"
    if not script.is_file():
        return 1, "missing scripts/check/gate-138-shared-capabilities-t1-t3.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_136() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-136-std-host-rollout.py"
    if not script.is_file():
        return 1, "missing scripts/check/gate-136-std-host-rollout.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_679() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-679-docs-runtime-contract-audit.py"
    if not script.is_file():
        return 1, "missing scripts/check/gate-679-docs-runtime-contract-audit.py"
    env = dict(os.environ)
    env["ARUKELLT_GATE_679_SKIP_DOCS_CONSISTENCY"] = "1"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
        env=env,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


GATES: dict[str, callable[[], tuple[int, str]]] = {
    "074": gate_074,
    "076": gate_076,
    "510": gate_510,
    "472": gate_472,
    "500": gate_500,
    "051": gate_051,
    "648": gate_648,
    "123": gate_123,
    "639": gate_639,
    "641": gate_641,
    "643": gate_643,
    "657": gate_657,
    "658": gate_658,
    "139": gate_139,
    "655": gate_655,
    "656": gate_656,
    "077": gate_077,
    "138": gate_138,
    "136": gate_136,
    "652": gate_652,
    "653": gate_653,
    "654": gate_654,
    "663": gate_663,
    "664": gate_664,
    "443": gate_443,
    "665": gate_665,
    "651": gate_651,
    "473": gate_473,
    "034": gate_034,
    "679": gate_679,
}


def main() -> int:
    global _CACHE_ENABLED

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "-j",
        "--jobs",
        type=int,
        default=_default_jobs(),
        help="parallel close-gate workers (default: min(8, cpu_count))",
    )
    parser.add_argument(
        "--no-cache",
        action="store_true",
        help="disable persistent close-gate pass cache",
    )
    args = parser.parse_args()
    jobs = max(1, args.jobs)
    _CACHE_ENABLED = not args.no_cache and os.environ.get("ARUKELLT_CLOSE_GATE_CACHE") != "0"

    failures: list[str] = []
    skipped = 0
    enforced_ids = sorted(
        issue_id for issue_id in TRACKED if _issue_in_done(issue_id)
    )
    enforced = len(enforced_ids)

    if enforced == 0:
        print("false-done-close-gates: PASS (no tracked issues in issues/done/)")
        return 0

    workers = min(jobs, enforced)
    results: list[tuple[str, list[str], int, str]] = []

    def record(result: tuple[str, list[str], int, str]) -> None:
        issue_id, gate_names, rc, msg = result
        results.append(result)
        if rc == 0:
            suffix = " (cached)" if msg == "cached" else ""
            print(f"  pass close-gate #{issue_id}{suffix}", flush=True)

    if workers == 1:
        for issue_id in enforced_ids:
            record(_run_gate(issue_id))
    else:
        with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as executor:
            futures = [
                executor.submit(_run_gate, issue_id) for issue_id in enforced_ids
            ]
            for future in concurrent.futures.as_completed(futures):
                record(future.result())

    for issue_id, gate_names, rc, msg in sorted(results, key=lambda item: item[0]):
        if rc == 2:
            skipped += 1
            failures.append(
                f"#{issue_id} ({', '.join(gate_names)}): SKIP prereq — {msg}"
            )
            continue
        if rc != 0:
            failures.append(f"#{issue_id} ({', '.join(gate_names)}): {msg}")
            continue

    if failures:
        print("false-done-close-gates: FAIL", file=sys.stderr)
        for err in failures:
            print(f"  {err}", file=sys.stderr)
        return 1

    print(f"false-done-close-gates: PASS ({enforced} enforced, jobs={workers})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
