"""Canonical formatter, linter, and quality-gate orchestration."""

from __future__ import annotations

import concurrent.futures
import hashlib
import json
import os
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath

try:
    from .metrics import run_metrics_report
    from .structure import quality_contract_findings, run_structure
except ImportError:  # manager.py executes with scripts/ on sys.path
    from quality.metrics import run_metrics_report
    from quality.structure import quality_contract_findings, run_structure

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 on supported development hosts
    import tomli as tomllib  # type: ignore[no-redef]


ARK_PACKAGE_ROOTS = ("src/compiler/", "std/")
TEXT_FINAL_NEWLINE_SUFFIXES = {
    ".ark", ".css", ".html", ".js", ".json", ".jsonc", ".py", ".rs",
    ".sh", ".toml", ".ts", ".wit", ".yaml", ".yml",
}
TEXT_TRAILING_WHITESPACE_SUFFIXES = {
    ".css", ".html", ".js", ".json", ".jsonc", ".py", ".rs", ".sh",
    ".toml", ".ts", ".yaml", ".yml",
}
INDENT_SUFFIXES = {
    ".ark", ".css", ".html", ".js", ".json", ".jsonc", ".md", ".py",
    ".rs", ".sh", ".toml", ".ts", ".wit", ".yaml", ".yml",
}


@dataclass(frozen=True)
class ToolResult:
    path: str
    command: tuple[str, ...]
    returncode: int
    output: str


def _git_paths(root: Path, args: list[str]) -> list[str]:
    result = subprocess.run(
        ["git", *args, "-z"], cwd=root, capture_output=True, check=False,
    )
    if result.returncode != 0:
        return []
    return sorted(
        raw.decode("utf-8", errors="surrogateescape")
        for raw in result.stdout.split(b"\0")
        if raw
    )


def quality_base(root: Path) -> str:
    requested = os.environ.get("ARUKELLT_QUALITY_BASE", "")
    candidates = [requested, "HEAD^", "HEAD"] if requested else ["HEAD"]
    for candidate in candidates:
        result = subprocess.run(
            ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
            cwd=root,
            capture_output=True,
            check=False,
        )
        if result.returncode == 0:
            return candidate
    return "HEAD"


def changed_paths(root: Path, base: str | None = None) -> list[str]:
    compare_base = base or quality_base(root)
    paths = set(
        _git_paths(root, ["diff", "--name-only", "--diff-filter=ACMR", compare_base])
    )
    paths.update(_git_paths(root, ["ls-files", "--others", "--exclude-standard"]))
    return sorted(path for path in paths if (root / path).is_file())


def tracked_paths(root: Path) -> list[str]:
    return _git_paths(root, ["ls-files"])


