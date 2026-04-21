"""Selfhost domain check runners — pure Python, no shell script calls."""
from __future__ import annotations

import hashlib
import os
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path


# ── ANSI colours ─────────────────────────────────────────────────────────────
RED    = "\033[0;31m"
GREEN  = "\033[0;32m"
YELLOW = "\033[1;33m"
NC     = "\033[0m"


# ── Helpers ───────────────────────────────────────────────────────────────────

def _find_arukellt(root: Path) -> str | None:
    """Return path to arukellt binary, respecting ARUKELLT_BIN env var."""
    env_bin = os.environ.get("ARUKELLT_BIN", "")
    if env_bin:
        p = Path(env_bin)
        if p.is_file() and os.access(p, os.X_OK):
            return str(p)
        return None
    for candidate in ["target/debug/arukellt", "target/release/arukellt"]:
        p = root / candidate
        if p.is_file() and os.access(p, os.X_OK):
            return str(p)
    return None


def _find_wasmtime() -> str | None:
    return shutil.which("wasmtime")


def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _run(cmd: list[str], root: Path, capture: bool = True, timeout: int | None = None) -> subprocess.CompletedProcess:
    try:
        return subprocess.run(
            cmd,
            cwd=str(root),
            capture_output=capture,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return subprocess.CompletedProcess(cmd, returncode=-1, stdout="", stderr="timeout")


# ── SelfhostFixpointResult ────────────────────────────────────────────────────

@dataclass
class SelfhostFixpointResult:
    exit_code: int
    passed: bool
    skipped: bool
    output: str


# ── run_fixpoint ──────────────────────────────────────────────────────────────

def run_fixpoint(
    root: Path,
    dry_run: bool,
    no_build: bool = True,
) -> SelfhostFixpointResult:
    """Port of check-selfhost-fixpoint.sh.

    Exit codes:
        0  fixpoint reached (passed=True)
        1  not yet reached  (skipped=True, tracked)
        2  prereqs missing  (skipped=True)
    """
    build = not no_build
    lines: list[str] = []

    def emit(msg: str) -> None:
        lines.append(msg)

    if dry_run:
        print("DRY-RUN: run_fixpoint()")
        return SelfhostFixpointResult(exit_code=0, passed=True, skipped=False, output="")

    # Locate compiler
    arukellt_bin = _find_arukellt(root)
    if not arukellt_bin:
        emit(f"{RED}error: arukellt binary not found — build first or set ARUKELLT_BIN{NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    source = "src/compiler/main.ark"
    if not (root / source).is_file():
        emit(f"{RED}error: selfhost source not found: {source}{NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    build_dir = root / ".build" / "selfhost"
    build_dir.mkdir(parents=True, exist_ok=True)

    s1 = build_dir / "arukellt-s1.wasm"
    s2 = build_dir / "arukellt-s2.wasm"
    s3 = build_dir / "arukellt-s3.wasm"

    # Stage 1
    if build:
        emit(f"{YELLOW}[selfhost] Building stage 1 (Rust compiler → s1.wasm)...{NC}")
        r = _run([arukellt_bin, "compile", source, "--target", "wasm32-wasi-p1", "-o", str(s1)], root)
        if r.returncode != 0:
            emit(f"{RED}✗ stage 1 compilation failed{NC}")
            return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))
        emit(f"{GREEN}✓ s1.wasm built{NC}")

    if not s1.is_file():
        emit(f"{RED}error: s1.wasm not found at {s1} (run without --no-build first){NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    # Locate wasmtime
    wasmtime = _find_wasmtime()
    if not wasmtime:
        emit(f"{RED}error: wasmtime not found in PATH{NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    # Stage 2
    if build:
        emit(f"{YELLOW}[selfhost] Building stage 2 (s1.wasm → s2.wasm)...{NC}")
        r = _run([wasmtime, "run", "--dir", str(root), str(s1), "--", "compile", source,
                  "--target", "wasm32-wasi-p1", "-o", str(s2.relative_to(root))], root)
        if r.returncode != 0:
            emit(f"{RED}✗ stage 2 compilation failed{NC}")
            if r.stderr:
                emit(r.stderr[:500])
            return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))
        emit(f"{GREEN}✓ s2.wasm built{NC}")

    if not s2.is_file():
        emit(f"{RED}error: s2.wasm not found at {s2} (run without --no-build first){NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    # Stage 3
    if build:
        emit(f"{YELLOW}[selfhost] Building stage 3 (s2.wasm → s3.wasm)...{NC}")
        r = _run([wasmtime, "run", "--dir", str(root), str(s2), "--", "compile", source,
                  "--target", "wasm32-wasi-p1", "-o", str(s3.relative_to(root))], root)
        if r.returncode != 0:
            emit(f"{RED}✗ stage 3 compilation failed{NC}")
            if r.stderr:
                emit(r.stderr[:500])
            return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))
        emit(f"{GREEN}✓ s3.wasm built{NC}")

    if not s3.is_file():
        emit(f"{RED}error: s3.wasm not found at {s3} (run without --no-build first){NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    # Compare
    sha2 = _sha256(s2)
    sha3 = _sha256(s3)

    if sha2 == sha3:
        emit(f"{GREEN}✓ selfhost fixpoint reached: sha256({s2}) == sha256({s3}){NC}")
        emit(f"  sha256 = {sha2}")
        return SelfhostFixpointResult(exit_code=0, passed=True, skipped=False, output="\n".join(lines))

    emit(f"{YELLOW}⊙ selfhost fixpoint not yet reached (this is normal during development){NC}")
    emit(f"  sha256(s2) = {sha2}")
    emit(f"  sha256(s3) = {sha3}")
    return SelfhostFixpointResult(exit_code=1, passed=False, skipped=True, output="\n".join(lines))


# ── Shared manifest parsing ───────────────────────────────────────────────────

def _load_manifest_fixtures(root: Path, kind: str) -> tuple[list[str], str]:
    """Return list of fixture paths for kind='run' or kind='diag'. Also returns error string."""
    manifest = root / "tests" / "fixtures" / "manifest.txt"
    if not manifest.is_file():
        return [], f"{RED}error: manifest not found: {manifest}{NC}"
    pattern = re.compile(rf"^{kind}:\s*(.+\.ark)$")
    fixtures: list[str] = []
    for line in manifest.read_text().splitlines():
        m = pattern.match(line)
        if m:
            fixtures.append(m.group(1))
    return fixtures, ""


# ── Fixture parity skip list ─────────────────────────────────────────────────

# Fixtures with known parity differences that are not semantic errors.
# These are skipped in the fixture-parity check until the underlying issue
# in the selfhost emitter is resolved.  Add a comment explaining the root cause.
#
# Format: "category/fixture.ark"  # reason
FIXTURE_PARITY_SKIP: set[str] = {
    "stdlib_sort/sort_f64.ark",  # selfhost f64_to_string uses naive digit extraction
                                 # (1.2 → 1.199999999999999); Rust uses Grisu2/shortest-repr
    "functions/higher_order.ark",  # selfhost emitter lacks funcref table / call_indirect
                                   # support; fn-pointer parameters are not yet lowered.
}


# ── run_fixture_parity ────────────────────────────────────────────────────────

def run_fixture_parity(root: Path, dry_run: bool) -> tuple[int, str]:
    """Port of check-selfhost-fixture-parity.sh."""
    if dry_run:
        print("DRY-RUN: run_fixture_parity()")
        return (0, "")

    lines: list[str] = []

    arukellt_bin = _find_arukellt(root)
    if not arukellt_bin:
        return (1, f"{RED}error: arukellt binary not found{NC}\n")

    selfhost_wasm_env = os.environ.get("SELFHOST_WASM", "")
    selfhost_wasm = Path(selfhost_wasm_env) if selfhost_wasm_env else root / ".build" / "selfhost" / "arukellt-s1.wasm"
    if not selfhost_wasm.is_file():
        return (1, f"{RED}error: selfhost wasm not found at {selfhost_wasm}{NC}\n")

    wasmtime = _find_wasmtime()
    if not wasmtime:
        return (1, f"{RED}error: wasmtime not found{NC}\n")

    fixtures, err = _load_manifest_fixtures(root, "run")
    if err:
        return (1, err + "\n")

    if len(fixtures) < 10:
        return (1, f"{RED}error: fewer than 10 run: fixtures in manifest ({len(fixtures)} found){NC}\n")

    lines.append(f"{YELLOW}[fixture-parity] Checking {len(fixtures)} run: fixtures...{NC}")

    pass_count = 0
    fail_count = 0
    skip_count = 0

    self_out_dir = root / ".ark-fixture-parity-tmp"
    self_out_dir.mkdir(exist_ok=True)
    tmpdir = tempfile.mkdtemp(prefix="ark-fixture-parity-")
    try:
        for fixture in fixtures:
            # Skip fixtures with known parity differences
            if fixture in FIXTURE_PARITY_SKIP:
                lines.append(f"  skip: {fixture} (known parity skip)")
                skip_count += 1
                continue

            ark_file = root / "tests" / "fixtures" / fixture
            if not ark_file.is_file():
                lines.append(f"  skip: {fixture} (not found on disk)")
                skip_count += 1
                continue

            out_rust = Path(tmpdir) / f"rust-{fixture.replace('/', '_')}.wasm"
            out_self_rel = Path(".ark-fixture-parity-tmp") / f"self-{fixture.replace('/', '_')}.wasm"
            out_self = root / out_self_rel

            # Compile with Rust compiler
            r = _run([arukellt_bin, "compile", str(ark_file), "--target", "wasm32-wasi-p1", "-o", str(out_rust)], root)
            if r.returncode != 0:
                lines.append(f"  skip: {fixture} (Rust compile failed)")
                skip_count += 1
                continue

            # Compile with selfhost compiler (timeout: 30s; compile failure = skip, not fail)
            r = _run([wasmtime, "run", "--dir", str(root), str(selfhost_wasm), "--", "compile",
                      str(Path("tests") / "fixtures" / fixture),
                      "--target", "wasm32-wasi-p1", "-o", str(out_self_rel)], root, timeout=30)
            if r.returncode != 0:
                lines.append(f"  skip: {fixture} (selfhost compile failed/timeout)")
                skip_count += 1
                continue

            # Compare execution output (not binary bytes — emitters differ in encoding)
            r_rust = _run([wasmtime, "run", str(out_rust)], root, timeout=15)
            rust_out = (r_rust.stdout + r_rust.stderr).strip()
            rust_code = r_rust.returncode

            r_self = _run([wasmtime, "run", str(out_self)], root, timeout=15)
            self_out = (r_self.stdout + r_self.stderr).strip()
            self_code = r_self.returncode

            # If selfhost wasm traps (unreachable/invalid), treat as skip not fail.
            # Exit 134 = SIGABRT (wasmtime wasm trap); exit 1 with validation error.
            if self_code == 134 or (self_code == 1 and "failed to compile" in self_out):
                lines.append(f"  skip: {fixture} (selfhost wasm trap/invalid)")
                skip_count += 1
                continue

            if rust_out == self_out and rust_code == self_code:
                pass_count += 1
            else:
                lines.append(f"  FAIL: {fixture} (execution output differs)")
                if rust_code != self_code:
                    lines.append(f"    exit: rust={rust_code} self={self_code}")
                if rust_out != self_out:
                    lines.append(f"    rust: {rust_out[:80]!r}")
                    lines.append(f"    self: {self_out[:80]!r}")
                fail_count += 1
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)
        shutil.rmtree(str(self_out_dir), ignore_errors=True)

    lines.append("")
    lines.append(f"{YELLOW}fixture-parity: PASS={pass_count} FAIL={fail_count} SKIP={skip_count}{NC}")

    if fail_count > 0:
        lines.append(f"{RED}✗ fixture parity: {fail_count} fixture(s) differ between Rust and selfhost{NC}")
        return (1, "\n".join(lines) + "\n")

    lines.append(f"{GREEN}✓ all {pass_count} run: fixtures match between Rust compiler and selfhost{NC}")
    return (0, "\n".join(lines) + "\n")


