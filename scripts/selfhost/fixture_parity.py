"""Fast current-selfhost fixture gate.

This module replaces the historical per-fixture pinned-vs-current dual run.
The old implementation is retained only as an explicit bootstrap-reference
check via ``run_reference_fixture_parity``.

Normal fixture verification now has one oracle: the current compiler output.
Every run fixture is compiled and validated. Runtime execution is a bounded,
directory-spanning smoke sample plus all explicit trap fixtures. Set
``ARUKELLT_FIXTURE_EXECUTE_ALL=1`` to execute every fixture without reintroducing
pinned duplication.
"""
from __future__ import annotations

import concurrent.futures
import hashlib
import os
import shutil
import tempfile
from dataclasses import dataclass
from pathlib import Path

from . import checks as _checks

# Capture the historical implementation before selfhost.__init__ replaces the
# public checks.run_fixture_parity symbol. It remains available only for pinned
# bootstrap refreshes and explicit forensic comparisons.
_REFERENCE_RUN_FIXTURE_PARITY = _checks.run_fixture_parity


@dataclass(frozen=True)
class FixtureResult:
    fixture: str
    ok: bool
    executed: bool
    messages: tuple[str, ...]


def _worker_count() -> int:
    raw = os.environ.get("ARUKELLT_FIXTURE_WORKERS", "").strip()
    if raw:
        try:
            return max(1, int(raw))
        except ValueError:
            pass
    return min(4, max(1, os.cpu_count() or 1))


def _expected_path(root: Path, fixture: str) -> Path:
    return root / "tests" / "fixtures" / (fixture[:-4] + ".expected")


def _select_smoke_fixtures(root: Path, fixtures: list[str]) -> set[str]:
    """Choose one runtime fixture per top-level fixture domain.

    Prefer a fixture with a committed .expected golden. Explicit trap fixtures
    are always executed. ``ARUKELLT_FIXTURE_EXECUTE_ALL=1`` upgrades this to
    all-current runtime coverage while preserving the current-only design.
    """
    if os.environ.get("ARUKELLT_FIXTURE_EXECUTE_ALL") == "1":
        return set(fixtures)

    by_domain: dict[str, list[str]] = {}
    for fixture in sorted(fixtures):
        domain = fixture.split("/", 1)[0]
        by_domain.setdefault(domain, []).append(fixture)

    selected: set[str] = set()
    for items in by_domain.values():
        golden = next((item for item in items if _expected_path(root, item).is_file()), None)
        selected.add(golden or items[0])
    selected.update(f for f in _checks.FIXTURE_PARITY_EXPECTED_TRAPS if f in fixtures)
    return selected


def _compile_current_fixture(
    root: Path,
    wasmtime: str,
    compiler: Path,
    fixture: str,
    out_rel: str,
) -> tuple[int, str]:
    """Compile one fixture without the shared AST cache.

    Parallel fixture workers deliberately avoid ``--cache-dir``: the old
    parity path serialized all compiles, while concurrent writers to the same
    cache would make correctness depend on cache implementation details.
    """
    if compiler.suffix == ".cwasm":
        run_flags = [
            "--allow-precompiled",
            *_checks.WASMTIME_SELFHOST_WASM_FLAGS,
            "--wasm",
            "max-wasm-stack=16777216",
        ]
    else:
        run_flags = [
            *_checks.WASMTIME_SELFHOST_WASM_FLAGS,
            "--wasm",
            "max-wasm-stack=16777216",
        ]
    src_rel = str(Path("tests") / "fixtures" / fixture)
    argv = [
        wasmtime,
        "run",
        *run_flags,
        "--dir",
        str(root),
        str(compiler),
        "--",
        "compile",
        src_rel,
        "--target",
        _checks.SELFHOST_TARGET,
        "--wasi-version",
        _checks.SELFHOST_WASI_VERSION,
        "-o",
        out_rel,
    ]
    result = _checks._run(argv, root, timeout=30)
    detail = (result.stderr or result.stdout or "").strip()
    return result.returncode, detail


def _validate_wasm(root: Path, wasm_tools: str, wasm_path: Path) -> tuple[int, str]:
    """Validate with a wasm-tools path resolved once by the parent gate."""
    result = _checks._run(
        [
            wasm_tools,
            "validate",
            "--features",
            "gc,function-references,memory64",
            str(wasm_path.resolve()),
        ],
        root,
        timeout=60,
    )
    if result.returncode == 0:
        return 0, ""
    return result.returncode, (result.stderr or result.stdout or "").strip()[-800:]


