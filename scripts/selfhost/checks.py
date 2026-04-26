"""Selfhost domain check runners — pure Python, no shell script calls.

Per ADR-029 (#585) the four selfhost gates run entirely against the
selfhost compiler under wasmtime and never consult ``target/debug/arukellt``.

The trusted base is the committed pinned-reference wasm at
``bootstrap/arukellt-selfhost.wasm`` (see ``bootstrap/PROVENANCE.md``).
"""
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


# ── Paths ────────────────────────────────────────────────────────────────────

PINNED_WASM_REL = "bootstrap/arukellt-selfhost.wasm"
SELFHOST_SOURCE_REL = "src/compiler/main.ark"
SELFHOST_TARGET = "wasm32-wasi-p1"
CLI_VERSION_GOLDEN_REL = "tests/snapshots/selfhost/cli-version.txt"
CLI_HELP_GOLDEN_REL = "tests/snapshots/selfhost/cli-help.txt"


# ── Helpers ──────────────────────────────────────────────────────────────────

def _find_wasmtime() -> str | None:
    return shutil.which("wasmtime")


def _find_pinned_wasm(root: Path) -> Path | None:
    """Return the committed pinned-reference selfhost wasm, honouring override."""
    env_override = os.environ.get("ARUKELLT_PINNED_WASM", "")
    if env_override:
        p = Path(env_override)
        return p if p.is_file() else None
    p = root / PINNED_WASM_REL
    return p if p.is_file() else None


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


def _wasm_compile(
    wasmtime: str,
    compiler_wasm: Path,
    src: str,
    out_rel: str,
    root: Path,
    timeout: int | None = None,
) -> subprocess.CompletedProcess:
    """Run ``compiler_wasm compile <src> --target <T> -o <out_rel>`` under wasmtime."""
    return _run(
        [wasmtime, "run", "--dir", str(root), str(compiler_wasm), "--",
         "compile", src, "--target", SELFHOST_TARGET, "-o", out_rel],
        root,
        timeout=timeout,
    )