def _load_toml(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def _ark_inventory(root: Path) -> dict:
    inventory = _load_toml(root / "docs/data/tooling-inventory.toml")
    for family in inventory.get("families", []):
        if family.get("ext") == ".ark":
            return family
    raise ValueError("tooling inventory has no .ark family")


def ark_paths(root: Path, requested: list[str] | None = None) -> list[str]:
    if requested:
        candidates: set[str] = set()
        for raw in requested:
            path = Path(raw)
            absolute = path if path.is_absolute() else root / path
            if absolute.is_dir():
                candidates.update(
                    str(item.relative_to(root)).replace("\\", "/")
                    for item in absolute.rglob("*.ark")
                    if item.is_file()
                )
            elif absolute.is_file() and absolute.suffix == ".ark":
                candidates.add(str(absolute.relative_to(root)).replace("\\", "/"))
        return sorted(candidates)

    family = _ark_inventory(root)
    roots = tuple(family.get("roots", []))
    excludes = tuple(family.get("exclude_globs", []))
    selected: list[str] = []
    for rel in tracked_paths(root):
        if not rel.endswith(".ark") or not rel.startswith(roots):
            continue
        pure = PurePosixPath(rel)
        if any(pure.match(pattern) for pattern in excludes):
            continue
        selected.append(rel)
    return selected


def _has_enforced_ark_sources(root: Path) -> bool:
    try:
        roots = _ark_inventory(root).get("roots", [])
    except (OSError, KeyError, ValueError, tomllib.TOMLDecodeError):
        return False
    return any(
        any((root / source_root).rglob("*.ark"))
        for source_root in roots
        if (root / source_root).is_dir()
    )


def _empty_selection_failure(root: Path, selected: list[str], label: str) -> int:
    if selected or not _has_enforced_ark_sources(root):
        return 0
    print(
        f"{label}: FAIL: enforced Ark sources exist but the canonical inventory selected 0 files"
    )
    return 1


def _run_tool(root: Path, path: str, command: tuple[str, ...], dry_run: bool, timeout: int = 60) -> ToolResult:
    if dry_run:
        return ToolResult(path, command, 0, "DRY-RUN: " + " ".join(command))
    env = os.environ.copy()
    current_s2 = root / ".build/selfhost/arukellt-s2.wasm"
    if current_s2.is_file():
        env["ARUKELLT_SELFHOST_WASM"] = str(current_s2)
    try:
        result = subprocess.run(
            command,
            cwd=root,
            capture_output=True,
            text=True,
            check=False,
            env=env,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as exc:
        def timeout_text(value: str | bytes | None) -> str:
            if isinstance(value, bytes):
                return value.decode("utf-8", errors="replace")
            return value or ""

        output = timeout_text(exc.stdout) + timeout_text(exc.stderr)
        return ToolResult(path, command, 124, f"timeout after {timeout}s\n{output}")
    output = result.stdout + result.stderr
    output = "\n".join(
        line
        for line in output.splitlines()
        if not line.startswith("bash: warning: setlocale:")
    )
    if output:
        output += "\n"
    return ToolResult(path, command, result.returncode, output)


def _run_parallel(
    root: Path,
    jobs: list[tuple[str, tuple[str, ...]]],
    dry_run: bool,
    timeout: int = 60,
) -> list[ToolResult]:
    workers = min(16, max(1, os.cpu_count() or 1))
    with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as executor:
        futures = [
            executor.submit(_run_tool, root, path, command, dry_run, timeout)
            for path, command in jobs
        ]
        return [future.result() for future in futures]


def _print_results(label: str, results: list[ToolResult], json_output: bool) -> int:
    failures = [result for result in results if result.returncode != 0]
    baseline_skips = [result for result in results if result.output.startswith("BASELINE:")]
    reported = [
        result
        for result in results
        if result.returncode != 0 or (label == "lint" and result.output.strip())
    ]
    if json_output:
        print(json.dumps({
            "command": label,
            "checked": len(results),
            "failed": len(failures),
            "diagnostics": [
                {
                    "path": result.path,
                    "rule_id": "CQ-FMT-001" if label == "fmt" else "CQ-LINT-001",
                    "autofix": label == "fmt",
                    "exit_code": result.returncode,
                    "message": result.output.strip(),
                }
                for result in reported
            ],
        }, ensure_ascii=False))
    else:
        if label == "lint":
            for result in results:
                if result.returncode == 0 and result.output.strip() and not result.output.startswith("BASELINE:"):
                    print(result.output.rstrip())
        for result in failures:
            print(f"FAIL: {result.path}")
            if result.output.strip():
                print(result.output.rstrip())
        if results and results[0].output.startswith("DRY-RUN:"):
            for result in results:
                print(result.output)
        print(
            f"{label}: checked={len(results)} failed={len(failures)} "
            f"baseline-skipped={len(baseline_skips)}"
        )
    return 1 if failures else 0


def _formatter_baseline(root: Path) -> dict[str, str]:
    path = root / "docs/data/ark-formatter-baseline.toml"
    if not path.is_file():
        return {}
    data = _load_toml(path)
    return {
        entry["path"]: entry["sha256"]
        for entry in data.get("exceptions", [])
    }


def _content_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _apply_parser_failure_baseline(root: Path, results: list[ToolResult]) -> list[ToolResult]:
    baseline = _formatter_baseline(root)
    adjusted: list[ToolResult] = []
    for result in results:
        expected = baseline.get(result.path)
        path = root / result.path
        if result.returncode != 0 and expected and path.is_file():
            actual = _content_sha256(path)
            if actual == expected:
                adjusted.append(ToolResult(
                    result.path,
                    result.command,
                    0,
                    f"BASELINE: canonical parser exception tracked by issue #791 ({expected})",
                ))
                continue
        adjusted.append(result)
    return adjusted


def _chunk(paths: list[str], size: int):
    for i in range(0, len(paths), size):
        yield paths[i:i + size]


def _match_batch_path(rest: str, batch_paths: list[str]) -> str | None:
    for path in batch_paths:
        if rest == path:
            return path
    return None


_FMT_BATCH_STATUS_PREFIXES: tuple[tuple[str, int], ...] = (
    ("ok: ", 0),
    ("formatted: ", 0),
    ("fixed: ", 0),
    ("fmt check failed: ", 1),
    ("fmt parse error: ", 1),
)


def _split_batch_fmt_result(result: ToolResult, batch_paths: list[str]) -> list[ToolResult]:
    """Deinterleave one multi-file ``arukellt fmt`` run into per-file results.

    ``arukellt fmt`` emits a status line per file (``ok: <path>``,
    ``fmt check failed: <path>``, ``fmt parse error: <path>``, ``formatted: ``,
    ``fixed: ``) followed by an optional ``fmt: checked=N failed=M`` summary.
    Parse-error detail lines (``E0001:`` …) attach to the preceding parse-error
    file.  When the output cannot be parsed (timeout, crash), the batch result
    is returned unchanged so the failure is still reported.
    """
    if result.returncode == 124:
        return [result]
    by_path: dict[str, int] = {}
    detail_by_path: dict[str, list[str]] = {}
    current_error_path: str | None = None
    for line in result.output.splitlines():
        matched_prefix: str | None = None
        for prefix, rc in _FMT_BATCH_STATUS_PREFIXES:
            if line.startswith(prefix):
                matched_prefix = prefix
                rest = line[len(prefix):]
                path = _match_batch_path(rest, batch_paths)
                if path is not None:
                    by_path[path] = rc
                    detail_by_path[path] = []
                    current_error_path = path if prefix == "fmt parse error: " else None
                break
        if matched_prefix is not None:
            continue
        if current_error_path is not None:
            detail_by_path[current_error_path].append(line)
    if not by_path:
        return [result]
    results: list[ToolResult] = []
    for path in batch_paths:
        if path in by_path:
            detail = detail_by_path.get(path, [])
            detail_text = ("\n".join(detail) + "\n") if detail else ""
            results.append(ToolResult(path, result.command, by_path[path], detail_text))
        else:
            results.append(ToolResult(path, result.command, result.returncode, ""))
    return results


def run_fmt(root: Path, paths: list[str], check: bool, dry_run: bool, json_output: bool) -> int:
    wrapper = str(root / "scripts/run/arukellt-selfhost.sh")
    selected = ark_paths(root, paths)
    if _empty_selection_failure(root, selected, "fmt"):
        return 1
    # Baseline (known parser-exception) files need per-file results so their
    # content hash can be checked against ark-formatter-baseline.toml.  All
    # other files are batched into one ``arukellt fmt`` invocation per batch to
    # amortize the wasmtime cold-start cost (one process per ~80 files instead
    # of one per file).
    baseline = {} if dry_run else _formatter_baseline(root)
    baseline_paths = [p for p in selected if p in baseline]
    other_paths = [p for p in selected if p not in baseline]
    workers = min(16, max(1, os.cpu_count() or 1))
    batch_count = max(1, workers * 2)
    batch_size = max(1, (len(other_paths) + batch_count - 1) // batch_count) if other_paths else 1
    jobs: list[tuple[str, tuple[str, ...]]] = []
    batch_groups: list[list[str]] = []
    for path in baseline_paths:
        args = [wrapper, "fmt"]
        if check:
            args.append("--check")
        args.append(path)
        jobs.append((path, tuple(args)))
    for batch in _chunk(other_paths, batch_size):
        label = f"batch:{len(batch_groups)}"
        args = [wrapper, "fmt"]
        if check:
            args.append("--check")
        args.extend(batch)
        jobs.append((label, tuple(args)))
        batch_groups.append(list(batch))
    results = _run_parallel(root, jobs, dry_run, timeout=180)
    per_file: list[ToolResult] = []
    batch_idx = 0
    for (job_path, _), result in zip(jobs, results):
        if job_path.startswith("batch:"):
            per_file.extend(_split_batch_fmt_result(result, batch_groups[batch_idx]))
            batch_idx += 1
        else:
            per_file.append(result)
    if not dry_run:
        per_file = _apply_parser_failure_baseline(root, per_file)
    return _print_results("fmt", per_file, json_output)


def run_lint(
    root: Path,
    paths: list[str],
    fix: bool,
    dry_run: bool,
    json_output: bool,
    deny_prefer_else_if: bool = False,
) -> int:
    selected = ark_paths(root, paths)
    if _empty_selection_failure(root, selected, "lint"):
        return 1
    if fix:
        fmt_rc = run_fmt(root, selected, check=False, dry_run=dry_run, json_output=json_output)
        if fmt_rc != 0:
            return fmt_rc
    wrapper = str(root / "scripts/run/arukellt-selfhost.sh")
    jobs = []
    for path in selected:
        args = [wrapper, "lint"]
        if path.startswith(ARK_PACKAGE_ROOTS):
            args.append("--local")
        if deny_prefer_else_if:
            args.extend(("--deny", "prefer-else-if"))
        args.append(path)
        jobs.append((path, tuple(args)))
    results = _run_parallel(root, jobs, dry_run)
    if not dry_run:
        results = _apply_parser_failure_baseline(root, results)
    return _print_results("lint", results, json_output)


def run_lint_command(
    root: Path,
    paths: list[str],
    fix: bool,
    dry_run: bool,
    json_output: bool,
) -> int:
    failures = run_lint(root, paths, fix, dry_run, json_output)
    smoke = _run_command(root, ["python3", "scripts/check/check-ark-lint-smoke.py"], dry_run)
    return 1 if failures or smoke else 0


def _lint_w0011_count(root: Path, path: str) -> tuple[int, int, str]:
    wrapper = str(root / "scripts/run/arukellt-selfhost.sh")
    result = _run_tool(root, path, (wrapper, "lint", "--local", path), dry_run=False)
    return result.returncode, result.output.count("[W0011|"), result.output


def _base_lint_w0011_count(root: Path, path: str, base: str) -> tuple[int, str]:
    source = subprocess.run(
        ["git", "show", f"{base}:{path}"],
        cwd=root,
        capture_output=True,
        check=False,
    )
    if source.returncode != 0:
        return 0, ""
    fixture_dir = root / ".build/quality-lint-base"
    fixture_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(suffix=".ark", dir=fixture_dir) as fixture:
        fixture.write(source.stdout)
        fixture.flush()
        fixture_path = str(Path(fixture.name).relative_to(root))
        rc, count, output = _lint_w0011_count(root, fixture_path)
    if rc != 0:
        return 0, output
    return count, ""


def run_lint_ratchet(
    root: Path,
    paths: list[str],
    base: str,
    dry_run: bool,
    json_output: bool,
) -> int:
    if dry_run:
        print(f"DRY-RUN: W0011 ratchet ({len(paths)} files vs {base})")
        return 0
    failures: list[dict[str, object]] = []
    for path in paths:
        current_rc, current_count, current_output = _lint_w0011_count(root, path)
        if current_rc != 0:
            failures.append({
                "path": path,
                "current": current_count,
                "base": 0,
                "message": current_output.strip() or "local lint failed",
            })
            continue
        base_count, base_error = _base_lint_w0011_count(root, path, base)
        if base_error:
            failures.append({
                "path": path,
                "current": current_count,
                "base": base_count,
                "message": f"base local lint failed: {base_error.strip()}",
            })
            continue
        if current_count > base_count:
            failures.append({
                "path": path,
                "current": current_count,
                "base": base_count,
                "message": f"W0011 count increased: {current_count} > {base_count}",
            })
    if json_output:
        print(json.dumps({
            "command": "lint-ratchet",
            "base": base,
            "checked": len(paths),
            "failed": len(failures),
            "diagnostics": failures,
        }, ensure_ascii=False))
    else:
        for failure in failures:
            print(f"FAIL: {failure['path']}: {failure['message']}")
        print(f"lint ratchet: checked={len(paths)} failed={len(failures)} base={base}")
    return 1 if failures else 0


def check_editorconfig_basics(root: Path, paths: list[str] | None = None) -> int:
    config_path = root / ".editorconfig"
    required = ("root = true", "indent_style = space", "indent_size = 4")
    failures: list[str] = []
    if not config_path.is_file():
        failures.append("CQ-FMT-002: missing .editorconfig")
    else:
        config = config_path.read_text(encoding="utf-8")
        for marker in required:
            if marker not in config:
                failures.append(f"CQ-FMT-002: .editorconfig missing `{marker}`")

    selected = paths if paths is not None else tracked_paths(root)
    for rel in selected:
        if rel.startswith("docs/playground/dist/"):
            continue
        path = root / rel
        if not path.is_file():
            continue
        try:
            data = path.read_bytes()
            if b"\0" in data:
                continue
            text = data.decode("utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        suffix = path.suffix
        if suffix in TEXT_FINAL_NEWLINE_SUFFIXES and data and not data.endswith(b"\n"):
            failures.append(f"CQ-FMT-002: {rel}: missing final newline")
        for line_no, line in enumerate(text.splitlines(), 1):
            if suffix in INDENT_SUFFIXES and line.startswith("\t"):
                failures.append(f"CQ-FMT-002: {rel}:{line_no}: tab indentation")
            if suffix in TEXT_TRAILING_WHITESPACE_SUFFIXES and line.rstrip(" \t") != line:
                failures.append(f"CQ-FMT-002: {rel}:{line_no}: trailing whitespace")

    for failure in failures[:100]:
        print(failure)
    if len(failures) > 100:
        print(f"... and {len(failures) - 100} more")
    if failures:
        print(f"editorconfig basics: FAIL ({len(failures)})")
        return 1
    print(f"editorconfig basics: PASS ({len(selected)} files)")
    return 0


def check_quality_contract(root: Path) -> int:
    findings = quality_contract_findings(root)
    for finding in findings:
        print(f"quality contract: {finding.path}:{finding.line}: {finding.message}")
    if findings:
        return 1
    rules = _load_toml(root / "docs/data/code-quality-rules.toml").get("rules", [])
    inventory = _load_toml(root / "docs/data/tooling-inventory.toml").get("families", [])
    print(f"quality contract: PASS ({len(rules)} rules, {len(inventory)} families)")
    return 0


def _run_command(root: Path, command: list[str], dry_run: bool) -> int:
    if dry_run:
        print("DRY-RUN: " + " ".join(command))
        return 0
    return subprocess.run(command, cwd=root, check=False).returncode


def run_quality(
    root: Path,
    mode: str,
    dry_run: bool,
    json_output: bool = False,
    output: str | None = None,
    write_baseline: bool = False,
    issue: int | None = None,
    reason: str | None = None,
) -> int:
    if mode == "structure":
        if dry_run:
            print("DRY-RUN: repository structure contracts")
            return 0
        return run_structure(root, json_output=json_output)
    if mode == "report":
        if dry_run:
            print("DRY-RUN: advisory metrics and hotspot report")
            return 0
        return run_metrics_report(
            root,
            json_output=json_output,
            output=output,
            write_baseline=write_baseline,
            issue=issue,
            reason=reason,
        )

    base = quality_base(root)
    selected = changed_paths(root, base) if mode in {"changed", "quick", "full"} else None
    ark_selected = [path for path in (selected or []) if path.endswith(".ark")]
    failures = 0
    failures += check_editorconfig_basics(root, selected) if not dry_run else _run_command(
        root, ["python3", "scripts/check/check-editorconfig-basics.py"], True,
    )
    failures += check_quality_contract(root) if not dry_run else _run_command(
        root, ["python3", "scripts/check/check-code-quality-contract.py"], True,
    )
    failures += _run_command(root, ["python3", "scripts/check/check-comment-policy.py"], dry_run)
    failures += run_fmt(
        root,
        ark_selected if mode in {"changed", "quick"} else [],
        True,
        dry_run,
        json_output,
    )
    if mode == "changed":
        failures += run_lint_command(root, ark_selected, False, dry_run, json_output)
        failures += run_lint_ratchet(root, ark_selected, base, dry_run, json_output)
        failures += _run_command(
            root,
            [
                "python3", "scripts/check/check-ark-code-quality.py",
                "--changed", "--base", base,
            ],
            dry_run,
        )
    elif mode == "quick":
        failures += run_lint_command(root, ark_selected, False, dry_run, json_output)
        failures += run_lint_ratchet(root, ark_selected, base, dry_run, json_output)
        failures += _run_command(
            root,
            [
                "python3", "scripts/check/check-ark-code-quality.py",
                "--changed", "--base", base,
            ],
            dry_run,
        )
        failures += run_structure(root, json_output=json_output) if not dry_run else 0
    else:
        failures += run_lint_command(root, [], False, dry_run, json_output)
        failures += _run_command(root, ["python3", "scripts/check/check-ark-code-quality.py"], dry_run)
        failures += run_lint_ratchet(root, ark_selected, base, dry_run, json_output)
        failures += _run_command(
            root,
            [
                "python3", "scripts/check/check-ark-code-quality.py",
                "--changed", "--base", base,
            ],
            dry_run,
        )
        failures += run_structure(root, json_output=json_output) if not dry_run else 0
        failures += run_metrics_report(root, json_output=json_output) if not dry_run else 0
    return 1 if failures else 0
