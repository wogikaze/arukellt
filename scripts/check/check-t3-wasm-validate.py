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

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

# ── Constants ────────────────────────────────────────────────────────────────

REPO_ROOT = Path(__file__).resolve().parents[2]
MANIFEST = REPO_ROOT / "tests" / "fixtures" / "manifest.txt"
T3_TARGET = "wasm32-wasi-p2"
VALIDATE_FEATURES = "gc"
COMPILE_TIMEOUT = 60  # seconds per fixture
VALIDATE_TIMEOUT = 30  # seconds per fixture

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
    wt = shutil.which("wasm-tools")
    if wt:
        return wt
    cargo = Path.home() / ".cargo" / "bin" / "wasm-tools"
    if cargo.is_file():
        return str(cargo)
    return None


def compile_fixture(
    wasmtime: str,
    compiler_wasm: Path,
    src: str,
    out: Path,
    root: Path,
) -> tuple[bool, str]:
    """Compile a fixture with the selfhost compiler. Returns (ok, stderr).

    The selfhost runs under wasmtime with ``--dir=<root>`` so the output
    path must be relative to the repo root.  We compile into a temp dir
    *inside* the repo to satisfy that constraint.
    """
    # Use a temp directory inside the repo so wasmtime --dir can write to it.
    tmp_rel = ".ark-t3-validate-tmp"
    tmp_inside = root / tmp_rel
    tmp_inside.mkdir(parents=True, exist_ok=True)
    guest_out = f"{tmp_rel}/{out.name}"
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


# ── Main ─────────────────────────────────────────────────────────────────────

def main() -> int:
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

    # Deduplicate (t3-compile and t3-run may list the same fixture)
    seen: set[str] = set()
    unique: list[str] = []
    for f in fixtures:
        if f not in seen:
            seen.add(f)
            unique.append(f)
    fixtures = unique

    tmpdir = Path(tempfile.mkdtemp(prefix="ark-t3-validate-"))
    try:
        # Clean up the repo-internal temp dir from any prior run
        repo_tmp = REPO_ROOT / ".ark-t3-validate-tmp"
        if repo_tmp.exists():
            shutil.rmtree(str(repo_tmp), ignore_errors=True)
        pass_count = 0
        fail_validate = 0
        fail_compile = 0
        skip_count = 0
        fail_details: list[str] = []

        for fixture in fixtures:
            if fixture in T3_COMPILE_SKIP:
                skip_count += 1
                continue

            src_abs = REPO_ROOT / "tests" / "fixtures" / fixture
            if not src_abs.is_file():
                skip_count += 1
                continue

            # selfhost expects a path relative to the repo root
            src = str(Path("tests") / "fixtures" / fixture)
            name = fixture.replace("/", "_").replace(".ark", "")
            out = tmpdir / f"{name}.wasm"

            ok, stderr = compile_fixture(wasmtime, compiler_wasm, src, out, REPO_ROOT)
            if not ok:
                fail_compile += 1
                fail_details.append(f"  COMPILE FAIL: {fixture}")
                continue

            ok, err = validate_wasm(wasm_tools, out)
            if ok:
                pass_count += 1
            else:
                fail_validate += 1
                # Extract the key error line
                err_line = err.split("\n")[0] if err else "unknown"
                fail_details.append(f"  VALIDATE FAIL: {fixture} — {err_line}")

        total = pass_count + fail_validate + fail_compile + skip_count
        print(f"T3 WASM validation: {pass_count} pass, {fail_validate} validate-fail, "
              f"{fail_compile} compile-fail, {skip_count} skip (total {total})")

        if fail_details:
            print()
            for line in fail_details[:50]:
                print(line)
            if len(fail_details) > 50:
                print(f"  ... and {len(fail_details) - 50} more")

        if fail_validate > 0 or fail_compile > 0:
            return 1
        return 0

    finally:
        shutil.rmtree(str(tmpdir), ignore_errors=True)
        repo_tmp = REPO_ROOT / ".ark-t3-validate-tmp"
        if repo_tmp.exists():
            shutil.rmtree(str(repo_tmp), ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