def _run_one_fixture(
    root: Path,
    wasmtime: str,
    wasm_tools: str,
    compiler: Path,
    fixture: str,
    execute: bool,
    package_root: Path,
) -> FixtureResult:
    digest = hashlib.sha1(fixture.encode("utf-8"), usedforsecurity=False).hexdigest()[:12]
    out_rel = str(Path(".build") / "fixture-test" / f"{digest}.wasm")
    out = root / out_rel
    try:
        out.unlink()
    except FileNotFoundError:
        pass
    except OSError as exc:
        return FixtureResult(fixture, False, False, (f"cannot clear stale output: {exc}",))

    rc, detail = _compile_current_fixture(root, wasmtime, compiler, fixture, out_rel)
    if rc != 0 or not out.is_file():
        tail = detail[-240:] if detail else f"exit {rc}"
        return FixtureResult(fixture, False, False, (f"compile failed: {tail}",))

    val_rc, val_msg = _validate_wasm(root, wasm_tools, out)
    if val_rc != 0:
        return FixtureResult(
            fixture,
            False,
            False,
            (f"wasm validation failed: {(val_msg or str(val_rc))[-240:]}",),
        )

    if not execute:
        return FixtureResult(fixture, True, False, ())

    package_dir = package_root / digest
    runnable, package_error = _checks._package_p2_core_for_execution(
        root,
        out,
        package_dir,
        f"current-{digest}",
    )
    if runnable is None:
        return FixtureResult(
            fixture,
            False,
            True,
            (f"P2 component packaging failed: {package_error[-240:]}",),
        )

    runtime_args, read_only = _checks._fixture_runtime_args(root, fixture)
    if read_only and shutil.which("bwrap") is None:
        return FixtureResult(
            fixture,
            False,
            True,
            ("read-only fixture requires bwrap; refusing writable fallback",),
        )

    proc = _checks._run(
        _checks._wasm_run_argv(
            root,
            runnable,
            runtime_args=runtime_args,
            read_only=read_only,
        ),
        root,
        timeout=15,
    )
    output = (proc.stdout + proc.stderr).strip()

    if proc.returncode == 134:
        if fixture in _checks.FIXTURE_PARITY_EXPECTED_TRAPS:
            return FixtureResult(fixture, True, True, ())
        return FixtureResult(fixture, False, True, ("unexpected runtime trap",))

    expected_path = _expected_path(root, fixture)
    if expected_path.is_file():
        expected = expected_path.read_text(encoding="utf-8").strip()
        actual = _checks._normalize_fixture_parity_output(output)
        if actual != expected:
            return FixtureResult(
                fixture,
                False,
                True,
                (
                    "output mismatch vs .expected golden",
                    f"expected: {expected[:100]!r}",
                    f"actual:   {actual[:100]!r}",
                ),
            )
        return FixtureResult(fixture, True, True, ())

    if proc.returncode != 0:
        return FixtureResult(
            fixture,
            False,
            True,
            (f"runtime exit {proc.returncode}: {output[-200:]}",),
        )
    return FixtureResult(fixture, True, True, ())


def run_fixture_parity(
    root: Path,
    dry_run: bool,
    filter_dirs: list[str] | None = None,
) -> tuple[int, str]:
    """Run the canonical current-only parallel fixture gate.

    The function keeps the historical public name so existing callers migrate
    atomically. Semantics are intentionally different: pinned/current dual
    generation is no longer part of normal fixture testing.
    """
    if dry_run:
        return (0, "DRY-RUN: current-only parallel fixture gate\n")

    pinned = _checks._find_pinned_wasm(root)
    if pinned is None:
        return (1, f"error: pinned bootstrap missing at {_checks.PINNED_WASM_REL}\n")
    wasmtime = _checks._find_wasmtime()
    if not wasmtime:
        return (1, "error: wasmtime not found\n")
    wasm_tools = _checks._find_wasm_tools()
    if wasm_tools is None:
        return (1, "error: bytecodealliance wasm-tools not found\n")

    def prepare_current() -> tuple[Path | None, str]:
        current, err = _checks._ensure_current_selfhost(root, wasmtime, pinned)
        if current is None:
            return None, err
        return _checks._ensure_aot_cwasm(current), ""

    compiler, err = _checks._with_runtime_lock(prepare_current, root)
    if compiler is None:
        return (1, err)

    fixtures, load_err = _checks._load_manifest_fixtures(root, "run")
    if load_err:
        return (1, load_err + "\n")
    if filter_dirs:
        prefixes = tuple(
            str(Path(item).as_posix()).strip("/") + "/"
            for item in filter_dirs
            if str(item).strip("/")
        )
        fixtures = [fixture for fixture in fixtures if fixture.startswith(prefixes)]
        if not fixtures:
            return (1, f"error: no run fixtures matched --filter-dir ({', '.join(filter_dirs)})\n")
    if not fixtures:
        return (1, "error: no run fixtures found\n")

    output_dir = root / ".build" / "fixture-test"
    output_dir.mkdir(parents=True, exist_ok=True)
    smoke = _select_smoke_fixtures(root, fixtures)
    workers = _worker_count()
    lines = [
        f"[fixture-test] current-only: {len(fixtures)} compile+validate, "
        f"{len(smoke)} runtime smoke, workers={workers}"
    ]

    with tempfile.TemporaryDirectory(prefix="arukellt-fixture-components-") as tmp:
        package_root = Path(tmp)
        results: list[FixtureResult] = []
        with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
            future_map = {
                pool.submit(
                    _run_one_fixture,
                    root,
                    wasmtime,
                    wasm_tools,
                    compiler,
                    fixture,
                    fixture in smoke,
                    package_root,
                ): fixture
                for fixture in fixtures
            }
            for future in concurrent.futures.as_completed(future_map):
                fixture = future_map[future]
                try:
                    results.append(future.result())
                except Exception as exc:  # keep one worker failure diagnostic local
                    results.append(
                        FixtureResult(fixture, False, fixture in smoke, (f"worker crashed: {exc}",))
                    )

    results.sort(key=lambda item: item.fixture)
    passed = sum(1 for item in results if item.ok)
    failed = len(results) - passed
    executed = sum(1 for item in results if item.executed)
    for item in results:
        if item.ok:
            continue
        lines.append(f"  FAIL: {item.fixture}")
        lines.extend(f"    {message}" for message in item.messages)
    lines.append(
        f"[fixture-test] PASS={passed} FAIL={failed} "
        f"COMPILE_VALIDATE={len(results)} EXECUTED={executed}"
    )
    return (0 if failed == 0 else 1, "\n".join(lines) + "\n")


def run_reference_fixture_parity(
    root: Path,
    dry_run: bool,
    filter_dirs: list[str] | None = None,
) -> tuple[int, str]:
    """Run the retired pinned-vs-current oracle for bootstrap refresh only."""
    return _REFERENCE_RUN_FIXTURE_PARITY(root, dry_run, filter_dirs=filter_dirs)