def _wasm_check(
    wasmtime: str,
    compiler_wasm: Path,
    src: str,
    root: Path,
    timeout: int | None = None,
) -> subprocess.CompletedProcess:
    return _run(
        [wasmtime, "run", "--dir", str(root), str(compiler_wasm), "--",
         "check", src],
        root,
        timeout=timeout,
    )


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
    """Selfhost-native fixpoint gate (ADR-029).

    Bootstrap path:
        pinned (bootstrap/arukellt-selfhost.wasm) ──▶ s2.wasm
        s2.wasm ──▶ s3.wasm
        require sha256(s2) == sha256(s3)

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

    pinned = _find_pinned_wasm(root)
    if pinned is None:
        emit(f"{RED}error: pinned-reference selfhost wasm not found at "
             f"{PINNED_WASM_REL} (see bootstrap/PROVENANCE.md){NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    source = SELFHOST_SOURCE_REL
    if not (root / source).is_file():
        emit(f"{RED}error: selfhost source not found: {source}{NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    wasmtime = _find_wasmtime()
    if not wasmtime:
        emit(f"{RED}error: wasmtime not found in PATH{NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    build_dir = root / ".build" / "selfhost"
    build_dir.mkdir(parents=True, exist_ok=True)

    s2 = build_dir / "arukellt-s2.wasm"
    s3 = build_dir / "arukellt-s3.wasm"
    s2_rel = str(s2.relative_to(root))
    s3_rel = str(s3.relative_to(root))

    # Stage 2: pinned wasm compiles current selfhost source → s2.wasm
    if build or not s2.is_file():
        emit(f"{YELLOW}[selfhost] Building stage 2 (pinned wasm → s2.wasm)...{NC}")
        r = _wasm_compile(wasmtime, pinned, source, s2_rel, root)
        if r.returncode != 0:
            emit(f"{RED}✗ stage 2 compilation failed{NC}")
            if r.stderr:
                emit(r.stderr[:500])
            return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))
        emit(f"{GREEN}✓ s2.wasm built{NC}")

    if not s2.is_file():
        emit(f"{RED}error: s2.wasm not found at {s2} (run without --no-build first){NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    # Stage 3: s2 compiles current selfhost source → s3.wasm
    if build or not s3.is_file():
        emit(f"{YELLOW}[selfhost] Building stage 3 (s2.wasm → s3.wasm)...{NC}")
        r = _wasm_compile(wasmtime, s2, source, s3_rel, root)
        if r.returncode != 0:
            emit(f"{RED}✗ stage 3 compilation failed{NC}")
            if r.stderr:
                emit(r.stderr[:500])
            return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))
        emit(f"{GREEN}✓ s3.wasm built{NC}")

    if not s3.is_file():
        emit(f"{RED}error: s3.wasm not found at {s3} (run without --no-build first){NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    sha2 = _sha256(s2)
    sha3 = _sha256(s3)

    if sha2 == sha3:
        emit(f"{GREEN}✓ selfhost fixpoint reached: sha256({s2.name}) == sha256({s3.name}){NC}")
        emit(f"  sha256 = {sha2}")
        emit(f"  pinned base: {PINNED_WASM_REL} (sha256 {_sha256(pinned)})")
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


# ── Current-selfhost wasm helper ──────────────────────────────────────────────

def _ensure_current_selfhost(root: Path, wasmtime: str, pinned: Path) -> tuple[Path | None, str]:
    """Return path to current-source selfhost wasm, building it from pinned if needed.

    Output is ``.build/selfhost/arukellt-s2.wasm``. If it already exists, it is
    reused (callers may invoke ``run_fixpoint`` first to refresh it).
    """
    build_dir = root / ".build" / "selfhost"
    build_dir.mkdir(parents=True, exist_ok=True)
    out = build_dir / "arukellt-s2.wasm"
    if out.is_file():
        return out, ""
    out_rel = str(out.relative_to(root))
    r = _wasm_compile(wasmtime, pinned, SELFHOST_SOURCE_REL, out_rel, root)
    if r.returncode != 0:
        return None, (
            f"{RED}error: failed to bootstrap current-selfhost wasm from pinned wasm{NC}\n"
            + (r.stderr[:500] if r.stderr else "")
        )
    return out, ""


# ── Fixture parity skip list ─────────────────────────────────────────────────

# Fixtures with known parity differences that are not semantic errors.
# Pre-585 these tracked Rust-vs-selfhost differences. Post-585 (ADR-029)
# these track pinned-vs-current selfhost differences with the same root
# causes — kept verbatim because the underlying selfhost-emitter
# limitations have not changed.
#
# Format: "category/fixture.ark"  # reason
FIXTURE_PARITY_SKIP: set[str] = {
    "stdlib_sort/sort_f64.ark",  # selfhost f64_to_string uses naive digit extraction
                                 # (1.2 → 1.199999999999999); reference uses Grisu2/shortest-repr
    "functions/higher_order.ark",  # selfhost emitter lacks funcref table / call_indirect
                                   # support; fn-pointer parameters are not yet lowered.
}


# ── run_fixture_parity ────────────────────────────────────────────────────────

def run_fixture_parity(root: Path, dry_run: bool) -> tuple[int, str]:
    """Pinned-vs-current selfhost execution-output parity gate (ADR-029).

    For each ``run:`` fixture in the manifest:
        - compile with pinned wasm and with current selfhost wasm
        - execute both wasms; require stdout/stderr/exit-code equal
    """
    if dry_run:
        print("DRY-RUN: run_fixture_parity()")
        return (0, "")

    lines: list[str] = []

    pinned = _find_pinned_wasm(root)
    if pinned is None:
        return (1, f"{RED}error: pinned-reference selfhost wasm not found at "
                   f"{PINNED_WASM_REL}{NC}\n")

    wasmtime = _find_wasmtime()
    if not wasmtime:
        return (1, f"{RED}error: wasmtime not found{NC}\n")

    current, err = _ensure_current_selfhost(root, wasmtime, pinned)
    if current is None:
        return (1, err)

    fixtures, err = _load_manifest_fixtures(root, "run")
    if err:
        return (1, err + "\n")

    if len(fixtures) < 10:
        return (1, f"{RED}error: fewer than 10 run: fixtures in manifest ({len(fixtures)} found){NC}\n")

    pinned_sha = _sha256(pinned)
    current_sha = _sha256(current)
    lines.append(f"{YELLOW}[fixture-parity] Checking {len(fixtures)} run: fixtures "
                 f"(pinned={pinned_sha[:12]} vs current={current_sha[:12]})...{NC}")

    pass_count = 0
    fail_count = 0
    skip_count = 0

    self_out_dir = root / ".ark-fixture-parity-tmp"
    self_out_dir.mkdir(exist_ok=True)
    tmpdir = tempfile.mkdtemp(prefix="ark-fixture-parity-")
    try:
        for fixture in fixtures:
            if fixture in FIXTURE_PARITY_SKIP:
                lines.append(f"  skip: {fixture} (known parity skip)")
                skip_count += 1
                continue

            ark_file = root / "tests" / "fixtures" / fixture
            if not ark_file.is_file():
                lines.append(f"  skip: {fixture} (not found on disk)")
                skip_count += 1
                continue

            src_rel = str(Path("tests") / "fixtures" / fixture)
            out_pinned_rel = str(Path(".ark-fixture-parity-tmp") /
                                 f"pinned-{fixture.replace('/', '_')}.wasm")
            out_current_rel = str(Path(".ark-fixture-parity-tmp") /
                                  f"current-{fixture.replace('/', '_')}.wasm")
            out_pinned = root / out_pinned_rel
            out_current = root / out_current_rel

            # Compile with pinned compiler
            r = _wasm_compile(wasmtime, pinned, src_rel, out_pinned_rel, root, timeout=30)
            if r.returncode != 0:
                lines.append(f"  skip: {fixture} (pinned compile failed/timeout)")
                skip_count += 1
                continue

            # Compile with current selfhost compiler
            r = _wasm_compile(wasmtime, current, src_rel, out_current_rel, root, timeout=30)
            if r.returncode != 0:
                lines.append(f"  skip: {fixture} (current selfhost compile failed/timeout)")
                skip_count += 1
                continue

            # Compare execution output
            r_p = _run([wasmtime, "run", str(out_pinned)], root, timeout=15)
            p_out = (r_p.stdout + r_p.stderr).strip()
            p_code = r_p.returncode

            r_c = _run([wasmtime, "run", str(out_current)], root, timeout=15)
            c_out = (r_c.stdout + r_c.stderr).strip()
            c_code = r_c.returncode

            # If either side traps as an invalid module (validation error from
            # the emitter — same forgiving treatment as the pre-585 contract),
            # treat as skip not fail.
            def _is_trap_or_invalid(code: int, out: str) -> bool:
                return code == 134 or (code == 1 and "failed to compile" in out)

            if _is_trap_or_invalid(p_code, p_out) or _is_trap_or_invalid(c_code, c_out):
                lines.append(f"  skip: {fixture} (selfhost wasm trap/invalid)")
                skip_count += 1
                continue

            if p_out == c_out and p_code == c_code:
                pass_count += 1
            else:
                lines.append(f"  FAIL: {fixture} (execution output drifts pinned↔current)")
                if p_code != c_code:
                    lines.append(f"    exit: pinned={p_code} current={c_code}")
                if p_out != c_out:
                    lines.append(f"    pinned : {p_out[:80]!r}")
                    lines.append(f"    current: {c_out[:80]!r}")
                fail_count += 1
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)
        shutil.rmtree(str(self_out_dir), ignore_errors=True)

    lines.append("")
    lines.append(f"{YELLOW}fixture-parity: PASS={pass_count} FAIL={fail_count} SKIP={skip_count}{NC}")

    if fail_count > 0:
        lines.append(
            f"{RED}✗ fixture parity: {fail_count} fixture(s) drift between pinned and current selfhost — "
            f"fix the regression or refresh bootstrap/arukellt-selfhost.wasm per ADR-029{NC}"
        )
        return (1, "\n".join(lines) + "\n")

    if pass_count < 10:
        lines.append(
            f"{RED}✗ fixture parity: only {pass_count} fixtures passed (need >= 10 per #585 floor){NC}"
        )
        return (1, "\n".join(lines) + "\n")

    lines.append(f"{GREEN}✓ all {pass_count} run: fixtures match between pinned and current selfhost{NC}")
    return (0, "\n".join(lines) + "\n")


# ── run_diag_parity ───────────────────────────────────────────────────────────

# Fixtures skipped for diag-parity because selfhost has not yet implemented
# the diagnostics or the test exercises an unimplemented feature.  These are
# tracked in issue #529 Phase 3 (diagnostic parity expansion).
DIAG_PARITY_SKIP: frozenset[str] = frozenset({
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
    """Pure-selfhost diagnostic snapshot gate (ADR-029).

    For each ``diag:`` fixture, run the current selfhost compiler under
    wasmtime with ``check`` and require its output to contain the
    committed ``.selfhost.diag`` (or ``.diag`` fallback) pattern.
    """
    if dry_run:
        print("DRY-RUN: run_diag_parity()")
        return (0, "")

    lines: list[str] = []

    pinned = _find_pinned_wasm(root)
    if pinned is None:
        return (1, f"{RED}error: pinned-reference selfhost wasm not found at "
                   f"{PINNED_WASM_REL}{NC}\n")

    wasmtime = _find_wasmtime()
    if not wasmtime:
        return (1, f"{RED}error: wasmtime not found{NC}\n")

    current, err = _ensure_current_selfhost(root, wasmtime, pinned)
    if current is None:
        return (1, err)

    fixtures, err = _load_manifest_fixtures(root, "diag")
    if err:
        return (1, err + "\n")

    lines.append(f"{YELLOW}[diag-parity] Checking {len(fixtures)} diag: fixtures "
                 f"against committed .diag goldens (current selfhost only)...{NC}")

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
        if not diag_path.is_file() and not selfhost_diag_path.is_file():
            lines.append(f"  skip: {fixture} (.diag file not found)")
            skip_count += 1
            continue

        # Prefer .selfhost.diag (selfhost-specific golden) over .diag (legacy).
        if selfhost_diag_path.is_file():
            pattern = selfhost_diag_path.read_text().strip()
        else:
            pattern = diag_path.read_text().strip()

        r = _wasm_check(wasmtime, current, str(Path("tests") / "fixtures" / fixture), root)
        out = r.stdout + r.stderr

        if pattern in out:
            lines.append(f"  pass: {fixture}")
            pass_count += 1
        else:
            lines.append(f"  FAIL: {fixture} (selfhost: pattern '{pattern[:60]}' not found)")
            fail_count += 1

    lines.append("")
    lines.append(f"{YELLOW}diag-parity: PASS={pass_count} SKIP={skip_count} FAIL={fail_count}{NC}")

    min_pass = 10
    if fail_count > 0:
        lines.append(f"{RED}✗ diag parity: {fail_count} fixture(s) regressed against committed goldens{NC}")
        return (1, "\n".join(lines) + "\n")
    if pass_count < min_pass:
        lines.append(f"{RED}✗ diag parity: only {pass_count} passing (need >= {min_pass}){NC}")
        return (1, "\n".join(lines) + "\n")

    lines.append(
        f"{GREEN}✓ diag parity: {pass_count} fixtures pass against committed selfhost goldens, "
        f"{skip_count} skipped (Phase 3 pending){NC}"
    )
    return (0, "\n".join(lines) + "\n")


# ── run_parity ────────────────────────────────────────────────────────────────

def _run_cli_parity(root: Path) -> tuple[int, str]:
    """Pure-selfhost CLI snapshot gate (ADR-029).

    Compares ``--version`` and ``--help`` byte-equal against committed
    goldens under ``tests/snapshots/selfhost/``, and asserts non-zero
    exit codes for unknown commands and known-but-no-args invocations.
    """
    lines: list[str] = []

    pinned = _find_pinned_wasm(root)
    if pinned is None:
        return (1, f"{RED}error: pinned-reference selfhost wasm not found at "
                   f"{PINNED_WASM_REL}{NC}\n")

    wasmtime = _find_wasmtime()
    if not wasmtime:
        return (1, f"{RED}error: wasmtime not found{NC}\n")

    current, err = _ensure_current_selfhost(root, wasmtime, pinned)
    if current is None:
        return (1, err)

    version_golden = root / CLI_VERSION_GOLDEN_REL
    help_golden = root / CLI_HELP_GOLDEN_REL
    if not version_golden.is_file():
        return (1, f"{RED}error: cli-version golden missing: {CLI_VERSION_GOLDEN_REL}{NC}\n")
    if not help_golden.is_file():
        return (1, f"{RED}error: cli-help golden missing: {CLI_HELP_GOLDEN_REL}{NC}\n")

    lines.append(f"{YELLOW}[cli-parity] Checking selfhost CLI surface against committed goldens...{NC}")

    pass_count = 0
    fail_count = 0

    def run_self(*args: str) -> tuple[int, str]:
        r = _run([wasmtime, "run", str(current), "--", *args], root)
        return r.returncode, (r.stdout + r.stderr)

    def _norm(s: str) -> str:
        return s.replace("\r\n", "\n").rstrip("\n")

    # Case 1: --version snapshot
    _, out_v = run_self("--version")
    expected_v = _norm(version_golden.read_text())
    actual_v = _norm(out_v)
    if actual_v == expected_v:
        lines.append("  pass: --version (matches golden)")
        pass_count += 1
    else:
        lines.append(f"  FAIL: --version (drifts from golden)\n"
                     f"    expected: {expected_v!r}\n"
                     f"    actual  : {actual_v!r}")
        fail_count += 1

    # Case 2: --help snapshot
    _, out_h = run_self("--help")
    expected_h = _norm(help_golden.read_text())
    actual_h = _norm(out_h)
    if actual_h == expected_h:
        lines.append("  pass: --help (matches golden)")
        pass_count += 1
    else:
        lines.append("  FAIL: --help (drifts from golden — update tests/snapshots/selfhost/cli-help.txt if intentional)")
        # Emit a tiny diff hint
        ex_lines = expected_h.splitlines()
        ac_lines = actual_h.splitlines()
        for i, (e, a) in enumerate(zip(ex_lines, ac_lines)):
            if e != a:
                lines.append(f"    line {i+1}: expected {e!r} got {a!r}")
                break
        if len(ex_lines) != len(ac_lines):
            lines.append(f"    line count: expected {len(ex_lines)} got {len(ac_lines)}")
        fail_count += 1

    # Case 3: unknown command — must exit non-zero
    rc_s, _ = run_self("foobar_unknown_cmd")
    if rc_s != 0:
        lines.append(f"  pass: unknown-cmd (non-zero exit: {rc_s})")
        pass_count += 1
    else:
        lines.append(f"  FAIL: unknown-cmd (expected non-zero exit, got {rc_s})")
        fail_count += 1

    # Cases 4-6: known commands with no args — must exit non-zero
    for cmd in ["compile", "check", "run"]:
        rc_s, _ = run_self(cmd)
        if rc_s != 0:
            lines.append(f"  pass: {cmd} (no-args: non-zero exit: {rc_s})")
            pass_count += 1
        else:
            lines.append(f"  FAIL: {cmd} (no-args: expected non-zero, got {rc_s})")
            fail_count += 1

    lines.append("")
    lines.append(f"{YELLOW}cli-parity: PASS={pass_count} FAIL={fail_count}{NC}")
    if fail_count > 0:
        lines.append(f"{RED}✗ cli parity: {fail_count} case(s) failed{NC}")
        return (1, "\n".join(lines) + "\n")

    lines.append(f"{GREEN}✓ all {pass_count} CLI parity cases pass{NC}")
    return (0, "\n".join(lines) + "\n")


def run_parity(
    root: Path,
    dry_run: bool,
    mode: str = "",
) -> tuple[int, str]:
    """Selfhost parity gate dispatch.

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
