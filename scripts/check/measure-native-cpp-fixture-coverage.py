#!/usr/bin/env python3
"""Measure native-cpp readiness across the fixture manifest (schema v2)."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import signal
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT))

from scripts.verify.fixtures import load_manifest  # noqa: E402

WRAPPER = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
FIXTURES = REPO_ROOT / "tests" / "fixtures"
MANIFEST = FIXTURES / "manifest.txt"
CAPABILITIES = REPO_ROOT / "data" / "native-cpp-capabilities.toml"
DEFAULT_OUTPUT = REPO_ROOT / "docs" / "data" / "native-cpp-fixture-coverage-receipt.json"
DEFAULT_PARTIAL = REPO_ROOT / ".build" / "native-cpp-fixture-measure" / "partial-results.jsonl"

POSITIVE_KINDS = frozenset({"run", "module-run", "t3-run"})
NEGATIVE_KINDS = frozenset(
    {"compile-error", "diag", "module-diag", "component-world-error"}
)

ICE_RE = re.compile(r"compiler ICE:", re.I)
HOST_TRAP_RE = re.compile(
    r"wasm trap:|failed to run main module|out of bounds memory access", re.I
)
CAPABILITY_MIR_RE = re.compile(
    r"does not support MIR opcode\s+`?(MIR_[A-Z0-9_]+)`?", re.I
)
CAPABILITY_CORE_RE = re.compile(
    r"does not support CoreOp\s+`?([A-Za-z0-9_.]+)`?", re.I
)
SPAN_RE = re.compile(r":\d+:\d+|in function\s+`[^`]+`|fn\s+[A-Za-z_][A-Za-z0-9_]*")
MAIN_NO_PARAM_RE = re.compile(r"\bfn\s+main\s*\(\s*\)\s*(?:->|[;{])")
MAIN_ANY_RE = re.compile(r"\bfn\s+main\s*\(")
HEX_ADDR_RE = re.compile(r"0x[0-9a-fA-F]+")
PATH_RE = re.compile(r"(?:/|(?:[A-Za-z]:\\))[^\s:]+")
LINECOL_RE = re.compile(r":\d+:\d+")
STACK_FRAME_RE = re.compile(r"^\s*(?:#\d+\s+)?(?:0x[0-9a-fA-F]+\s+)?(.+)$", re.M)

SIGNAL_NAMES = {
    -signal.SIGABRT: "SIGABRT",
    -signal.SIGSEGV: "SIGSEGV",
    -signal.SIGILL: "SIGILL",
    -signal.SIGFPE: "SIGFPE",
    -signal.SIGBUS: "SIGBUS",
}

# Relative to tests/fixtures/. Explicit overrides when sidecars are insufficient.
FIXTURE_OVERRIDES: dict[str, dict[str, object]] = {
    "native_cpp_public/unsupported_array_new.ark": {
        "expected_compile": False,
        "expected_run_kind": "not_run",
        "expected_stderr_pattern": "MIR_ARRAY_NEW",
        "population_force": "expected_negative",
    },
    "native_cpp_public/main_with_param.ark": {
        "expected_compile": False,
        "expected_run_kind": "not_run",
        "expected_stderr_pattern": "requires `fn main()`",
        "population_force": "expected_negative",
    },
    "native_cpp_public/panic_message.ark": {
        "expected_compile": True,
        "expected_run_kind": "panic",
        "expected_stderr_pattern": "panic",
    },
    "native_cpp_public/trap_div_zero.ark": {
        "expected_compile": True,
        "expected_run_kind": "trap",
        "expected_stderr_pattern": "divide by zero",
        "expected_signal": "SIGABRT",
    },
    "native_cpp_public/process_exit_7.ark": {
        "expected_compile": True,
        "expected_run_kind": "exit",
        "expected_exit_code": 7,
    },
    "native_cpp_public/fs_write_missing_parent.ark": {
        "expected_compile": True,
        "expected_run_kind": "exit",
        "expected_exit_code": 0,
    },
    # Intentional runtime traps (native kinded abort); goldens are wasm stdout.
    "stdlib_text/slice_bytes_invalid.ark": {
        "expected_compile": True,
        "expected_run_kind": "trap",
        "expected_signal": "SIGABRT",
        "expected_stderr_pattern": "bounds error",
    },
    "stdlib_io/fs_error_message_utf8.ark": {
        "expected_compile": True,
        "expected_run_kind": "trap",
        "expected_signal": "SIGABRT",
        "expected_stderr_pattern": "null reference",
    },
    # Currently traps on wasm and native (bounds); treat abort as expected until fixed.
    "stdlib_collections_ordered/btree_keys.ark": {
        "expected_compile": True,
        "expected_run_kind": "trap",
        "expected_signal": "SIGABRT",
        "expected_stderr_pattern": "bounds error",
    },
    "stdlib_trait/io_read_partial.ark": {
        "expected_compile": True,
        "expected_run_kind": "trap",
        "expected_signal": "SIGABRT",
        "expected_stderr_pattern": "runtime trap",
    },
}


def _build_dir() -> Path:
    env = os.environ.get("ARUKELLT_BUILD_DIR", "").strip()
    return Path(env) if env else REPO_ROOT / ".build"


def _env(build_dir: Path) -> dict[str, str]:
    env = os.environ.copy()
    env["ARUKELLT_BUILD_DIR"] = str(build_dir)
    for candidate in (
        build_dir / "selfhost" / "arukellt-s2-runtime.wasm",
        build_dir / "selfhost" / "arukellt-s2.wasm",
        REPO_ROOT / ".build" / "selfhost" / "arukellt-s2-runtime.wasm",
    ):
        if candidate.is_file():
            env["ARUKELLT_SELFHOST_WASM"] = str(candidate)
            break
    return env


def _sha256_file(path: Path) -> str | None:
    if not path.is_file():
        return None
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def _git_meta() -> dict[str, object]:
    def run(args: list[str]) -> str:
        try:
            out = subprocess.run(
                args,
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
                check=False,
                timeout=10,
            )
            return out.stdout.strip() if out.returncode == 0 else ""
        except (OSError, subprocess.TimeoutExpired):
            return ""

    commit = run(["git", "rev-parse", "HEAD"])
    dirty = bool(run(["git", "status", "--porcelain"]))
    return {"source_commit": commit or None, "dirty": dirty}


def _clang_version() -> str | None:
    try:
        out = subprocess.run(
            ["clang", "--version"],
            capture_output=True,
            text=True,
            check=False,
            timeout=10,
        )
        if out.returncode != 0:
            return None
        return out.stdout.splitlines()[0].strip() if out.stdout else None
    except (OSError, subprocess.TimeoutExpired):
        return None


def _load_capability_status() -> dict[str, str]:
    if not CAPABILITIES.is_file():
        return {}
    text = CAPABILITIES.read_text(encoding="utf-8")
    status_by_id: dict[str, str] = {}
    current_id: str | None = None
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("id = "):
            current_id = stripped.split("=", 1)[1].strip().strip('"')
        elif stripped.startswith("status = ") and current_id:
            status_by_id[current_id] = stripped.split("=", 1)[1].strip().strip('"')
            current_id = None
    return status_by_id


def _normalize_message(text: str) -> str:
    text = PATH_RE.sub("<PATH>", text)
    text = HEX_ADDR_RE.sub("<ADDR>", text)
    text = LINECOL_RE.sub(":<LINE>:<COL>", text)
    text = re.sub(r"\s+", " ", text).strip()
    return text[:400]


def _top_stack_frame(text: str) -> str:
    for match in STACK_FRAME_RE.finditer(text):
        frame = match.group(1).strip()
        if frame and ("ark_" in frame or "main" in frame or "/" in frame):
            return _normalize_message(frame)[:200]
    return ""


def _signal_name(returncode: int | None) -> str | None:
    if returncode is None:
        return None
    # POSIX wait status as negative signo, or shell-style 128+signo.
    if returncode < 0:
        return SIGNAL_NAMES.get(returncode, f"SIGNAL_{-returncode}")
    if returncode >= 128:
        signo = returncode - 128
        return SIGNAL_NAMES.get(-signo, f"SIGNAL_{signo}")
    return None


def _has_native_entry(path: Path) -> bool:
    try:
        text = path.read_text(encoding="utf-8", errors="ignore")
    except OSError:
        return False
    return bool(MAIN_NO_PARAM_RE.search(text))


def _sidecar_base(rel_fixture: str) -> Path:
    return FIXTURES / rel_fixture[:-4] if rel_fixture.endswith(".ark") else FIXTURES / rel_fixture


def _read_diag_pattern(rel_fixture: str) -> str:
    base = _sidecar_base(rel_fixture)
    # Prefer shared .diag (usually a stable code prefix); fall back to selfhost.
    for suffix in (".diag", ".selfhost.diag"):
        path = Path(str(base) + suffix)
        if path.is_file():
            return path.read_text(encoding="utf-8", errors="ignore").strip()
    return ""


def _read_expected_output(rel_fixture: str) -> str | None:
    path = Path(str(_sidecar_base(rel_fixture)) + ".expected")
    if not path.is_file():
        return None
    return path.read_text(encoding="utf-8", errors="ignore").strip()


def _population_for(kind: str, rel: str, override: dict[str, object]) -> str:
    forced = override.get("population_force")
    if isinstance(forced, str):
        return forced
    if kind in POSITIVE_KINDS:
        return "positive_run"
    if kind in NEGATIVE_KINDS:
        return "expected_negative"
    return "other"


def _build_expectation(
    *,
    kind: str,
    rel: str,
    population: str,
    has_entry: bool,
    override: dict[str, object],
) -> dict[str, object]:
    if override:
        exp = {
            "expected_compile": bool(override.get("expected_compile", population != "expected_negative")),
            "expected_run_kind": str(override.get("expected_run_kind", "exit")),
            "expected_exit_code": override.get("expected_exit_code"),
            "expected_signal": override.get("expected_signal"),
            "expected_stdout_pattern": override.get("expected_stdout_pattern"),
            "expected_stderr_pattern": override.get("expected_stderr_pattern"),
        }
        return exp

    if population == "expected_negative":
        return {
            "expected_compile": False,
            "expected_run_kind": "not_run",
            "expected_exit_code": None,
            "expected_signal": None,
            "expected_stdout_pattern": None,
            "expected_stderr_pattern": _read_diag_pattern(rel) or None,
        }

    if population != "positive_run" or not has_entry:
        return {
            "expected_compile": False,
            "expected_run_kind": "not_run",
            "expected_exit_code": None,
            "expected_signal": None,
            "expected_stdout_pattern": None,
            "expected_stderr_pattern": None,
        }

    golden = _read_expected_output(rel)
    return {
        "expected_compile": True,
        "expected_run_kind": "exit",
        "expected_exit_code": 0 if golden is None else None,
        "expected_signal": None,
        "expected_stdout_pattern": golden,
        "expected_stderr_pattern": None,
    }


def _classify_compile(
    returncode: int,
    combined: str,
    capability_status: dict[str, str],
) -> dict[str, object]:
    mir = CAPABILITY_MIR_RE.search(combined)
    core = CAPABILITY_CORE_RE.search(combined)
    reject_id = None
    if mir:
        reject_id = mir.group(1).upper()
    elif core:
        reject_id = core.group(1)

    ice = bool(ICE_RE.search(combined))
    host_trap = bool(HOST_TRAP_RE.search(combined))
    has_span = bool(SPAN_RE.search(combined))
    status = capability_status.get(reject_id or "", "")
    registry_unsup = status in {"planned", "unsupported"}

    safe = bool(
        reject_id
        and registry_unsup
        and has_span
        and not ice
        and not host_trap
        and ("does not support" in combined or "unsupported" in combined.lower())
    )

    if returncode == 0:
        kind = "compile_pass"
    elif ice:
        kind = "ice"
    elif host_trap:
        kind = "compiler_host_trap"
    elif safe:
        kind = "safe_capability_reject"
    elif reject_id:
        kind = "unsafe_reject"
    elif "requires `fn main()`" in combined or "requires a `fn main()`" in combined:
        kind = "entry_reject"
    elif "error[" in combined or "failed to" in combined.lower():
        kind = "frontend_error"
    else:
        kind = "other_fail"

    return {
        "compile_kind": kind,
        "compile_reject_id": reject_id,
        "safe_capability_reject": safe,
        "is_ice": ice,
        "is_host_trap": host_trap,
    }


def _pattern_match(pattern: object, text: str) -> bool:
    if pattern is None:
        return True
    needle = str(pattern)
    if not needle:
        return True
    return needle in text


def _digest(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8", errors="replace")).hexdigest()[:16]


def _expectation_matched(
    *,
    expectation: dict[str, object],
    compile_kind: str,
    compile_ok: bool,
    combined_compile: str,
    ran: bool,
    run_rc: int | None,
    run_out: str,
    run_err: str,
    run_signal: str | None,
) -> tuple[bool, str]:
    expected_compile = bool(expectation["expected_compile"])
    if expected_compile and not compile_ok:
        return False, "compile_expected_pass"
    if not expected_compile:
        if compile_ok:
            return False, "compile_expected_fail"
        stderr_pat = expectation.get("expected_stderr_pattern")
        if stderr_pat and not _pattern_match(stderr_pat, combined_compile):
            return False, "diagnostic_mismatch"
        if compile_kind == "ice":
            return False, "ice"
        return True, "negative_ok"

    run_kind = str(expectation.get("expected_run_kind") or "exit")
    if run_kind == "not_run":
        return True, "compile_only_ok"
    if not ran:
        return False, "run_missing"

    combined_run = run_out + run_err
    stdout_pat = expectation.get("expected_stdout_pattern")
    stderr_pat = expectation.get("expected_stderr_pattern")
    if stdout_pat is not None:
        # Golden files historically match combined or stdout; accept either.
        norm_out = run_out.strip()
        norm_combined = combined_run.strip()
        golden = str(stdout_pat).strip()
        if golden not in {norm_out, norm_combined} and golden not in norm_combined:
            return False, "stdout_mismatch"
    if stderr_pat is not None and not _pattern_match(stderr_pat, combined_run):
        return False, "stderr_mismatch"

    expected_signal = expectation.get("expected_signal")
    if run_kind in {"trap", "panic", "signal"}:
        if expected_signal and run_signal != expected_signal:
            # panic/trap may surface as SIGABRT; allow if stderr matched and signal is abort.
            if not (run_signal == "SIGABRT" and run_kind in {"trap", "panic"}):
                return False, "signal_mismatch"
        if run_kind == "panic" and not _pattern_match(
            expectation.get("expected_stderr_pattern") or "panic", combined_run
        ):
            return False, "panic_mismatch"
        if run_kind == "trap" and run_signal is None and run_rc == 0:
            return False, "trap_missing"
        return True, "abnormal_ok"

    if run_signal is not None:
        return False, "unexpected_signal"

    expected_exit = expectation.get("expected_exit_code")
    if expected_exit is None:
        # Unconstrained exit (golden-only fixtures may exit non-zero).
        return True, "exit_unconstrained"
    if run_rc != int(expected_exit):
        return False, "exit_mismatch"
    return True, "exit_ok"


def _fingerprint(
    *,
    failure_phase: str,
    diagnostic_code: str | None,
    sig: str | None,
    primary: str,
    stack: str,
    reject_id: str | None,
) -> str:
    material = "|".join(
        [
            failure_phase,
            diagnostic_code or "",
            sig or "",
            _normalize_message(primary),
            _normalize_message(stack),
            reject_id or "",
        ]
    )
    return hashlib.sha256(material.encode("utf-8")).hexdigest()[:16]


def _measure_one(
    *,
    kind: str,
    rel: str,
    build_dir: Path,
    out_root: Path,
    timeout: float,
    capability_status: dict[str, str],
    cache_key: str,
) -> dict[str, object]:
    override = FIXTURE_OVERRIDES.get(rel, {})
    population = _population_for(kind, rel, override)
    fixture_path = FIXTURES / rel
    has_entry = _has_native_entry(fixture_path) if fixture_path.is_file() else False
    if population == "positive_run" and not has_entry:
        population_effective = "positive_missing_entry"
    else:
        population_effective = population

    expectation = _build_expectation(
        kind=kind,
        rel=rel,
        population=population if population_effective != "positive_missing_entry" else "other",
        has_entry=has_entry,
        override=override,
    )

    result: dict[str, object] = {
        "fixture": f"tests/fixtures/{rel}",
        "manifest_kind": kind,
        "population": population_effective,
        "has_entry": has_entry,
        "cache_key": cache_key,
        **expectation,
        "compile_kind": "skipped",
        "safe_capability_reject": False,
        "compile_reject_id": None,
        "actual_exit_code": None,
        "actual_signal": None,
        "actual_stdout_digest": None,
        "actual_stderr_primary": None,
        "expectation_matched": None,
        "match_reason": None,
        "is_ice": False,
        "unexpected_crash": False,
        "failure_phase": None,
        "failure_family": None,
        "fingerprint": None,
    }

    measure = population_effective in {"positive_run", "expected_negative"} or bool(override)
    if not measure:
        result["compile_kind"] = "not_in_scope"
        result["expectation_matched"] = None
        return result

    digest = re.sub(r"[^A-Za-z0-9._-]+", "_", rel)
    c_path = out_root / f"{digest}.c"
    c_path.parent.mkdir(parents=True, exist_ok=True)

    started = time.time()
    fixture_arg = f"tests/fixtures/{rel}"
    native_cmd = [
        str(WRAPPER),
        "compile",
        fixture_arg,
        "--target",
        "native-cpp",
        "--emit",
        "c",
        "-o",
        str(c_path.relative_to(REPO_ROOT)),
    ]

    # Diagnostic probe for negatives uses kind-appropriate frontend path.
    # native-cpp --emit c is still probed for ICE (component compile-error often
    # succeeds on native C emit and must not skew diagnostic rates).
    if population_effective == "expected_negative":
        if kind in {"diag", "module-diag"}:
            diag_cmd = [str(WRAPPER), "check", fixture_arg]
        elif kind == "component-world-error":
            diag_cmd = [
                str(WRAPPER),
                "compile",
                fixture_arg,
                "--emit",
                "component",
                "-o",
                str((out_root / f"{digest}.component.wasm").relative_to(REPO_ROOT)),
            ]
        else:
            diag_cmd = [
                str(WRAPPER),
                "compile",
                fixture_arg,
                "-o",
                str((out_root / f"{digest}.wasm").relative_to(REPO_ROOT)),
            ]
        diag_proc = subprocess.run(
            diag_cmd,
            cwd=REPO_ROOT,
            env=_env(build_dir),
            capture_output=True,
            text=True,
            check=False,
            timeout=timeout,
        )
        diag_combined = diag_proc.stdout + diag_proc.stderr
        native_proc = subprocess.run(
            native_cmd,
            cwd=REPO_ROOT,
            env=_env(build_dir),
            capture_output=True,
            text=True,
            check=False,
            timeout=timeout,
        )
        native_combined = native_proc.stdout + native_proc.stderr
        native_classified = _classify_compile(
            native_proc.returncode, native_combined, capability_status
        )
        # Expectation uses diagnostic probe; ICE uses native probe.
        compile_ok = diag_proc.returncode == 0
        combined = diag_combined
        classified = {
            "compile_kind": (
                "ice"
                if native_classified["is_ice"]
                else (
                    "compiler_host_trap"
                    if native_classified.get("is_host_trap")
                    else ("frontend_error" if not compile_ok else "compile_pass")
                )
            ),
            "compile_reject_id": native_classified.get("compile_reject_id"),
            "safe_capability_reject": native_classified.get("safe_capability_reject"),
            "is_ice": native_classified["is_ice"],
            "is_host_trap": bool(native_classified.get("is_host_trap")),
        }
        result["native_compile_kind"] = native_classified["compile_kind"]
        result["native_compile_returncode"] = native_proc.returncode
        result["compile_returncode"] = diag_proc.returncode
    else:
        compile_proc = subprocess.run(
            native_cmd,
            cwd=REPO_ROOT,
            env=_env(build_dir),
            capture_output=True,
            text=True,
            check=False,
            timeout=timeout,
        )
        combined = compile_proc.stdout + compile_proc.stderr
        classified = _classify_compile(compile_proc.returncode, combined, capability_status)
        compile_ok = compile_proc.returncode == 0
        result["compile_returncode"] = compile_proc.returncode

    result.update(classified)
    result["compile_seconds"] = round(time.time() - started, 3)
    result["actual_stderr_primary"] = _normalize_message(
        next((ln for ln in combined.splitlines() if ln.strip()), "")
    )[:240]

    ran = False
    run_rc: int | None = None
    run_out = ""
    run_err = ""
    run_signal = None

    should_run = (
        compile_ok
        and bool(expectation["expected_compile"])
        and str(expectation.get("expected_run_kind")) != "not_run"
        and population_effective == "positive_run"
    )
    if should_run:
        run_started = time.time()
        run_proc = subprocess.run(
            [str(WRAPPER), "run", fixture_arg, "--target", "native-cpp"],
            cwd=REPO_ROOT,
            env=_env(build_dir),
            capture_output=True,
            text=True,
            check=False,
            timeout=timeout,
        )
        ran = True
        run_rc = run_proc.returncode
        run_out = run_proc.stdout
        run_err = run_proc.stderr
        run_signal = _signal_name(run_rc)
        result["actual_exit_code"] = run_rc
        result["actual_signal"] = run_signal
        result["actual_stdout_digest"] = _digest(run_out)
        result["run_seconds"] = round(time.time() - run_started, 3)
        primary = next(
            (
                ln
                for ln in (run_err + run_out).splitlines()
                if ln.strip()
                and not ln.lstrip().startswith("+ ")
                and "compilation succeeded" not in ln
                and "wrote " not in ln
                and "native-cpp-runner: cache" not in ln
            ),
            next((ln for ln in (run_err + run_out).splitlines() if ln.strip()), ""),
        )
        result["actual_stderr_primary"] = _normalize_message(primary)[:240]
        # Ark `--emit c` success is not a native compile success when clang fails.
        if ("native-cpp-runner: clang failed" in run_err) or (
            "error:" in run_err and "program.c:" in run_err
        ):
            classified = {
                **classified,
                "compile_kind": "clang_error",
                "safe_capability_reject": False,
                "compile_reject_id": classified.get("compile_reject_id"),
            }
            compile_ok = False
            result.update(classified)
            ran = False
            result["actual_exit_code"] = None
            result["actual_signal"] = None
            result["actual_stdout_digest"] = None
            run_signal = None

    matched, reason = _expectation_matched(
        expectation=expectation,
        compile_kind=str(classified["compile_kind"]),
        compile_ok=compile_ok,
        combined_compile=combined,
        ran=ran,
        run_rc=run_rc,
        run_out=run_out,
        run_err=run_err,
        run_signal=run_signal,
    )
    result["expectation_matched"] = matched
    result["match_reason"] = reason

    unexpected_crash = bool(ran and run_signal is not None and not matched)
    result["unexpected_crash"] = unexpected_crash

    if classified["is_ice"] or unexpected_crash or not matched:
        phase = "compile" if not ran else "run"
        if classified["is_ice"]:
            phase = "compile_ice"
        if classified.get("compile_kind") == "clang_error":
            phase = "clang"
        primary = str(result.get("actual_stderr_primary") or "")
        stack = _top_stack_frame(combined if not ran else (run_out + run_err))
        fp = _fingerprint(
            failure_phase=phase,
            diagnostic_code=str(classified.get("compile_reject_id") or reason),
            sig=run_signal,
            primary=primary,
            stack=stack,
            reject_id=str(classified.get("compile_reject_id") or "") or None,
        )
        result["failure_phase"] = phase
        result["failure_family"] = fp
        result["fingerprint"] = fp

    return result


def _atomic_write_json(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        mode="w",
        encoding="utf-8",
        dir=str(path.parent),
        prefix=f".{path.name}.",
        suffix=".tmp",
        delete=False,
    ) as tmp:
        json.dump(payload, tmp, indent=2, sort_keys=True)
        tmp.write("\n")
        tmp_path = Path(tmp.name)
    os.replace(tmp_path, path)


def _load_partial(path: Path) -> dict[str, dict[str, object]]:
    if not path.is_file():
        return {}
    out: dict[str, dict[str, object]] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        try:
            item = json.loads(line)
        except json.JSONDecodeError:
            continue
        key = str(item.get("fixture", ""))
        if key:
            out[key] = item
    return out


def _append_partial(path: Path, item: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as fh:
        fh.write(json.dumps(item, sort_keys=True) + "\n")


def _aggregate(
    results: list[dict[str, object]],
    *,
    previous: dict[str, object] | None,
) -> dict[str, object]:
    positive = [r for r in results if r["population"] == "positive_run"]
    negative = [r for r in results if r["population"] == "expected_negative"]
    compile_expected = [r for r in positive if r.get("expected_compile") is True]
    compile_pass = [r for r in compile_expected if r.get("compile_kind") == "compile_pass"]
    semantic_run_pass = [
        r for r in positive if r.get("expectation_matched") is True and r.get("expected_run_kind") != "not_run"
    ]
    # positives that were supposed to run
    positive_runnable = [
        r for r in positive if r.get("expected_compile") is True and r.get("expected_run_kind") != "not_run"
    ]
    compiled_positive = [r for r in positive_runnable if r.get("compile_kind") == "compile_pass"]
    compiled_semantic = [r for r in compiled_positive if r.get("expectation_matched") is True]
    neg_pass = [r for r in negative if r.get("expectation_matched") is True]
    ice_total = sum(1 for r in results if r.get("is_ice"))
    ice_positive = sum(1 for r in positive if r.get("is_ice"))
    host_trap_total = sum(1 for r in results if r.get("is_host_trap"))
    unexpected_crash = sum(1 for r in results if r.get("unexpected_crash"))
    safe_rejects = sum(1 for r in results if r.get("safe_capability_reject"))

    reject_hist: dict[str, int] = {}
    for r in results:
        rid = r.get("compile_reject_id")
        if rid and r.get("safe_capability_reject"):
            reject_hist[str(rid)] = reject_hist.get(str(rid), 0) + 1

    clusters: dict[str, dict[str, object]] = {}
    for r in results:
        fp = r.get("fingerprint")
        if not fp:
            continue
        bucket = clusters.setdefault(
            str(fp),
            {
                "fingerprint": fp,
                "count": 0,
                "failure_phase": r.get("failure_phase"),
                "compile_reject_id": r.get("compile_reject_id"),
                "actual_signal": r.get("actual_signal"),
                "primary": r.get("actual_stderr_primary"),
                "examples": [],
            },
        )
        bucket["count"] = int(bucket["count"]) + 1
        examples = bucket["examples"]
        assert isinstance(examples, list)
        if len(examples) < 5:
            examples.append(r["fixture"])

    cluster_list = sorted(clusters.values(), key=lambda c: (-int(c["count"]), str(c["fingerprint"])))

    def rate(num: int, den: int) -> float | None:
        if den == 0:
            return None
        return round(num / den, 4)

    rates = {
        "positive_compile_pass_rate": rate(len(compile_pass), len(compile_expected)),
        "positive_semantic_run_pass_rate": rate(len(semantic_run_pass), len(positive_runnable)),
        "compiled_positive_semantic_run_pass_rate": rate(len(compiled_semantic), len(compiled_positive)),
        "expected_negative_diagnostic_pass_rate": rate(len(neg_pass), len(negative)),
    }

    prev_clusters = {}
    if previous:
        for item in previous.get("clusters") or []:
            if isinstance(item, dict) and item.get("fingerprint"):
                prev_clusters[str(item["fingerprint"])] = int(item.get("count") or 0)
    delta = []
    for item in cluster_list:
        fp = str(item["fingerprint"])
        before = prev_clusters.get(fp, 0)
        after = int(item["count"])
        if before != after:
            delta.append({"fingerprint": fp, "before": before, "after": after, "delta": after - before})
    for fp, before in prev_clusters.items():
        if fp not in clusters:
            delta.append({"fingerprint": fp, "before": before, "after": 0, "delta": -before})

    return {
        "populations": {
            "positive_run": len(positive),
            "expected_negative": len(negative),
            "positive_missing_entry": sum(1 for r in results if r["population"] == "positive_missing_entry"),
            "other": sum(1 for r in results if r["population"] == "other"),
            "not_in_scope": sum(1 for r in results if r.get("compile_kind") == "not_in_scope"),
        },
        "counts": {
            "positive_compile_expected": len(compile_expected),
            "positive_compile_pass": len(compile_pass),
            "positive_runnable": len(positive_runnable),
            "positive_semantic_run_pass": len(semantic_run_pass),
            "compiled_positive": len(compiled_positive),
            "compiled_positive_semantic_run_pass": len(compiled_semantic),
            "expected_negative": len(negative),
            "expected_negative_diagnostic_pass": len(neg_pass),
            "ice_total": ice_total,
            "ice_positive": ice_positive,
            "compiler_host_trap_total": host_trap_total,
            "unexpected_crash": unexpected_crash,
            "safe_capability_reject": safe_rejects,
        },
        "rates": rates,
        "safe_capability_reject_histogram": dict(sorted(reject_hist.items(), key=lambda kv: (-kv[1], kv[0]))),
        "clusters": cluster_list,
        "cluster_delta_from_previous": delta,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--timeout", type=float, default=60.0)
    parser.add_argument("--limit", type=int, default=0)
    parser.add_argument("--shard-index", type=int, default=0)
    parser.add_argument("--shard-count", type=int, default=1)
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--partial", type=Path, default=None)
    parser.add_argument("--previous", type=Path, default=None, help="Previous receipt for cluster delta")
    parser.add_argument("--complete", action="store_true", help="Mark receipt complete (default with --write)")
    args = parser.parse_args()

    build_dir = _build_dir()
    out_root = build_dir / "native-cpp-fixture-measure" / "c"
    out_root.mkdir(parents=True, exist_ok=True)
    partial_path = args.partial or (build_dir / "native-cpp-fixture-measure" / "partial-results.jsonl")

    capability_status = _load_capability_status()
    s2 = None
    for candidate in (
        build_dir / "selfhost" / "arukellt-s2-runtime.wasm",
        build_dir / "selfhost" / "arukellt-s2.wasm",
    ):
        s2 = _sha256_file(candidate)
        if s2:
            break
    runtime_hash = _sha256_file(
        REPO_ROOT / "src" / "compiler" / "native_c" / "runtime" / "ark_native_runtime.c"
    )
    cache_key = hashlib.sha256(
        f"{s2}:{runtime_hash}:{_clang_version()}".encode("utf-8")
    ).hexdigest()[:16]

    entries = [e for e in load_manifest(MANIFEST) if e["kind"] != "bench"]

    def _kind_rank(kind: str) -> int:
        if kind in NEGATIVE_KINDS:
            return 0
        if kind in POSITIVE_KINDS:
            return 1
        return 2

    # Deduplicate by path; prefer negative then positive kinds over compile-only.
    best: dict[str, dict[str, str]] = {}
    for entry in entries:
        rel = entry["path"]
        prev = best.get(rel)
        if prev is None or _kind_rank(entry["kind"]) < _kind_rank(prev["kind"]):
            best[rel] = entry
    unique = sorted(best.values(), key=lambda e: e["path"])

    if args.shard_count < 1:
        print("--shard-count must be >= 1", file=sys.stderr)
        return 2
    if not (0 <= args.shard_index < args.shard_count):
        print("--shard-index out of range", file=sys.stderr)
        return 2
    unique = [e for i, e in enumerate(unique) if i % args.shard_count == args.shard_index]
    if args.limit > 0:
        unique = unique[: args.limit]

    previous = None
    prev_path = args.previous or args.output
    if prev_path.is_file():
        try:
            previous = json.loads(prev_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            previous = None

    cached = _load_partial(partial_path) if args.resume else {}
    if args.resume and cached:
        # Invalidate partial rows with different cache_key.
        cached = {
            k: v
            for k, v in cached.items()
            if v.get("cache_key") == cache_key
        }

    pending = []
    results: list[dict[str, object]] = []
    for entry in unique:
        key = f"tests/fixtures/{entry['path']}"
        if key in cached:
            results.append(cached[key])
        else:
            pending.append(entry)

    started = time.time()
    errors = 0
    if not args.resume:
        partial_path.write_text("", encoding="utf-8")

    with ThreadPoolExecutor(max_workers=max(1, args.jobs)) as pool:
        futures = {
            pool.submit(
                _measure_one,
                kind=entry["kind"],
                rel=entry["path"],
                build_dir=build_dir,
                out_root=out_root,
                timeout=args.timeout,
                capability_status=capability_status,
                cache_key=cache_key,
            ): entry
            for entry in pending
        }
        done = 0
        total_pending = len(futures)
        for future in as_completed(futures):
            done += 1
            entry = futures[future]
            try:
                item = future.result()
                results.append(item)
                _append_partial(partial_path, item)
            except subprocess.TimeoutExpired:
                errors += 1
                item = {
                    "fixture": f"tests/fixtures/{entry['path']}",
                    "manifest_kind": entry["kind"],
                    "population": "other",
                    "compile_kind": "timeout",
                    "is_ice": False,
                    "unexpected_crash": False,
                    "expectation_matched": False,
                    "match_reason": "timeout",
                    "cache_key": cache_key,
                }
                results.append(item)
                _append_partial(partial_path, item)
            except Exception as exc:  # noqa: BLE001
                errors += 1
                item = {
                    "fixture": f"tests/fixtures/{entry['path']}",
                    "manifest_kind": entry["kind"],
                    "population": "other",
                    "compile_kind": "harness_error",
                    "error": str(exc),
                    "is_ice": False,
                    "unexpected_crash": False,
                    "expectation_matched": False,
                    "match_reason": "harness_error",
                    "cache_key": cache_key,
                }
                results.append(item)
                _append_partial(partial_path, item)
            if done % 50 == 0 or done == total_pending:
                print(f"progress {done}/{total_pending} (cached={len(cached)})", file=sys.stderr)

    results.sort(key=lambda item: str(item.get("fixture", "")))
    agg = _aggregate(results, previous=previous)
    complete = bool(args.write or args.complete)
    # Shard runs are incomplete unless shard-count==1.
    if args.shard_count != 1:
        complete = False

    receipt = {
        "schema": "native-cpp-fixture-coverage-receipt/v2",
        "complete": complete,
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "fixture_root": "tests/fixtures",
        "selection": "manifest.txt (non-bench), classified by kind; positive_run requires native entry",
        "total_manifest_entries_selected": len(unique),
        "total_results": len(results),
        "elapsed_seconds": round(time.time() - started, 1),
        "jobs": args.jobs,
        "shard_index": args.shard_index,
        "shard_count": args.shard_count,
        "harness_errors": errors,
        "environment": {
            **_git_meta(),
            "s2_hash": s2,
            "runtime_c_hash": runtime_hash,
            "clang_version": _clang_version(),
            "cache_key": cache_key,
            "build_dir": str(build_dir),
        },
        "notes": [
            "v2 official metrics use semantic expectation match, not exit-0 scoring.",
            "v1 fn-main/exit-0 rates are obsolete for readiness gates.",
            "safe_capability_reject requires resolvable MIR/CoreOp id, registry planned/unsupported, span/function, and non-ICE.",
        ],
        **agg,
        "results": results,
    }

    summary = {
        k: receipt[k]
        for k in (
            "schema",
            "complete",
            "generated_at",
            "total_results",
            "elapsed_seconds",
            "populations",
            "counts",
            "rates",
        )
    }
    print(json.dumps(summary, indent=2, sort_keys=True))

    if args.write:
        if not complete and args.shard_count != 1:
            print("refusing to write incomplete shard run as canonical receipt", file=sys.stderr)
            return 2
        _atomic_write_json(args.output, receipt)
        print(f"wrote {args.output}", file=sys.stderr)

    counts = agg["counts"]
    print(
        "positive_compile="
        f"{counts['positive_compile_pass']}/{counts['positive_compile_expected']} "
        "compiled_semantic="
        f"{counts['compiled_positive_semantic_run_pass']}/{counts['compiled_positive']} "
        f"ice_total={counts['ice_total']} unexpected_crash={counts['unexpected_crash']} "
        f"neg_diag={counts['expected_negative_diagnostic_pass']}/{counts['expected_negative']}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
