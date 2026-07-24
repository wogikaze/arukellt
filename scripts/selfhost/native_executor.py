"""Native C99 selfhost executor pipeline (ADR-049, RFC-008)."""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
import time
from pathlib import Path

from selfhost.checks import (
    _postprocess_selfhost_compiler_wasm,
    _prepare_bootstrap_workspace,
    _reject_invalid_compiler_wasm,
    _selfhost_source_fingerprint,
)

RUNTIME_ABI_VERSION = 1
BACKEND_SCHEMA_VERSION = 1
CAPABILITY_TABLE_VERSION = 1
MINIMUM_CLANG_VERSION = 14


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _toolchain() -> tuple[str | None, str]:
    override = os.environ.get("ARUKELLT_CC", "").strip()
    candidates = [override] if override else [
        "clang",
        "clang-18",
        "clang-17",
        "clang-16",
        "clang-15",
        "clang-14",
    ]
    path = next((shutil.which(candidate) for candidate in candidates if shutil.which(candidate)), None)
    if path is None:
        requested = override or "clang 14+"
        return None, f"toolchain diagnostic: C compiler `{requested}` was not found"
    result = subprocess.run(
        [path, "--version"], capture_output=True, text=True, check=False
    )
    version = (result.stdout or result.stderr).splitlines()[0] if result.returncode == 0 else ""
    match = re.search(r"clang version (\d+)", version)
    if match is None or int(match.group(1)) < MINIMUM_CLANG_VERSION:
        return None, (
            f"toolchain diagnostic: clang {MINIMUM_CLANG_VERSION}+ is required; "
            f"detected `{version or path}`"
        )
    return str(Path(path).resolve()), version


def _timed_run(
    command: list[str],
    *,
    root: Path,
    measurement: Path,
    environment: dict[str, str] | None = None,
) -> tuple[subprocess.CompletedProcess[str], int, int]:
    measurement.unlink(missing_ok=True)
    started = time.monotonic_ns()
    wrapped = [
        "/usr/bin/time",
        "-f",
        "%M",
        "-o",
        str(measurement),
        *command,
    ]
    result = subprocess.run(
        wrapped,
        cwd=root,
        env=environment,
        capture_output=True,
        text=True,
        check=False,
    )
    elapsed_ms = (time.monotonic_ns() - started) // 1_000_000
    peak_kib = 0
    if measurement.is_file():
        try:
            peak_kib = int(measurement.read_text(encoding="utf-8").strip())
        except ValueError:
            peak_kib = 0
    return result, elapsed_ms, peak_kib * 1024


def _runtime_hash(root: Path) -> str:
    digest = hashlib.sha256()
    for relative in (
        "src/compiler/native_c/runtime/ark_native_runtime.h",
        "src/compiler/native_c/runtime/ark_native_runtime.c",
    ):
        digest.update((root / relative).read_bytes())
    return digest.hexdigest()


