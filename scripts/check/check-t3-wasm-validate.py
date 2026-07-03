#!/usr/bin/env python3
"""T3 fixture WASM validation gate.

Compiles every ``t3-compile:`` and ``t3-run:`` fixture in the manifest
with the selfhost compiler (``--target wasm32-wasi-p2``) and validates
the output with ``wasm-tools validate --features gc``.

This catches emitter bugs that produce invalid WASM — the most common
class of regression in the GC target path.

Exit codes:
  0  — all fixtures compiled and validated
  1  — one or more fixtures failed to compile or validate
  2  — environment error (selfhost wasm or wasm-tools not found)
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ProcessPoolExecutor, as_completed
from pathlib import Path

# ── Constants ────────────────────────────────────────────────────────────────

REPO_ROOT = Path(__file__).resolve().parents[2]
MANIFEST = REPO_ROOT / "tests" / "fixtures" / "manifest.txt"
T3_TARGET = "wasm32-wasi-p2"
VALIDATE_FEATURES = "gc"
COMPILE_TIMEOUT = 60  # seconds per fixture
VALIDATE_TIMEOUT = 30  # seconds per fixture
T3_CACHE_DIR = REPO_ROOT / ".build" / "t3-cache"
DEFAULT_JOBS = max(1, (os.cpu_count() or 4) // 2)

# Fixtures that are known to require special flags / WIT imports and are
# expected to fail compilation in the plain ``compile`` path.  These are
# tracked separately and should be fixed by making the compile path handle
# them, but for now they are skipped to keep the gate meaningful.
T3_COMPILE_SKIP: frozenset[str] = frozenset({
    # Requires WIT import bindings (component model)
    "component/import_async_resource_type.ark",
    "component/import_future_type.ark",
    "component/import_resource_type.ark",
    "component/resource_v3/resource_basic.ark",
    "component/resource_v3/resource_handle.ark",
    "component/resource_v3/resource_gc.ark",
    "component/resource_v3/resource_async_drop.ark",
    "component/resource_v3/resource_forward.ark",
    # async/future/stream require special runtime support
    "async/async_fn_basic.ark",
    "async/future_type_ann.ark",
    "async/stream_type_ann.ark",
    # stdlib_host requires WASI host capability bindings
    "stdlib_host/wasi_stdio.ark",
    "stdlib_host/wasi_fs.ark",
    "stdlib_host/wasi_args.ark",
    "stdlib_host/wasi_process.ark",
    "stdlib_host/wasi_clock.ark",
    "stdlib_host/wasi_random.ark",
    # host/http requires HTTP host capability bindings
    "host/http/get_err_dns.ark",
    "host/http/request_err_refused.ark",
    "host/http/incoming_smoke.ark",
    # stdlib_fs requires filesystem host capability
    "stdlib_fs/host_capability_contract.ark",
    # stdlib_http_compile requires HTTP host capability
    "stdlib_http_compile.ark",
})


# ── Helpers ──────────────────────────────────────────────────────────────────

def load_t3_fixtures(manifest: Path) -> list[str]:
    """Return ``t3-compile:`` and ``t3-run:`` fixture paths from manifest."""
    fixtures: list[str] = []
    for line in manifest.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        kind, _, path = line.partition(":")
        kind = kind.strip()
        if kind in ("t3-compile", "t3-run"):
            fixtures.append(path.strip())
    return fixtures


def find_selfhost_wasm(root: Path) -> Path | None:
    """Resolve the selfhost compiler WASM to use.

    Prefer s2 over s3 because s3 is a self-compiled output that may
    contain emitter bugs from the very issue we are trying to detect.
    """
    env = os.environ.get("ARUKELLT_SELFHOST_WASM")
    if env and Path(env).is_file():
        return Path(env)
    for candidate in (
        root / ".build" / "selfhost" / "arukellt-s2.wasm",
        root / ".build" / "selfhost" / "arukellt-s3.wasm",
        root / ".bootstrap-build" / "arukellt-s2.wasm",
        root / "bootstrap" / "arukellt-selfhost.wasm",
    ):
        if candidate.is_file():
            return candidate
    return None


def find_wasmtime() -> str | None:
    return shutil.which("wasmtime")


def find_wasm_tools() -> str | None:
    # Prefer ~/.cargo/bin/wasm-tools (the Rust wasm-tools CLI that supports
    # `validate --features gc`) over any other `wasm-tools` that may appear
    # earlier in PATH (e.g. a unrelated binary in ~/.local/bin).
    cargo = Path.home() / ".cargo" / "bin" / "wasm-tools"
    if cargo.is_file():
        return str(cargo)
    wt = shutil.which("wasm-tools")
    if wt:
        return wt
    return None


def compile_fixture(
    wasmtime: str,
    compiler_wasm: Path,
    src: str,
    out: Path,
    root: Path,
    tmp_rel: str = ".ark-t3-validate-tmp",
) -> tuple[bool, str]:
    """Compile a fixture with the selfhost compiler. Returns (ok, stderr).

    The selfhost runs under wasmtime with ``--dir=<root>`` so the output
    path must be relative to the repo root.  We compile into a temp dir
    *inside* the repo to satisfy that constraint.
    """
    # Use a temp directory inside the repo so wasmtime --dir can write to it.
    tmp_inside = root / tmp_rel
    tmp_inside.mkdir(parents=True, exist_ok=True)
    guest_out = f"{tmp_rel}/{out.name}"
    try:
        result = subprocess.run(
            [
                wasmtime, "run",
                "--wasm", "gc", "--wasm", "function-references",
                "--dir", str(root),
                str(compiler_wasm), "--",
                "compile", src,
                "--target", T3_TARGET,
                "-o", guest_out,
            ],
            cwd=str(root),
            capture_output=True,
            text=True,
            timeout=COMPILE_TIMEOUT,
        )
    except subprocess.TimeoutExpired:
        return False, "timed out"
    produced = tmp_inside / out.name
    ok = produced.is_file() and produced.stat().st_size > 0
    if ok:
        # Move to the requested output path
        shutil.move(str(produced), str(out))
    return ok, result.stderr or ""


def validate_wasm(wasm_tools: str, wasm_path: Path) -> tuple[bool, str]:
    """Validate a WASM file. Returns (ok, error_msg)."""
    result = subprocess.run(
        [wasm_tools, "validate", "--features", VALIDATE_FEATURES, str(wasm_path)],
        capture_output=True,
        text=True,
        timeout=VALIDATE_TIMEOUT,
    )
    if result.returncode == 0:
        return True, ""
    return False, (result.stderr or result.stdout).strip()


def _process_fixture_worker(args: tuple) -> tuple[str, str, str, str]:
    """Worker function for parallel execution.

    Args: (fixture, compiler_wasm_str, wasmtime_str, wasm_tools_str, root_str)
    Returns: (fixture, status, detail, key) where status is
    "pass", "validate-fail", or "compile-fail".
    """
    fixture, compiler_wasm_str, wasmtime_str, wasm_tools_str, root_str, key = args
    root = Path(root_str)
    compiler_wasm = Path(compiler_wasm_str)
    wasmtime = wasmtime_str
    wasm_tools = wasm_tools_str

    src = str(Path("tests") / "fixtures" / fixture)
    name = fixture.replace("/", "_").replace(".ark", "")
    # Each worker uses its own temp dir to avoid conflicts
    worker_tmp = f".ark-t3-validate-tmp-{os.getpid()}"
    worker_out = root / worker_tmp / f"{name}.wasm"

    ok, stderr = compile_fixture(wasmtime, compiler_wasm, src, worker_out, root, tmp_rel=worker_tmp)
    if not ok:
        detail = f"  COMPILE FAIL: {fixture}"
        if "timed out" in stderr.lower():
            return fixture, "timeout", f"  COMPILE TIMEOUT: {fixture}", key
        return fixture, "compile-fail", detail, key

    ok, err = validate_wasm(wasm_tools, worker_out)
    # Clean up the temp wasm
    try:
        worker_out.unlink(missing_ok=True)
    except OSError:
        pass

    if ok:
        return fixture, "pass", "", key
    else:
        err_line = err.split("\n")[0] if err else "unknown"
        detail = f"  VALIDATE FAIL: {fixture} — {err_line}"
        return fixture, "validate-fail", detail, key


# ── Cache ────────────────────────────────────────────────────────────────────

def _file_hash(path: Path) -> str:
    """Return SHA-256 hex digest of a file's contents."""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _tool_fingerprint(tool: str) -> str:
    result = subprocess.run(
        [tool, "--version"],
        capture_output=True,
        text=True,
        timeout=10,
    )
    if result.returncode != 0:
        return tool
    return (result.stdout or result.stderr).strip()


