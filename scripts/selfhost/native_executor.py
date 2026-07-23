"""Native C99 selfhost executor pipeline (ADR-049, RFC-008).

The native executor host is native, but S3 output must inherit the comparison
S2 build profile. Never hardcode wasm32-gc as the S3 target.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
import time
from pathlib import Path
from typing import Any

from selfhost.checks import (
    BOOTSTRAP_EMIT_TARGET,
    BOOTSTRAP_EMIT_WASI_VERSION,
    _postprocess_selfhost_compiler_wasm,
    _prepare_bootstrap_workspace,
    _reject_invalid_compiler_wasm,
    _selfhost_dir,
    _selfhost_source_fingerprint,
)

RUNTIME_ABI_VERSION = 1
BACKEND_SCHEMA_VERSION = 1
CAPABILITY_TABLE_VERSION = 1
MINIMUM_CLANG_VERSION = 14
S2_BUILD_PROFILE_NAME = "arukellt-s2.build-profile.json"
MEMORY_GATE_BYTES = int(2.4 * 1024**3)
WALL_GATE_MS = 300_000


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


def _read_smaps_rss_bytes(pid: int) -> int:
    """Return current Rss from smaps_rollup for a single process (not a tree sum)."""
    path = Path(f"/proc/{pid}/smaps_rollup")
    try:
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.startswith("Rss:"):
                parts = line.split()
                return int(parts[1]) * 1024
    except (OSError, ValueError, IndexError):
        return 0
    return 0


def _timed_run(
    command: list[str],
    *,
    root: Path,
    measurement: Path,
    environment: dict[str, str] | None = None,
) -> tuple[subprocess.CompletedProcess[str], int, int, int]:
    """Run command under /usr/bin/time and sample the child's smaps_rollup peak.

    Returns (result, wall_ms, time_v_max_rss_bytes, smaps_peak_rss_bytes).
    Parent manager RSS is never included; only the timed child is sampled.

    stdout/stderr go to side files (not pipes) so verbose compiler logs cannot
    stall the child on a full pipe buffer and inflate the wall gate.
    """
    measurement.unlink(missing_ok=True)
    stdout_path = measurement.with_suffix(measurement.suffix + ".stdout")
    stderr_path = measurement.with_suffix(measurement.suffix + ".stderr")
    stdout_path.unlink(missing_ok=True)
    stderr_path.unlink(missing_ok=True)
    wrapped = [
        "/usr/bin/time",
        "-f",
        "%e %M",
        "-o",
        str(measurement),
        *command,
    ]
    with stdout_path.open("w", encoding="utf-8") as stdout_file, stderr_path.open(
        "w", encoding="utf-8"
    ) as stderr_file:
        proc = subprocess.Popen(
            wrapped,
            cwd=root,
            env=environment,
            stdout=stdout_file,
            stderr=stderr_file,
            text=True,
        )
        smaps_peak = 0
        # /usr/bin/time is the direct child; the measured workload is its child.
        while proc.poll() is None:
            try:
                children = Path(f"/proc/{proc.pid}/task/{proc.pid}/children")
                candidates = [proc.pid]
                try:
                    for token in children.read_text(encoding="utf-8").split():
                        candidates.append(int(token))
                except OSError:
                    pass
                for pid in candidates:
                    smaps_peak = max(smaps_peak, _read_smaps_rss_bytes(pid))
            except OSError:
                pass
            # smaps_rollup on a multi-GiB native compile is expensive; keep this
            # as a coarse peak probe. `/usr/bin/time %M` remains the RSS gate.
            time.sleep(1.0)
        returncode = proc.wait()
    stdout = stdout_path.read_text(encoding="utf-8", errors="replace")
    stderr = stderr_path.read_text(encoding="utf-8", errors="replace")
    result = subprocess.CompletedProcess(
        wrapped, returncode or 0, stdout=stdout, stderr=stderr
    )
    elapsed_ms = 0
    peak_kib = 0
    if measurement.is_file():
        raw = measurement.read_text(encoding="utf-8").strip().split()
        try:
            if len(raw) >= 2:
                elapsed_ms = int(float(raw[0]) * 1000.0)
                peak_kib = int(raw[1])
            elif len(raw) == 1:
                # Backward compatible with older "%M"-only measurements.
                peak_kib = int(raw[0])
        except ValueError:
            elapsed_ms = 0
            peak_kib = 0
    return result, elapsed_ms, peak_kib * 1024, smaps_peak


def _runtime_hash(root: Path) -> str:
    digest = hashlib.sha256()
    for relative in (
        "src/compiler/native_c/runtime/ark_native_runtime.h",
        "src/compiler/native_c/runtime/ark_native_runtime.c",
    ):
        digest.update((root / relative).read_bytes())
    return digest.hexdigest()


def _default_s2_build_profile(s2: Path, fingerprint: str) -> dict[str, Any]:
    """Explicit profile for existing S2 artifacts built via the bootstrap chain.

    Regular stage-2 emission uses BOOTSTRAP_EMIT_TARGET / WASI (wasm32 + wasi-p1),
    not SELFHOST_TARGET. Do not invent wasm32-gc here.
    """
    return {
        "artifact": s2.name,
        "sha256": _sha256(s2) if s2.is_file() else "",
        "output_target": BOOTSTRAP_EMIT_TARGET,
        "wasi": BOOTSTRAP_EMIT_WASI_VERSION,
        "memory64": False,
        "wasm_gc": False,
        "optimization": "release",
        "source_fingerprint": fingerprint,
        "compiler_flags": [],
    }


def write_s2_build_profile(root: Path, s2: Path, *, fingerprint: str | None = None) -> Path:
    """Persist a machine-readable S2 output profile next to the artifact."""
    build_dir = _selfhost_dir(root)
    build_dir.mkdir(parents=True, exist_ok=True)
    path = build_dir / S2_BUILD_PROFILE_NAME
    fp = fingerprint if fingerprint is not None else _selfhost_source_fingerprint(root)
    profile = _default_s2_build_profile(s2, fp)
    path.write_text(json.dumps(profile, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def load_s2_build_profile(root: Path, s2: Path) -> tuple[dict[str, Any] | None, str]:
    """Load the S2 profile manifest; refuse ambiguous target guessing."""
    path = _selfhost_dir(root) / S2_BUILD_PROFILE_NAME
    if not path.is_file():
        return None, (
            "native executor diagnostic: missing S2 build-profile manifest at "
            f"{path.relative_to(root)}; rebuild with "
            "`python3 scripts/manager.py selfhost build-compiler` "
            "(or write an explicit profile — do not infer wasm32-gc)"
        )
    try:
        profile = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return None, f"native executor diagnostic: invalid S2 build-profile manifest: {exc}"

    required = ("output_target", "wasi", "memory64", "wasm_gc", "sha256")
    missing = [key for key in required if key not in profile]
    if missing:
        return None, (
            "native executor diagnostic: S2 build-profile missing keys: "
            + ", ".join(missing)
        )

    target = str(profile["output_target"])
    if not target or target == "native-cpp" or target == "native-llvm":
        return None, (
            f"native executor diagnostic: S2 build-profile output_target `{target}` "
            "is not a Wasm comparison target"
        )

    expected_sha = str(profile.get("sha256", ""))
    if expected_sha and s2.is_file() and _sha256(s2) != expected_sha:
        return None, (
            "native executor diagnostic: S2 build-profile sha256 does not match "
            f"{s2.name}; rebuild S2 or refresh the manifest"
        )
    return profile, ""


def _cache_key(
    root: Path,
    s2_runtime: Path,
    clang_path: str,
    clang_version: str,
    compile_flags: list[str],
    link_flags: list[str],
    profile: dict[str, Any],
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
        "s2_output_target": profile["output_target"],
        "s2_wasi": profile["wasi"],
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
        "output_target": "",
        "wasi": "",
        "memory64": False,
        "wasm_gc": False,
        "time_v_max_rss_bytes": 0,
        "smaps_rollup_peak_rss_bytes": 0,
        "runtime_requested_bytes": 0,
        "runtime_committed_bytes": 0,
        "runtime_live_bytes": 0,
        "runtime_collection_count": 0,
        "runtime_reclaimed_bytes": 0,
        "gc_object_bytes": 0,
        "gc_string_buffer_bytes": 0,
        "gc_vec_buffer_bytes": 0,
        "gc_object_table_bytes": 0,
        "gc_root_frame_bytes": 0,
        "gc_live_object_count": 0,
        "gc_object_table_capacity": 0,
        "gc_collection_count": 0,
        "gc_reclaimed_object_bytes": 0,
        "gc_reclaimed_side_buffer_bytes": 0,
        "memory_gate_passed": False,
        "high_rss_override": False,
    }


def _merge_gc_stats(receipt: dict[str, object], stats_path: Path) -> None:
    """Merge runtime GC stats JSON written by ark_rt_shutdown into the receipt."""
    if not stats_path.is_file():
        return
    try:
        payload = json.loads(stats_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return
    for key in (
        "gc_object_bytes",
        "gc_string_buffer_bytes",
        "gc_vec_buffer_bytes",
        "gc_object_table_bytes",
        "gc_root_frame_bytes",
        "gc_live_object_count",
        "gc_object_table_capacity",
        "gc_collection_count",
        "gc_reclaimed_object_bytes",
        "gc_reclaimed_side_buffer_bytes",
        "runtime_requested_bytes",
        "runtime_committed_bytes",
        "runtime_live_bytes",
        "runtime_collection_count",
        "runtime_reclaimed_bytes",
    ):
        if key in payload:
            receipt[key] = payload[key]


def run_native_executor(
    root: Path,
    *,
    build: bool,
    dry_run: bool,
    allow_high_rss: bool = False,
) -> tuple[int, str]:
    """Build/cache the native compiler, produce s3 twice, and verify it."""
    if dry_run:
        return 0, "DRY-RUN: native C generation -> clang -> two native s3 runs -> equality"
    if not build:
        return 1, "native-executor requires --build until a verified cache exists"

    pipeline_started = time.monotonic_ns()
    build_dir = _selfhost_dir(root)
    output_dir = build_dir / "native"
    output_dir.mkdir(parents=True, exist_ok=True)
    receipt_path = output_dir / "native-executor-receipt.json"
    receipt = _empty_receipt()

    s2 = build_dir / "arukellt-s2.wasm"
    s2_runtime = build_dir / "arukellt-s2-runtime.wasm"
    if not s2.is_file() or not s2_runtime.is_file():
        return 1, "native executor diagnostic: missing s2 artifacts; run selfhost build-compiler"
    receipt["s2_sha256"] = _sha256(s2)

    profile, profile_error = load_s2_build_profile(root, s2)
    if profile is None:
        return 1, profile_error
    output_target = str(profile["output_target"])
    wasi = str(profile["wasi"])
    receipt["output_target"] = output_target
    receipt["wasi"] = wasi
    receipt["memory64"] = bool(profile["memory64"])
    receipt["wasm_gc"] = bool(profile["wasm_gc"])

    clang_path, toolchain = _toolchain()
    if clang_path is None:
        return 1, toolchain
    receipt["clang_version"] = toolchain

    compile_flags = ["-std=c99", "-O2"]
    link_flags: list[str] = []
    key = _cache_key(
        root, s2_runtime, clang_path, toolchain, compile_flags, link_flags, profile
    )
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
        generation, _, generation_peak, _generation_smaps = _timed_run(
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

        clang, _, clang_peak, _clang_smaps = _timed_run(
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

    # Smoke: --help before full S3.
    help_run, _, help_peak, _help_smaps = _timed_run(
        [str(executable), "--help"],
        root=root,
        measurement=output_dir / "native-help.maxrss",
    )
    pipeline_peak = max(pipeline_peak, help_peak)
    if help_run.returncode != 0:
        receipt["exit_code"] = help_run.returncode or 1
        receipt_path.write_text(
            json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        detail = (help_run.stderr + help_run.stdout)[-1000:]
        return 1, f"arukellt-native --help failed:\n{detail}"

    s3_first = output_dir / "arukellt-s3-native.wasm"
    s3_second = output_dir / "arukellt-s3-native-second.wasm"
    workspace = _prepare_bootstrap_workspace(root)
    compiler_source = workspace / "src/compiler/main.ark"

    executor_times: list[int] = []
    executor_peaks: list[int] = []
    executor_smaps_peaks: list[int] = []
    executor_hashes: list[str] = []
    for run_index, output in enumerate((s3_first, s3_second), start=1):
        output.unlink(missing_ok=True)
        stats_path = output_dir / f"executor-{run_index}.gc-stats.json"
        stats_path.unlink(missing_ok=True)
        run_env = os.environ.copy()
        run_env["ARUKELLT_NATIVE_GC_STATS_PATH"] = str(stats_path)
        # Strict lane enables GC for the RSS gate. --allow-high-rss prefers the
        # arena path so warm wall stays under 5 minutes while RSS work continues.
        if "ARUKELLT_NATIVE_GC" not in run_env:
            run_env["ARUKELLT_NATIVE_GC"] = "0" if allow_high_rss else "1"
        execution, elapsed, peak, smaps_peak = _timed_run(
            [
                str(executable),
                "compile",
                str(compiler_source),
                "--target",
                output_target,
                "--wasi-version",
                wasi,
                "--output",
                str(output),
                "--cache-dir",
                str(output_dir / f"native-ast-cache-{run_index}"),
            ],
            root=root,
            measurement=output_dir / f"executor-{run_index}.maxrss",
            environment=run_env,
        )
        executor_times.append(elapsed)
        executor_peaks.append(peak)
        executor_smaps_peaks.append(smaps_peak)
        pipeline_peak = max(pipeline_peak, peak)
        _merge_gc_stats(receipt, stats_path)
        if execution.returncode != 0 or not output.is_file():
            receipt["exit_code"] = execution.returncode or 1
            detail = (execution.stderr + execution.stdout)[-2000:]
            receipt["executor_error"] = detail
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
    receipt["time_v_max_rss_bytes"] = max(executor_peaks, default=0)
    receipt["smaps_rollup_peak_rss_bytes"] = max(executor_smaps_peaks, default=0)
    receipt["pipeline_peak_rss_bytes"] = pipeline_peak
    # First run pays process/cold-cache startup; the wall gate uses the second
    # determinism run as the warm sample when present.
    cold_ms = executor_times[0] if executor_times else 0
    warm_ms = executor_times[-1] if executor_times else 0
    receipt["executor_cold_wall_time_ms"] = cold_ms
    receipt["executor_wall_time_ms"] = warm_ms
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
    performance_ok = int(receipt["executor_wall_time_ms"]) < WALL_GATE_MS
    memory_ok = int(receipt["executor_peak_rss_bytes"]) <= MEMORY_GATE_BYTES
    receipt["memory_gate_passed"] = memory_ok
    receipt["high_rss_override"] = bool(allow_high_rss) and not memory_ok
    correctness_ok = is_valid and deterministic and byte_equal and performance_ok
    if allow_high_rss:
        succeeded = correctness_ok
    else:
        succeeded = correctness_ok and memory_ok
    receipt["exit_code"] = 0 if succeeded else 1
    receipt_path.write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )

    summary = [
        f"native executor receipt: {receipt_path}",
        f"output profile: target={output_target} wasi={wasi} "
        f"memory64={profile['memory64']} wasm_gc={profile['wasm_gc']}",
        f"s2 sha256: {receipt['s2_sha256']}",
        f"s3 sha256: {receipt['s3_sha256']}",
        f"deterministic: {deterministic}",
        f"byte equality: {byte_equal}",
        f"cold executor ms: {cold_ms}",
        f"warm executor ms: {receipt['executor_wall_time_ms']}",
        f"executor peak RSS bytes: {receipt['executor_peak_rss_bytes']}",
        f"memory gate passed: {memory_ok}",
        f"high_rss_override: {receipt['high_rss_override']}",
        (
            f"gc stats: objects={receipt['gc_live_object_count']} "
            f"object_bytes={receipt['gc_object_bytes']} "
            f"string_buf={receipt['gc_string_buffer_bytes']} "
            f"vec_buf={receipt['gc_vec_buffer_bytes']} "
            f"table_bytes={receipt['gc_object_table_bytes']} "
            f"collections={receipt['gc_collection_count']}"
        ),
    ]
    if allow_high_rss and not memory_ok and correctness_ok:
        summary.insert(
            0,
            "WARNING: executor RSS exceeds 2.4 GiB gate; --allow-high-rss opted in "
            "(CI must not use this; current-state stays scaffold; memory_gate_passed=false)",
        )
    if "validation_error" in receipt:
        summary.append(f"s3 validation: {receipt['validation_error']}")
    if "executor_error" in receipt:
        summary.append(f"executor error: {receipt['executor_error']}")
    return (0 if succeeded else 1), "\n".join(summary)