def _cache_key(
    root: Path,
    s2_runtime: Path,
    clang_path: str,
    clang_version: str,
    compile_flags: list[str],
    link_flags: list[str],
) -> str:
    identity = {
        "s2_compiler_artifact_hash": _sha256(s2_runtime),
        "compiler_source_fingerprint": _selfhost_source_fingerprint(root),
        "native_runtime_hash": _runtime_hash(root),
        "runtime_abi_version": RUNTIME_ABI_VERSION,
        "clang_path": clang_path,
        "clang_version": clang_version,
        "compile_flags": compile_flags,
        "link_flags": link_flags,
        "target_triple": "x86_64-unknown-linux-gnu",
        "backend_schema_version": BACKEND_SCHEMA_VERSION,
        "capability_table_version": CAPABILITY_TABLE_VERSION,
    }
    encoded = json.dumps(identity, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def _empty_receipt() -> dict[str, object]:
    return {
        "clang_peak_rss_bytes": 0,
        "executor_peak_rss_bytes": 0,
        "pipeline_peak_rss_bytes": 0,
        "executor_wall_time_ms": 0,
        "pipeline_wall_time_ms": 0,
        "s2_sha256": "",
        "s3_sha256": "",
        "determinism_run_1_sha256": "",
        "determinism_run_2_sha256": "",
        "clang_version": "",
        "runtime_abi_version": RUNTIME_ABI_VERSION,
        "cache_hit": False,
        "exit_code": 1,
    }


def run_native_executor(root: Path, *, build: bool, dry_run: bool) -> tuple[int, str]:
    """Build/cache the native compiler, produce s3 twice, and verify it."""
    if dry_run:
        return 0, "DRY-RUN: native C generation -> clang -> two native s3 runs -> equality"
    if not build:
        return 1, "native-executor requires --build until a verified cache exists"

    pipeline_started = time.monotonic_ns()
    output_dir = root / ".build/selfhost/native"
    output_dir.mkdir(parents=True, exist_ok=True)
    receipt_path = output_dir / "native-executor-receipt.json"
    receipt = _empty_receipt()

    s2 = root / ".build/selfhost/arukellt-s2.wasm"
    s2_runtime = root / ".build/selfhost/arukellt-s2-runtime.wasm"
    if not s2.is_file() or not s2_runtime.is_file():
        return 1, "native executor diagnostic: missing s2 artifacts; run selfhost build-compiler"
    receipt["s2_sha256"] = _sha256(s2)

    clang_path, toolchain = _toolchain()
    if clang_path is None:
        return 1, toolchain
    receipt["clang_version"] = toolchain

    compile_flags = ["-std=c99", "-O2"]
    link_flags: list[str] = []
    key = _cache_key(root, s2_runtime, clang_path, toolchain, compile_flags, link_flags)
    key_path = output_dir / "cache-key.txt"
    executable = output_dir / "arukellt-native"
    generated_c = output_dir / "compiler.c"
    cache_hit = (
        executable.is_file()
        and key_path.is_file()
        and key_path.read_text(encoding="utf-8").strip() == key
    )
    receipt["cache_hit"] = cache_hit

    clang_peak = 0
    pipeline_peak = 0
    if not cache_hit:
        workspace = _prepare_bootstrap_workspace(root)
        compiler_source = workspace / "src/compiler/main.ark"
        compiler_source_arg = str(compiler_source.relative_to(root))
        generated_c_arg = str(generated_c.relative_to(root))
        generation_cache_arg = str((output_dir / "ast-cache").relative_to(root))
        generation_environment = os.environ.copy()
        generation_environment["ARUKELLT_SELFHOST_WASM"] = str(s2_runtime)
        generation, _, generation_peak = _timed_run(
            [
                str(root / "scripts/run/arukellt-selfhost.sh"),
                "compile",
                compiler_source_arg,
                "--target",
                "native-cpp",
                "--output",
                generated_c_arg,
                "--cache-dir",
                generation_cache_arg,
            ],
            root=root,
            measurement=output_dir / "c-generation.maxrss",
            environment=generation_environment,
        )
        pipeline_peak = max(pipeline_peak, generation_peak)
        if generation.returncode != 0 or not generated_c.is_file():
            detail = (generation.stderr + generation.stdout)[-2000:]
            return 1, f"native C generation failed:\n{detail}"

        clang, _, clang_peak = _timed_run(
            [
                clang_path,
                *compile_flags,
                "-I",
                str(root / "src/compiler/native_c/runtime"),
                str(generated_c),
                str(root / "src/compiler/native_c/runtime/ark_native_runtime.c"),
                *link_flags,
                "-o",
                str(executable),
            ],
            root=root,
            measurement=output_dir / "clang.maxrss",
        )
        pipeline_peak = max(pipeline_peak, clang_peak)
        if clang.returncode != 0 or not executable.is_file():
            c_hash = _sha256(generated_c)
            detail = (clang.stderr + clang.stdout)[-4000:]
            return 1, (
                f"backend/toolchain failure: generated C `{generated_c}` "
                f"sha256={c_hash}\n{detail}"
            )
        key_path.write_text(key + "\n", encoding="utf-8")

    s3_first = output_dir / "arukellt-s3-native.wasm"
    s3_second = output_dir / "arukellt-s3-native-second.wasm"
    workspace = _prepare_bootstrap_workspace(root)
    compiler_source = workspace / "src/compiler/main.ark"

    executor_times: list[int] = []
    executor_peaks: list[int] = []
    executor_hashes: list[str] = []
    for run_index, output in enumerate((s3_first, s3_second), start=1):
        output.unlink(missing_ok=True)
        execution, elapsed, peak = _timed_run(
            [
                str(executable),
                "compile",
                str(compiler_source),
                "--target",
                "wasm32-gc",
                "--output",
                str(output),
                "--cache-dir",
                str(output_dir / f"native-ast-cache-{run_index}"),
            ],
            root=root,
            measurement=output_dir / f"executor-{run_index}.maxrss",
        )
        executor_times.append(elapsed)
        executor_peaks.append(peak)
        pipeline_peak = max(pipeline_peak, peak)
        if execution.returncode != 0 or not output.is_file():
            receipt["exit_code"] = execution.returncode or 1
            break
        _postprocess_selfhost_compiler_wasm(output, root)
        executor_hashes.append(_sha256(output))
        invalid = _reject_invalid_compiler_wasm(output)
        if invalid:
            receipt["exit_code"] = 1
            receipt["validation_error"] = invalid
            break

    if executor_hashes:
        receipt["s3_sha256"] = executor_hashes[0]
        receipt["determinism_run_1_sha256"] = executor_hashes[0]
    if len(executor_hashes) > 1:
        receipt["determinism_run_2_sha256"] = executor_hashes[1]

    receipt["clang_peak_rss_bytes"] = clang_peak
    receipt["executor_peak_rss_bytes"] = max(executor_peaks, default=0)
    receipt["pipeline_peak_rss_bytes"] = pipeline_peak
    receipt["executor_wall_time_ms"] = executor_times[0] if executor_times else 0
    receipt["pipeline_wall_time_ms"] = (
        time.monotonic_ns() - pipeline_started
    ) // 1_000_000

    is_valid = "validation_error" not in receipt and s3_second.is_file()
    deterministic = (
        receipt["determinism_run_1_sha256"]
        == receipt["determinism_run_2_sha256"]
        != ""
    )
    byte_equal = receipt["s2_sha256"] == receipt["s3_sha256"]
    performance_ok = int(receipt["executor_wall_time_ms"]) < 300_000
    memory_ok = int(receipt["executor_peak_rss_bytes"]) <= int(2.4 * 1024**3)
    succeeded = is_valid and deterministic and byte_equal and performance_ok and memory_ok
    receipt["exit_code"] = 0 if succeeded else 1
    receipt_path.write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )

    summary = [
        f"native executor receipt: {receipt_path.relative_to(root)}",
        f"s2 sha256: {receipt['s2_sha256']}",
        f"s3 sha256: {receipt['s3_sha256']}",
        f"deterministic: {deterministic}",
        f"byte equality: {byte_equal}",
        f"warm executor ms: {receipt['executor_wall_time_ms']}",
        f"executor peak RSS bytes: {receipt['executor_peak_rss_bytes']}",
    ]
    if "validation_error" in receipt:
        summary.append(f"s3 validation: {receipt['validation_error']}")
    return (0 if succeeded else 1), "\n".join(summary)