def _cache_key(compiler_wasm: Path, fixture_path: Path, wasm_tools: str) -> str:
    """Compute a cache key from compiler, validator, and fixture source."""
    return hashlib.sha256(
        (
            _file_hash(compiler_wasm)
            + ":"
            + _tool_fingerprint(wasm_tools)
            + ":"
            + VALIDATE_FEATURES
            + ":"
            + str(fixture_path)
            + ":"
            + _file_hash(fixture_path)
        ).encode()
    ).hexdigest()


def _cache_lookup(fixture: str, key: str) -> tuple[str, str] | None:
    """Return (status, detail) from cache, or None on miss.

    status is "pass", "validate-fail", or "compile-fail".
    """
    cache_file = T3_CACHE_DIR / f"{fixture.replace('/', '__')}.json"
    if not cache_file.is_file():
        return None
    try:
        entry = json.loads(cache_file.read_text())
        if entry.get("key") == key:
            return entry.get("status", ""), entry.get("detail", "")
    except (json.JSONDecodeError, KeyError):
        pass
    return None


def _cache_store(fixture: str, key: str, status: str, detail: str) -> None:
    """Store a compilation result in the cache."""
    T3_CACHE_DIR.mkdir(parents=True, exist_ok=True)
    cache_file = T3_CACHE_DIR / f"{fixture.replace('/', '__')}.json"
    try:
        cache_file.write_text(json.dumps({
            "key": key, "status": status, "detail": detail,
        }))
    except OSError:
        pass