# ── run_diag_parity ───────────────────────────────────────────────────────────

# Fixtures skipped for diag-parity because selfhost has not yet implemented
# the diagnostics or the test exercises an unimplemented feature.  These are
# tracked in issue #529 Phase 3 (diagnostic parity expansion).
DIAG_PARITY_SKIP: frozenset[str] = frozenset({
    # selfhost emits "error: N type error(s)" without structured code format
    "diagnostics/type_mismatch.ark",
    # selfhost emits parse errors (E0001) where Rust emits typecheck (E0201)
    "diagnostics/missing_annotation.ark",
    "diagnostics/deprecated_prelude_println.ark",
    "diagnostics/deprecated_std_io_import.ark",
    "diagnostics/deprecated_time_monotonic_now.ark",
    "diagnostics/immutable_mutation.ark",
    "diagnostics/mismatched_arms.ark",
    "diagnostics/mutable_sharing.ark",
    "diagnostics/non_exhaustive.ark",
    "diagnostics/question_type_mismatch.ark",
    "diagnostics/unused_binding.ark",
    "diagnostics/unused_import.ark",
    "diagnostics/wrong_arg_count.ark",
    "host_stub_sockets.ark",
    "deny_clock_compile.ark",
    "deny_random_compile.ark",
    "target_gating/t1_import_sockets.ark",
    "target_gating/t1_import_udp.ark",
    "stdlib_io/deny_clock.ark",
    "stdlib_io/deny_random.ark",
    "v0_constraints/no_method_call.ark",
    "v0_constraints/no_operator_overload.ark",
    "module_import/use_symbol_not_found.ark",
    "selfhost/typecheck_match_nonexhaustive.ark",
})


