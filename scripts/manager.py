#!/usr/bin/env python3
"""Arukellt scripts manager.

Usage:
    manager.py <domain> <subcommand> [options]

Domains:
    verify

Subcommands for verify:
    quick       Run the fast local gate checks (default behavior of verify-harness.sh).
    fixtures    Run the fixture harness via cargo test.
    size        Run the hello.wasm binary size gate.
    wat         Run the WAT roundtrip gate.
    component   Run the component interop smoke test.

Global flags:
    --dry-run   Print intent but do not execute commands.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import os
import subprocess
import sys
from pathlib import Path

# Ensure scripts/ is on sys.path so we can import lib and verify packages.
_SCRIPTS_DIR = Path(__file__).resolve().parent
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from lib.files import repo_root as _repo_root  # noqa: E402
from verify.fixtures import (  # noqa: E402
    count_fixtures,
    disk_fixture_paths,
    load_manifest,
)
from verify.harness import GREEN, NC, RED, YELLOW, Harness  # noqa: E402
from selfhost.checks import (  # noqa: E402
    SelfhostFixpointResult,
    run_diag_parity,
    run_fixpoint,
    run_fixture_parity,
    run_parity,
)
from docs_domain.checks import (  # noqa: E402
    run_consistency,
    run_examples,
    run_freshness,
    run_regenerate,
)
from perf.checks import (  # noqa: E402
    run_baseline,
    run_benchmarks,
    run_gate as run_perf_gate,
)
from gate_domain.checks import (  # noqa: E402
    run_local,
    run_pre_commit,
    run_pre_push,
    run_repro,
)

# ── helpers ──────────────────────────────────────────────────────────────────


def _run(cmd: list[str], *, cwd: Path, dry_run: bool) -> tuple[int, str, str]:
    if dry_run:
        print(f"DRY-RUN: {cmd}")
        return (0, "", "")
    result = subprocess.run(cmd, cwd=str(cwd), capture_output=True, text=True)
    return (result.returncode, result.stdout, result.stderr)


# ── verify subcommands ────────────────────────────────────────────────────────


def cmd_verify_quick(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    # ── Fixture manifest completeness check ──────────────────────────────────
    print(f"\n{YELLOW}[manifest] Checking fixture manifest completeness...{NC}")
    manifest_file = root / "tests" / "fixtures" / "manifest.txt"
    manifest_ok = True
    fixture_count = 0
    if not manifest_file.exists():
        h.check_fail("Fixture manifest not found: tests/fixtures/manifest.txt")
        manifest_ok = False
    else:
        fixture_count = count_fixtures(manifest_file)
        fixtures_root = root / "tests" / "fixtures"
        disk_paths = disk_fixture_paths(fixtures_root)
        manifest_entries = sorted(
            {
                e["path"]
                for e in load_manifest(manifest_file)
                if e["kind"] != "bench"
            }
        )
        if disk_paths != manifest_entries:
            h.check_fail("Fixture manifest out of sync with disk")
            disk_set = set(disk_paths)
            manifest_set = set(manifest_entries)
            for p in sorted(disk_set - manifest_set)[:10]:
                print(f"  < {p}")
            for p in sorted(manifest_set - disk_set)[:10]:
                print(f"  > {p}")
            manifest_ok = False

    if manifest_ok:
        h.check_pass(f"Fixture manifest completeness ({fixture_count} entries)")

    # ── Background checks ─────────────────────────────────────────────────────
    print(f"\n{YELLOW}[bg] Running background checks in parallel...{NC}")

    def _shell(cmd_str: str) -> tuple[int, str]:
        """Run a bash command string; return (rc, combined output)."""
        if dry_run:
            return (0, f"DRY-RUN: {cmd_str}")
        result = subprocess.run(
            ["bash", "-lc", cmd_str],
            cwd=str(root),
            capture_output=True,
            text=True,
        )
        return (result.returncode, result.stdout + result.stderr)

    bg_checks: list[tuple[str, str]] = [
        (
            "Documentation structure OK",
            "test -f AGENTS.md && test -f docs/process/agent-harness.md "
            "&& test -d docs/adr && test -d issues/open && test -d issues/done "
            "&& test -d docs/language && test -d docs/platform && test -d docs/stdlib "
            "&& test -d docs/process",
        ),
        (
            "All required ADRs decided",
            "for f in docs/adr/ADR-002-memory-model.md docs/adr/ADR-003-generics-strategy.md "
            "docs/adr/ADR-004-trait-strategy.md docs/adr/ADR-005-llvm-scope.md "
            "docs/adr/ADR-006-abi-policy.md; do "
            'test -f "$f" || exit 1; grep -q \'DECIDED\\|決定\' "$f" || exit 1; done',
        ),
        (
            "Language specification OK",
            "test -f docs/language/memory-model.md && test -f docs/language/type-system.md "
            "&& test -f docs/language/syntax.md",
        ),
        (
            "Platform specification OK",
            "test -f docs/platform/wasm-features.md && test -f docs/platform/abi.md "
            "&& test -f docs/platform/wasi-resource-model.md",
        ),
        (
            "Stdlib specification OK",
            "test -f docs/stdlib/README.md && test -f docs/stdlib/core.md "
            "&& test -f docs/stdlib/io.md",
        ),
        (
            "docs consistency",
            "python3 scripts/check/check-docs-consistency.py",
        ),
        (
            "docs freshness (project-state.toml vs manifest.txt)",
            "python3 scripts/check/check-docs-freshness.py",
        ),
        (
            "stdlib manifest check",
            "bash scripts/check/check-stdlib-manifest.sh",
        ),
        (
            "issues/done/ has no unchecked checkboxes",
            "files=$(grep -rl '\\- \\[ \\]' issues/done/ 2>/dev/null | grep '\\.md$' || true); "
            'if [ -n "$files" ]; then echo "Files in done/ with unchecked items:"; '
            'printf \'%s\\n\' "$files"; exit 1; fi',
        ),
        (
            "no panic/unwrap in user-facing crates",
            "bash scripts/check/check-panic-audit.sh",
        ),
        (
            "asset naming convention (snake_case)",
            "bash scripts/check/check-asset-naming.sh",
        ),
        (
            "generated file boundary check",
            "bash scripts/check/check-generated-files.sh",
        ),
        (
            "doc example check (ark blocks in docs/)",
            "python3 scripts/check/check-doc-examples.py docs/",
        ),
    ]

    bg_results: list[tuple[str, int, str]] = []
    with concurrent.futures.ThreadPoolExecutor() as executor:
        futures = {executor.submit(_shell, cmd_str): label for label, cmd_str in bg_checks}
        for future in concurrent.futures.as_completed(futures):
            label = futures[future]
            rc, out = future.result()
            bg_results.append((label, rc, out))

    print(f"\n{YELLOW}[bg] Collecting background check results...{NC}")
    for label, rc, out in bg_results:
        if rc == 0:
            h.check_pass(label)
        else:
            h.check_fail(label)
            for line in out.splitlines()[-30:]:
                print(line)

    # Static pass
    h.check_pass("Perf policy documented (check<=10%, compile<=20%; heavy perf separated)")

    # ── Stdlib fixture registration checks ───────────────────────────────────
    fixtures_root = root / "tests" / "fixtures"
    manifest_text = manifest_file.read_text(encoding="utf-8") if manifest_file.exists() else ""

    stdlib_missing = 0
    for stdlib_dir in sorted(fixtures_root.glob("stdlib_*")):
        if not stdlib_dir.is_dir():
            continue
        for ark in sorted(stdlib_dir.glob("*.ark")):
            rel_path = str(ark.relative_to(fixtures_root))
            if rel_path not in manifest_text:
                print(f"  Missing from manifest.txt: {rel_path}")
                stdlib_missing += 1
    if stdlib_missing == 0:
        h.check_pass("all stdlib fixtures registered in manifest.txt")
    else:
        h.check_fail(f"stdlib fixtures missing from manifest.txt ({stdlib_missing})")

    stdlib_fixture_count = manifest_text.count("stdlib_")
    if stdlib_fixture_count >= 5:
        h.check_pass(f"v3 stdlib fixtures registered ({stdlib_fixture_count} entries in manifest)")
    else:
        h.check_fail(f"v3 stdlib fixtures insufficient ({stdlib_fixture_count} < 5)")

    # ── Internal link integrity ───────────────────────────────────────────────
    links_script = root / "scripts" / "check" / "check-links.sh"
    if links_script.exists():
        rc, _, _ = _run(["bash", str(links_script)], cwd=root, dry_run=dry_run)
        if rc == 0:
            h.check_pass("internal link integrity")
        else:
            h.check_fail("broken internal links detected (run scripts/check/check-links.sh)")

    # ── Diagnostic codes check ────────────────────────────────────────────────
    diag_script = root / "scripts" / "check" / "check-diagnostic-codes.sh"
    if diag_script.exists():
        rc, _, _ = _run(["bash", str(diag_script)], cwd=root, dry_run=dry_run)
        if rc == 0:
            h.check_pass("diagnostic codes aligned")
        else:
            h.check_fail("diagnostic codes out of sync (run scripts/check/check-diagnostic-codes.sh)")

    # ── Summary ───────────────────────────────────────────────────────────────
    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}========================================{NC}")
    print(f"{YELLOW}Summary{NC}")
    print(f"{YELLOW}========================================{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")

    if failed == 0:
        print(f"\n{GREEN}\u2713 All selected harness checks passed{NC}")
    else:
        print(f"\n{RED}\u2717 Some harness checks failed ({failed} checks failed){NC}")

    return h.exit_code()


def cmd_verify_fixtures(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[fixtures] Running fixture harness...{NC}")

    arukellt_bin = os.environ.get("ARUKELLT_BIN", "")
    env = os.environ.copy()
    if arukellt_bin:
        env["ARUKELLT_BIN"] = arukellt_bin

    cmd = ["cargo", "test", "-p", "arukellt", "--test", "harness", "--", "--nocapture"]

    if dry_run:
        print(f"DRY-RUN: {cmd}")
        h.check_pass("fixture harness (dry-run)")
        total, passed, skipped, failed = h.summary()
        print(f"\n{YELLOW}Summary{NC}")
        print(f"Total checks: {total}")
        print(f"Passed: {GREEN}{passed}{NC}")
        print(f"Skipped: {YELLOW}{skipped}{NC}")
        print(f"Failed: {RED}{failed}{NC}")
        return h.exit_code()

    result = subprocess.run(cmd, cwd=str(root), capture_output=True, text=True, env=env)
    output = result.stdout + result.stderr

    if "FAIL: 0" in output:
        summary_line = next(
            (line for line in output.splitlines() if "PASS:" in line), ""
        )
        h.check_pass(f"fixture harness ({summary_line.strip()})")
    else:
        h.check_fail("fixture harness")
        for line in output.splitlines():
            if line.startswith(("PASS:", "FAIL ")):
                print(line)

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_verify_size(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[size] Checking hello.wasm binary size gate...{NC}")

    arukellt_bin = os.environ.get("ARUKELLT_BIN", "")
    if not arukellt_bin:
        debug = root / "target" / "debug" / "arukellt"
        release = root / "target" / "release" / "arukellt"
        if debug.exists():
            arukellt_bin = str(debug)
        elif release.exists():
            arukellt_bin = str(release)

    HELLO_WASM_OUT = "hello_perfgate.wasm"
    HELLO_SIZE_MAX = 5120

    if dry_run:
        print(f"DRY-RUN: would compile tests/fixtures/hello/hello.ark via {arukellt_bin!r}")
        h.check_pass("hello.wasm binary size (dry-run)")
        total, passed, skipped, failed = h.summary()
        print(f"\n{YELLOW}Summary{NC}")
        print(f"Total checks: {total}")
        print(f"Passed: {GREEN}{passed}{NC}")
        print(f"Skipped: {YELLOW}{skipped}{NC}")
        print(f"Failed: {RED}{failed}{NC}")
        return h.exit_code()

    if not arukellt_bin:
        h.check_fail("hello.wasm size gate (arukellt binary not found — build first)")
    else:
        compile_cmd = [
            arukellt_bin,
            "compile",
            "tests/fixtures/hello/hello.ark",
            "--target", "wasm32-wasi-p2",
            "--opt-level", "1",
            "-o", HELLO_WASM_OUT,
        ]
        result = subprocess.run(compile_cmd, cwd=str(root), capture_output=True)
        wasm_path = root / HELLO_WASM_OUT
        try:
            if result.returncode == 0 and wasm_path.exists():
                size = wasm_path.stat().st_size
                wasm_path.unlink(missing_ok=True)
                if size <= HELLO_SIZE_MAX:
                    h.check_pass(f"hello.wasm binary size: {size} bytes (<= {HELLO_SIZE_MAX})")
                else:
                    h.check_fail(f"hello.wasm binary size: {size} bytes (> {HELLO_SIZE_MAX} threshold)")
            else:
                wasm_path.unlink(missing_ok=True)
                h.check_fail("hello.wasm compilation failed")
        except Exception:
            wasm_path.unlink(missing_ok=True)
            h.check_fail("hello.wasm compilation failed")

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_verify_wat(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[wat] Running WAT roundtrip verification...{NC}")

    rc, _, _ = _run(
        ["bash", "scripts/run/wat-roundtrip.sh"],
        cwd=root,
        dry_run=dry_run,
    )
    if rc == 0:
        h.check_pass("WAT roundtrip (wasm2wat \u21c4 wat2wasm)")
    else:
        h.check_fail("WAT roundtrip (wasm2wat \u21c4 wat2wasm)")

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_verify_component(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[component] Component interop smoke test...{NC}")

    # Check for wasmtime
    wasmtime_check = subprocess.run(
        ["which", "wasmtime"], capture_output=True
    )
    if wasmtime_check.returncode != 0:
        h.check_skip("component interop (wasmtime not found)")
    else:
        interop_dir = root / "tests" / "component-interop" / "jco"
        run_scripts = sorted(interop_dir.glob("*/run.sh"))
        if not run_scripts:
            h.check_skip("component interop scripts not found")
        else:
            for run_sh in run_scripts:
                fixture_name = run_sh.parent.name
                rc, _, _ = _run(["bash", str(run_sh)], cwd=root, dry_run=dry_run)
                if rc == 0:
                    h.check_pass(f"component interop: {fixture_name} (wasmtime)")
                else:
                    h.check_fail(f"component interop: {fixture_name} (wasmtime)")

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


# ── CLI wiring ────────────────────────────────────────────────────────────────

# Flags not yet migrated to manager.py (Phase 1 out-of-scope).
_VERIFY_OUT_OF_SCOPE_FLAGS = {
    "--cargo", "--baseline", "--fixpoint", "--selfhost-fixture-parity",
    "--selfhost-diag-parity", "--lsp-perf", "--memory-gate", "--repro",
    "--opt-equiv", "--perf-gate",
}


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="manager.py",
        description="Arukellt scripts manager",
    )
    parser.add_argument("--dry-run", action="store_true", help="Print intent but do not execute.")

    sub_domain = parser.add_subparsers(dest="domain", metavar="<domain>")
    sub_domain.required = True

    # verify domain
    verify_parser = sub_domain.add_parser("verify", help="Verification commands")


    verify_parser.add_argument("--dry-run", action="store_true", help="Print intent but do not execute.")
    verify_parser.add_argument("--quick",     action="store_true", help="Run the fast local gate checks (default)")
    verify_parser.add_argument("--fixtures",  action="store_true", help="Run the fixture harness via cargo test")
    verify_parser.add_argument("--size",      action="store_true", help="Run the hello.wasm binary size gate")
    verify_parser.add_argument("--wat",       action="store_true", help="Run the WAT roundtrip gate")
    verify_parser.add_argument("--component", action="store_true", help="Run the component interop smoke test")
    verify_parser.add_argument("--docs",      action="store_true", help="[Phase 1 stub] skipped — not yet migrated")
    verify_parser.add_argument(
        "--full", action="store_true",
        help="Run quick + fixtures + size + wat + component sequentially",
    )

    # ── Positional subcommand interface (legacy, preserved) ───────────────────
    sub_verify = verify_parser.add_subparsers(dest="subcommand", metavar="<subcommand>")
    sub_verify.required = False

    for name, help_text in [
        ("quick",     "Run the fast local gate checks"),
        ("fixtures",  "Run the fixture harness via cargo test"),
        ("size",      "Run the hello.wasm binary size gate"),
        ("wat",       "Run the WAT roundtrip gate"),
        ("component", "Run the component interop smoke test"),
    ]:
        p = sub_verify.add_parser(name, help=help_text)
        p.add_argument("--dry-run", action="store_true", help="Print intent but do not execute.")

    _build_selfhost_subparser(sub_domain)
    _build_docs_subparser(sub_domain)
    _build_perf_subparser(sub_domain)
    _build_gate_subparser(sub_domain)

    return parser


def _build_selfhost_subparser(sub_domain: argparse._SubParsersAction) -> None:  # type: ignore[type-arg]
    sh = sub_domain.add_parser("selfhost", help="Selfhost check commands")
    sh.add_argument("--dry-run", action="store_true")
    sub = sh.add_subparsers(dest="subcommand", metavar="<subcommand>")
    sub.required = True
    p = sub.add_parser("fixpoint", help="Run selfhost fixpoint check")
    p.add_argument("--dry-run", action="store_true")
    p.add_argument("--build", action="store_true", default=False, help="Build before check")
    for name, help_text in [
        ("fixture-parity", "Run selfhost fixture parity"),
        ("diag-parity", "Run selfhost diagnostic parity"),
    ]:
        q = sub.add_parser(name, help=help_text)
        q.add_argument("--dry-run", action="store_true")
    p_par = sub.add_parser("parity", help="Run selfhost parity (fixture/cli/diag)")
    p_par.add_argument("--dry-run", action="store_true")
    p_par.add_argument("--mode", choices=["", "--fixture", "--cli", "--diag"], default="")


def _build_docs_subparser(sub_domain: argparse._SubParsersAction) -> None:  # type: ignore[type-arg]
    dp = sub_domain.add_parser("docs", help="Documentation commands")
    dp.add_argument("--dry-run", action="store_true")
    sub = dp.add_subparsers(dest="subcommand", metavar="<subcommand>")
    sub.required = True
    for name, help_text in [("check", "Run docs checks"), ("regenerate", "Regenerate docs")]:
        p = sub.add_parser(name, help=help_text)
        p.add_argument("--dry-run", action="store_true")
    # regenerate extra flag
    sub.choices["regenerate"].add_argument(
        "--check-only", dest="check_only", action="store_true",
        help="Check only, do not write",
    )


def _build_perf_subparser(sub_domain: argparse._SubParsersAction) -> None:  # type: ignore[type-arg]
    pp = sub_domain.add_parser("perf", help="Performance commands")
    pp.add_argument("--dry-run", action="store_true")
    sub = pp.add_subparsers(dest="subcommand", metavar="<subcommand>")
    sub.required = True
    for name, help_text in [
        ("gate", "Run perf gate"),
        ("baseline", "Collect perf baseline"),
        ("benchmarks", "Run benchmarks"),
    ]:
        p = sub.add_parser(name, help=help_text)
        p.add_argument("--dry-run", action="store_true")
    sub.choices["gate"].add_argument("--update", action="store_true", help="Update baseline")
    sub.choices["benchmarks"].add_argument(
        "--no-quick", dest="no_quick", action="store_true", help="Full (not quick) benchmarks"
    )


def _build_gate_subparser(sub_domain: argparse._SubParsersAction) -> None:  # type: ignore[type-arg]
    gp = sub_domain.add_parser("gate", help="Gate checks")
    gp.add_argument("--dry-run", action="store_true")
    sub = gp.add_subparsers(dest="subcommand", metavar="<subcommand>")
    sub.required = True
    for name, help_text in [
        ("local", "Run full local CI gate"),
        ("pre-commit", "Run pre-commit verification"),
        ("pre-push", "Run pre-push verification"),
        ("repro", "Run reproducible build check"),
    ]:
        p = sub.add_parser(name, help=help_text)
        p.add_argument("--dry-run", action="store_true")
    sub.choices["local"].add_argument("--skip-ext", dest="skip_ext", action="store_true")
    for flag, dest, help_text in [
        ("--fixture", "fixture", "Fixture path"),
        ("--target", "target", "Target triple"),
    ]:
        sub.choices["repro"].add_argument(flag, dest=dest, default="", help=help_text)
    sub.choices["repro"].add_argument("--verbose", action="store_true")


def main() -> int:
    argv = list(sys.argv[1:])

    # Normalize selfhost parity mode values so documented invocations like
    # `selfhost parity --mode --cli` work under argparse.
    if len(argv) >= 4 and argv[0] == "selfhost" and argv[1] == "parity":
        for i in range(len(argv) - 1):
            if argv[i] == "--mode" and argv[i + 1] in {"--fixture", "--cli", "--diag"}:
                argv[i] = f"--mode={argv[i + 1]}"
                del argv[i + 1]
                break

    # Pre-scan argv for out-of-scope flags and give a clear error before argparse
    # touches anything, so we don't get confusing "unrecognized arguments" messages.
    if len(argv) > 0 and argv[0] == "verify":
        for raw in argv[1:]:
            flag = raw.split("=")[0]  # strip =value if any
            if flag in _VERIFY_OUT_OF_SCOPE_FLAGS:
                print(
                    f"error: flag not yet migrated to manager.py (Phase 1 scope: "
                    f"quick/fixtures/size/wat/component): {flag}",
                    file=sys.stderr,
                )
                return 2

    parser = build_parser()
    args = parser.parse_args(argv)

    # Propagate global --dry-run down (argparse already puts both in same namespace
    # for nested parsers, but guard in case of future restructuring).
    dry_run: bool = getattr(args, "dry_run", False)

    dispatch_positional = {
        "quick":     cmd_verify_quick,
        "fixtures":  cmd_verify_fixtures,
        "size":      cmd_verify_size,
        "wat":       cmd_verify_wat,
        "component": cmd_verify_component,
    }

    if args.domain == "verify":
        subcommand: str | None = getattr(args, "subcommand", None)

        # ── Positional subcommand takes priority when present ─────────────────
        if subcommand:
            handler = dispatch_positional.get(subcommand)
            if handler is None:
                print(f"{RED}error: unknown subcommand: {subcommand}{NC}", file=sys.stderr)
                return 1
            return handler(args)

        # ── Flag-based dispatch ───────────────────────────────────────────────
        # Expand --full into the individual Phase-1 flags.
        if args.full:
            args.quick = args.fixtures = args.size = args.wat = args.component = True

        # Collect requested steps in a deterministic order.
        steps: list[tuple[str, object]] = []
        for flag, fn in [
            ("quick",     cmd_verify_quick),
            ("fixtures",  cmd_verify_fixtures),
            ("size",      cmd_verify_size),
            ("wat",       cmd_verify_wat),
            ("component", cmd_verify_component),
        ]:
            if getattr(args, flag, False):
                steps.append((flag, fn))

        # --docs: Phase 1 stub — skip with a message.
        docs_requested = getattr(args, "docs", False)

        # Default: no flags given → run quick.
        if not steps and not docs_requested:
            return cmd_verify_quick(args)

        overall_rc = 0
        for flag, fn in steps:
            rc = fn(args)  # type: ignore[operator]
            if rc != 0:
                overall_rc = rc

        if docs_requested:
            print("[verify docs] skipped — not yet migrated (see Issue #534)")
            # exit 0 for docs stub; don't override a real failure.

        return overall_rc

    if args.domain == "selfhost":
        _sh_dispatch = {
            "fixpoint":       cmd_selfhost_fixpoint,
            "fixture-parity": cmd_selfhost_fixture_parity,
            "diag-parity":    cmd_selfhost_diag_parity,
            "parity":         cmd_selfhost_parity,
        }
        subcommand = getattr(args, "subcommand", None)
        handler = _sh_dispatch.get(subcommand or "")
        if handler is None:
            print(f"{RED}error: unknown selfhost subcommand: {subcommand}{NC}", file=sys.stderr)
            return 1
        return handler(args)

    if args.domain == "docs":
        _docs_dispatch = {
            "check":      cmd_docs_check,
            "regenerate": cmd_docs_regenerate,
        }
        subcommand = getattr(args, "subcommand", None)
        handler = _docs_dispatch.get(subcommand or "")
        if handler is None:
            print(f"{RED}error: unknown docs subcommand: {subcommand}{NC}", file=sys.stderr)
            return 1
        return handler(args)

    if args.domain == "perf":
        _perf_dispatch = {
            "gate":       cmd_perf_gate,
            "baseline":   cmd_perf_baseline,
            "benchmarks": cmd_perf_benchmarks,
        }
        subcommand = getattr(args, "subcommand", None)
        handler = _perf_dispatch.get(subcommand or "")
        if handler is None:
            print(f"{RED}error: unknown perf subcommand: {subcommand}{NC}", file=sys.stderr)
            return 1
        return handler(args)

    if args.domain == "gate":
        _gate_dispatch = {
            "local":      cmd_gate_local,
            "pre-commit": cmd_gate_pre_commit,
            "pre-push":   cmd_gate_pre_push,
            "repro":      cmd_gate_repro,
        }
        subcommand = getattr(args, "subcommand", None)
        handler = _gate_dispatch.get(subcommand or "")
        if handler is None:
            print(f"{RED}error: unknown gate subcommand: {subcommand}{NC}", file=sys.stderr)
            return 1
        return handler(args)

    print(f"{RED}error: unknown domain: {args.domain}{NC}", file=sys.stderr)
    return 1
# ── selfhost subcommands ──────────────────────────────────────────────────────


def cmd_selfhost_fixpoint(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    no_build: bool = not getattr(args, "build", False)
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[selfhost] Running selfhost fixpoint check...{NC}")
    res: SelfhostFixpointResult = run_fixpoint(root, dry_run, no_build=no_build)

    if res.passed:
        h.check_pass("selfhost fixpoint reached")
    elif res.skipped:
        h.check_skip(f"selfhost fixpoint not yet reached (exit {res.exit_code})")
    else:
        h.check_fail("selfhost fixpoint check failed")
        for line in res.output.splitlines()[-30:]:
            print(line)

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_selfhost_fixture_parity(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[selfhost] Running selfhost fixture parity check...{NC}")
    rc, out = run_fixture_parity(root, dry_run)
    if rc == 0:
        h.check_pass("selfhost fixture parity")
    else:
        h.check_fail("selfhost fixture parity")
        for line in out.splitlines()[-30:]:
            print(line)

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_selfhost_diag_parity(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[selfhost] Running selfhost diagnostic parity check...{NC}")
    rc, out = run_diag_parity(root, dry_run)
    if rc == 0:
        h.check_pass("selfhost diagnostic parity")
    else:
        h.check_fail("selfhost diagnostic parity")
        for line in out.splitlines()[-30:]:
            print(line)

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_selfhost_parity(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    mode: str = getattr(args, "mode", "") or ""
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[selfhost] Running selfhost parity check (mode={mode!r})...{NC}")
    rc, out = run_parity(root, dry_run, mode=mode)
    if rc == 0:
        h.check_pass(f"selfhost parity{' ' + mode if mode else ''}")
    else:
        h.check_fail(f"selfhost parity{' ' + mode if mode else ''}")
        for line in out.splitlines()[-30:]:
            print(line)

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


# ── docs subcommands ──────────────────────────────────────────────────────────


def cmd_docs_check(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[docs check] Running docs checks in parallel...{NC}")
    import concurrent.futures as _cf

    checks = [
        ("docs consistency", run_consistency),
        ("docs freshness", run_freshness),
        ("doc examples", run_examples),
    ]
    with _cf.ThreadPoolExecutor() as executor:
        futures = {executor.submit(fn, root, dry_run): label for label, fn in checks}
        for future in _cf.as_completed(futures):
            label = futures[future]
            rc, out = future.result()
            if rc == 0:
                h.check_pass(label)
            else:
                h.check_fail(label)
                for line in out.splitlines()[-20:]:
                    print(line)

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_docs_regenerate(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    check_only: bool = getattr(args, "check_only", False)
    h = Harness(repo_root=root, dry_run=dry_run)

    label = "docs regenerate (check)" if check_only else "docs regenerate"
    print(f"\n{YELLOW}[docs regenerate] {label}...{NC}")
    rc, out = run_regenerate(root, dry_run=dry_run, check_only=check_only)
    if out:
        print(out, end="")
    if rc == 0:
        h.check_pass(label)
    else:
        h.check_fail(label)

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


# ── perf subcommands ──────────────────────────────────────────────────────────


def cmd_perf_gate(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    update: bool = getattr(args, "update", False)
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[perf gate] Running performance gate...{NC}")
    rc, out = run_perf_gate(root, dry_run=dry_run, update=update)
    if out:
        print(out, end="")
    if rc == 0:
        h.check_pass("perf gate")
    else:
        h.check_fail("perf gate")

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_perf_baseline(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[perf baseline] Collecting perf baseline...{NC}")
    rc, out = run_baseline(root, dry_run=dry_run)
    if out:
        print(out, end="")
    if rc == 0:
        h.check_pass("perf baseline")
    else:
        h.check_fail("perf baseline")

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_perf_benchmarks(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    quick: bool = not getattr(args, "no_quick", False)
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[perf benchmarks] Running benchmarks (quick={quick})...{NC}")
    rc, out = run_benchmarks(root, dry_run=dry_run, quick=quick)
    if out:
        print(out, end="")
    if rc == 0:
        h.check_pass("perf benchmarks")
    else:
        h.check_fail("perf benchmarks")

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


# ── gate subcommands ──────────────────────────────────────────────────────────


def cmd_gate_local(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    skip_ext: bool = getattr(args, "skip_ext", False)
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[gate local] Running full local CI gate...{NC}")
    rc, out = run_local(root, dry_run=dry_run, skip_ext=skip_ext)
    if out:
        print(out, end="")
    if rc == 0:
        h.check_pass("gate local")
    else:
        h.check_fail("gate local")

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_gate_pre_commit(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[gate pre-commit] Running pre-commit verification...{NC}")
    rc, out = run_pre_commit(root, dry_run=dry_run)
    if out:
        print(out, end="")
    if rc == 0:
        h.check_pass("gate pre-commit")
    else:
        h.check_fail("gate pre-commit")

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_gate_pre_push(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[gate pre-push] Running pre-push verification...{NC}")
    rc, out = run_pre_push(root, dry_run=dry_run)
    if out:
        print(out, end="")
    if rc == 0:
        h.check_pass("gate pre-push")
    else:
        h.check_fail("gate pre-push")

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_gate_repro(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[gate repro] Running reproducible build check...{NC}")
    rc, out = run_repro(
        root,
        dry_run=dry_run,
        fixture=getattr(args, "fixture", ""),
        target=getattr(args, "target", ""),
        verbose=getattr(args, "verbose", False),
    )
    if out:
        print(out, end="")
    if rc == 2:
        h.check_skip("gate repro (prereqs missing)")
    elif rc == 0:
        h.check_pass("gate repro")
    else:
        h.check_fail("gate repro")

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


if __name__ == "__main__":
    sys.exit(main())