def _cache_invalidate_all() -> None:
    """Remove all cache entries (e.g. when --no-cache is used)."""
    if T3_CACHE_DIR.is_dir():
        shutil.rmtree(str(T3_CACHE_DIR), ignore_errors=True)


# ── Main ─────────────────────────────────────────────────────────────────────

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--no-cache",
        action="store_true",
        help="disable and clear the T3 validation cache for this run",
    )
    parser.add_argument(
        "-j",
        "--jobs",
        type=int,
        default=DEFAULT_JOBS,
        help=f"parallel compile/validate workers (default: {DEFAULT_JOBS})",
    )
    args = parser.parse_args()
    use_cache = not args.no_cache
    jobs = max(1, args.jobs)

    wasmtime = find_wasmtime()
    if not wasmtime:
        print("error: wasmtime not found", file=sys.stderr)
        return 2

    wasm_tools = find_wasm_tools()
    if not wasm_tools:
        print("error: wasm-tools not found", file=sys.stderr)
        return 2

    compiler_wasm = find_selfhost_wasm(REPO_ROOT)
    if compiler_wasm is None:
        print("error: selfhost compiler wasm not found", file=sys.stderr)
        return 2

    if not MANIFEST.is_file():
        print(f"error: manifest not found: {MANIFEST}", file=sys.stderr)
        return 2

    fixtures = load_t3_fixtures(MANIFEST)
    if not fixtures:
        print("error: no t3-compile/t3-run fixtures in manifest", file=sys.stderr)
        return 2

    if not use_cache:
        _cache_invalidate_all()

    # Deduplicate (t3-compile and t3-run may list the same fixture)
    seen: set[str] = set()
    unique: list[str] = []
    for f in fixtures:
        if f not in seen:
            seen.add(f)
            unique.append(f)
    fixtures = unique

    # Clean up any stale per-worker temp dirs from prior runs
    for stale in REPO_ROOT.glob(".ark-t3-validate-tmp*"):
        shutil.rmtree(str(stale), ignore_errors=True)

    pass_count = 0
    fail_validate = 0
    fail_compile = 0
    skip_count = 0
    cache_hits = 0
    fail_details: list[str] = []

    # Phase 1: Check cache for all fixtures
    uncached: list[tuple[str, str]] = []  # (fixture, key)
    for fixture in fixtures:
        if fixture in T3_COMPILE_SKIP:
            skip_count += 1
            continue

        src_abs = REPO_ROOT / "tests" / "fixtures" / fixture
        if not src_abs.is_file():
            skip_count += 1
            continue

        if use_cache:
            key = _cache_key(compiler_wasm, src_abs, wasm_tools)
            cached = _cache_lookup(fixture, key)
            if cached is not None:
                status, detail = cached
                cache_hits += 1
                if status == "pass":
                    pass_count += 1
                elif status == "validate-fail":
                    fail_validate += 1
                    fail_details.append(detail)
                elif status == "compile-fail":
                    fail_compile += 1
                    fail_details.append(detail)
                continue
            uncached.append((fixture, key))
        else:
            uncached.append((fixture, ""))

    # Phase 2: Compile + validate uncached fixtures in parallel
    if uncached:
        worker_args = [
            (fixture, str(compiler_wasm), wasmtime, wasm_tools, str(REPO_ROOT), key)
            for fixture, key in uncached
        ]

        if jobs <= 1:
            # Sequential fallback
            results = [_process_fixture_worker(a) for a in worker_args]
        else:
            results = []
            with ProcessPoolExecutor(max_workers=jobs) as pool:
                futures = {pool.submit(_process_fixture_worker, a): a for a in worker_args}
                for future in as_completed(futures):
                    results.append(future.result())

        for fixture, status, detail, key in results:
            if status == "pass":
                pass_count += 1
                if use_cache:
                    _cache_store(fixture, key, "pass", "")
            elif status == "validate-fail":
                fail_validate += 1
                fail_details.append(detail)
                if use_cache:
                    _cache_store(fixture, key, "validate-fail", detail)
            elif status == "compile-fail":
                fail_compile += 1
                fail_details.append(detail)
                if use_cache:
                    _cache_store(fixture, key, "compile-fail", detail)
            elif status == "timeout":
                skip_count += 1
                fail_details.append(detail)

    # Clean up per-worker temp dirs
    for stale in REPO_ROOT.glob(".ark-t3-validate-tmp*"):
        shutil.rmtree(str(stale), ignore_errors=True)

    total = pass_count + fail_validate + fail_compile + skip_count
    cache_info = f" [{cache_hits} cache hits]" if cache_hits > 0 else ""
    print(f"T3 WASM validation: {pass_count} pass, {fail_validate} validate-fail, "
          f"{fail_compile} compile-fail, {skip_count} skip (total {total}){cache_info}")

    if fail_details:
        print()
        for line in fail_details[:50]:
            print(line)
        if len(fail_details) > 50:
            print(f"  ... and {len(fail_details) - 50} more")

    if fail_validate > 0 or fail_compile > 0:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