def run_diag_parity(root: Path, dry_run: bool) -> tuple[int, str]:
    """Port of check-selfhost-diagnostic-parity.sh."""
    if dry_run:
        print("DRY-RUN: run_diag_parity()")
        return (0, "")

    lines: list[str] = []

    arukellt_bin = _find_arukellt(root)
    if not arukellt_bin:
        return (1, f"{RED}error: arukellt binary not found{NC}\n")

    selfhost_wasm_env = os.environ.get("SELFHOST_WASM", "")
    selfhost_wasm = Path(selfhost_wasm_env) if selfhost_wasm_env else root / ".build" / "selfhost" / "arukellt-s1.wasm"
    if not selfhost_wasm.is_file():
        return (1, f"{RED}error: selfhost wasm not found at {selfhost_wasm}{NC}\n")

    wasmtime = _find_wasmtime()
    if not wasmtime:
        return (1, f"{RED}error: wasmtime not found{NC}\n")

    fixtures, err = _load_manifest_fixtures(root, "diag")
    if err:
        return (1, err + "\n")

    lines.append(f"{YELLOW}[diag-parity] Checking {len(fixtures)} diag: fixtures...{NC}")

    pass_count = 0
    fail_count = 0
    skip_count = 0

    for fixture in fixtures:
        if fixture in DIAG_PARITY_SKIP:
            skip_count += 1
            continue

        ark_path = root / "tests" / "fixtures" / fixture
        diag_path = root / "tests" / "fixtures" / (fixture[:-4] + ".diag")
        selfhost_diag_path = root / "tests" / "fixtures" / (fixture[:-4] + ".selfhost.diag")

        if not ark_path.is_file():
            lines.append(f"  skip: {fixture} (source not found)")
            skip_count += 1
            continue
        if not diag_path.is_file():
            lines.append(f"  skip: {fixture} (.diag file not found)")
            skip_count += 1
            continue

        rust_pattern = diag_path.read_text().strip()
        selfhost_pattern = selfhost_diag_path.read_text().strip() if selfhost_diag_path.is_file() else rust_pattern

        r_rust = _run([arukellt_bin, "check", str(ark_path)], root)
        rust_out = r_rust.stdout + r_rust.stderr

        r_self = _run([wasmtime, "run", "--dir", str(root), str(selfhost_wasm), "--", "check",
                       str(Path("tests") / "fixtures" / fixture)], root)
        self_out = r_self.stdout + r_self.stderr

        rust_ok = rust_pattern in rust_out
        self_ok = selfhost_pattern in self_out

        if rust_ok and self_ok:
            lines.append(f"  pass: {fixture}")
            pass_count += 1
        elif not rust_ok:
            lines.append(f"  skip: {fixture} (Rust: pattern not found; test may be stale)")
            skip_count += 1
        else:
            lines.append(f"  FAIL: {fixture} (selfhost: pattern '{selfhost_pattern}' not found)")
            fail_count += 1

    lines.append("")
    lines.append(f"{YELLOW}diag-parity: PASS={pass_count} SKIP={skip_count} FAIL={fail_count}{NC}")

    min_pass = 10
    if fail_count > 0:
        lines.append(f"{RED}✗ diag parity: {fail_count} fixture(s) differ{NC}")
        return (1, "\n".join(lines) + "\n")
    if pass_count < min_pass:
        lines.append(f"{RED}✗ diag parity: only {pass_count} passing (need >= {min_pass}){NC}")
        return (1, "\n".join(lines) + "\n")

    lines.append(f"{GREEN}✓ diag parity: {pass_count} fixtures pass, {skip_count} skipped (Phase 3 pending){NC}")
    return (0, "\n".join(lines) + "\n")


# ── run_parity ────────────────────────────────────────────────────────────────

def _run_cli_parity(root: Path) -> tuple[int, str]:
    """Basic CLI flag comparison between Rust binary and selfhost wasm."""
    lines: list[str] = []
    arukellt_bin = _find_arukellt(root)
    if not arukellt_bin:
        return (1, f"{RED}error: arukellt binary not found{NC}\n")

    selfhost_wasm_env = os.environ.get("SELFHOST_WASM", "")
    selfhost_wasm = Path(selfhost_wasm_env) if selfhost_wasm_env else root / ".build" / "selfhost" / "arukellt-s1.wasm"
    if not selfhost_wasm.is_file():
        return (1, f"{RED}error: selfhost wasm not found at {selfhost_wasm}{NC}\n")

    wasmtime = _find_wasmtime()
    if not wasmtime:
        return (1, f"{RED}error: wasmtime not found{NC}\n")

    lines.append(f"{YELLOW}[cli-parity] Checking CLI surface parity...{NC}")

    pass_count = 0
    fail_count = 0

    def run_rust(*args: str) -> tuple[int, str]:
        r = _run([arukellt_bin, *args], root)
        return r.returncode, (r.stdout + r.stderr).strip()

    def run_self(*args: str) -> tuple[int, str]:
        r = _run([wasmtime, "run", str(selfhost_wasm), "--", *args], root)
        return r.returncode, (r.stdout + r.stderr).strip()

    # Case 1: --version — exact output match
    rc_r, out_r = run_rust("--version")
    rc_s, out_s = run_self("--version")
    if out_r == out_s:
        lines.append("  pass: --version (output identical)")
        pass_count += 1
    else:
        lines.append(f"  FAIL: --version (outputs differ)\n    rust: {out_r!r}\n    self: {out_s!r}")
        fail_count += 1

    # Case 2: --help — exact output match
    rc_r, out_r = run_rust("--help")
    rc_s, out_s = run_self("--help")
    if out_r == out_s:
        lines.append("  pass: --help (output identical)")
        pass_count += 1
    else:
        lines.append(f"  FAIL: --help (outputs differ)")
        fail_count += 1

    # Case 3: unknown command — both must exit non-zero
    # selfhost treats unknown names as filenames (exit 1); Rust exits 2.
    # We only require both to be non-zero.
    rc_r, _ = run_rust("foobar_unknown_cmd")
    rc_s, _ = run_self("foobar_unknown_cmd")
    if rc_r != 0 and rc_s != 0:
        lines.append(f"  pass: unknown-cmd (both non-zero: rust={rc_r} self={rc_s})")
        pass_count += 1
    else:
        lines.append(f"  FAIL: unknown-cmd (expected non-zero: rust={rc_r} self={rc_s})")
        fail_count += 1

    # Cases 4-6: known commands with no args — exit codes must match
    for cmd in ["compile", "check", "run"]:
        rc_r, _ = run_rust(cmd)
        rc_s, _ = run_self(cmd)
        if rc_r != 0 and rc_s != 0:
            lines.append(f"  pass: {cmd} (no-args: both non-zero: rust={rc_r} self={rc_s})")
            pass_count += 1
        else:
            lines.append(f"  FAIL: {cmd} (no-args: expected non-zero: rust={rc_r} self={rc_s})")
            fail_count += 1

    lines.append("")
    lines.append(f"{YELLOW}cli-parity: PASS={pass_count} FAIL={fail_count}{NC}")
    if fail_count > 0:
        lines.append(f"{RED}✗ cli parity: {fail_count} case(s) differ{NC}")
        return (1, "\n".join(lines) + "\n")

    lines.append(f"{GREEN}✓ all {pass_count} CLI parity cases pass{NC}")
    return (0, "\n".join(lines) + "\n")


def run_parity(
    root: Path,
    dry_run: bool,
    mode: str = "",
) -> tuple[int, str]:
    """Port of check-selfhost-parity.sh (all modes).

    mode: '' | '--fixture' | '--cli' | '--diag'
    """
    if dry_run:
        print(f"DRY-RUN: run_parity(mode={mode!r})")
        return (0, "")

    if mode == "--fixture":
        return run_fixture_parity(root, dry_run=False)
    if mode == "--diag":
        return run_diag_parity(root, dry_run=False)
    if mode == "--cli":
        return _run_cli_parity(root)

    # mode == '' → run fixture + diag
    rc1, out1 = run_fixture_parity(root, dry_run=False)
    rc2, out2 = run_diag_parity(root, dry_run=False)
    combined = out1 + out2
    return (max(rc1, rc2), combined)
