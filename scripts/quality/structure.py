"""Canonical repository-structure findings and renderers."""

from __future__ import annotations

import json
import hashlib
import re
import shlex
import subprocess
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Callable, Iterable

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 on supported development hosts
    import tomli as tomllib  # type: ignore[no-redef]


SCHEMA_VERSION = 1
REQUIRED_NAMESPACES = (
    "compiler",
    "component",
    "corehir",
    "diagnostics",
    "driver",
    "fmt",
    "hir",
    "lexer",
    "mir",
    "mir/lower",
    "parser",
    "resolver",
    "typechecker",
    "wasm",
    "wasm/intrinsics",
)
REQUIRED_BOUNDARY_MODULES = (
    "src/compiler/component/mod.ark",
    "src/compiler/corehir/mod.ark",
    "src/compiler/wasm/mod.ark",
    "src/compiler/mir/mod.ark",
    "src/compiler/diagnostics/mod.ark",
    "src/compiler/fmt/mod.ark",
    "src/compiler/parser/mod.ark",
    "src/compiler/resolver/mod.ark",
    "src/compiler/typechecker/mod.ark",
)
ALLOWED_COMPILER_ROOT_FILES = {
    "analysis.ark",
    "ark.toml",
    "component_emit.ark",
    "component_emitter.ark",
    "corehir.ark",
    "dap.ark",
    "diagnostics.ark",
    "driver.ark",
    "emit_wat.ark",
    "emitter.ark",
    "hir.ark",
    "lint.ark",
    "lsp.ark",
    "main.ark",
    "mir_dump.ark",
    "mir_lower.ark",
    "native.ark",
    "parser.ark",
    "resolver.ark",
    "target.ark",
    "typechecker.ark",
}
FACADE_REEXPORT_CYCLES = frozenset(
    {
        frozenset(
            {
                "src/compiler/resolver/program.ark",
                "src/compiler/resolver.ark",
                "src/compiler/resolver/mod.ark",
            }
        ),
        frozenset(
            {
                "src/compiler/typechecker/entry.ark",
                "src/compiler/typechecker.ark",
                "src/compiler/typechecker/mod.ark",
            }
        ),
    }
)
ALLOWED_RULE_SEVERITIES = {"error", "warning", "advisory"}
ALLOWED_RULE_GATES = {"local", "quick", "full"}
ALLOWED_TOOL_STATUSES = {"enforced", "deferred"}


@dataclass(frozen=True, order=True)
class Finding:
    rule_id: str
    severity: str
    path: str
    line: int
    message: str
    owner: str


@dataclass(frozen=True)
class StructureReport:
    findings: tuple[Finding, ...]

    @property
    def status(self) -> str:
        return "fail" if any(item.severity == "error" for item in self.findings) else "pass"

    @property
    def summary(self) -> dict[str, int]:
        return {
            "errors": sum(item.severity == "error" for item in self.findings),
            "warnings": sum(item.severity == "warning" for item in self.findings),
            "advisories": sum(item.severity == "advisory" for item in self.findings),
        }

    def to_dict(self) -> dict[str, object]:
        return {
            "schema_version": SCHEMA_VERSION,
            "status": self.status,
            "summary": self.summary,
            "findings": [asdict(item) for item in self.findings],
        }


def _load_toml(path: Path) -> dict:
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise ValueError(f"{path}: {exc}") from exc


def _finding(
    rule_id: str,
    path: str,
    message: str,
    owner: str = "compiler",
    line: int = 1,
    severity: str = "error",
) -> Finding:
    return Finding(rule_id, severity, path, line, message, owner)


def _compiler_import_targets(compiler_root: Path, path: Path) -> list[Path]:
    targets: list[Path] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped.startswith("use "):
            continue
        module = stripped[4:].split()[0]
        if module.startswith("std::"):
            continue
        module_path = compiler_root / f"{module.replace('::', '/')}.ark"
        if not module_path.is_file():
            module_path = compiler_root / f"{module.split('::')[0]}.ark"
        if module_path.is_file():
            targets.append(module_path)
    return targets


def compiler_import_graph(root: Path) -> dict[Path, list[Path]]:
    compiler_root = root / "src/compiler"
    return {
        path: _compiler_import_targets(compiler_root, path)
        for path in sorted(compiler_root.rglob("*.ark"))
        if path.is_file()
    }


def _is_facade_reexport_cycle(cycle: list[str]) -> bool:
    return (
        len(cycle) >= 2
        and cycle[0] == cycle[-1]
        and frozenset(cycle[:-1]) in FACADE_REEXPORT_CYCLES
    )


def _compiler_import_cycle_violations(root: Path) -> list[list[str]]:
    graph = compiler_import_graph(root)
    visiting: set[Path] = set()
    visited: set[Path] = set()
    stack: list[Path] = []
    cycles: list[list[str]] = []

    def visit(path: Path) -> None:
        if path in visited:
            return
        if path in visiting:
            start = stack.index(path)
            paths = stack[start:] + [path]
            cycles.append([str(item.relative_to(root)) for item in paths])
            return
        visiting.add(path)
        stack.append(path)
        for dependency in graph.get(path, []):
            visit(dependency)
        stack.pop()
        visiting.remove(path)
        visited.add(path)

    for path in sorted(graph):
        visit(path)
    return [cycle for cycle in cycles if not _is_facade_reexport_cycle(cycle)]


def _compiler_dependency_direction_violations(root: Path) -> list[tuple[str, int, str]]:
    rules: tuple[tuple[str, tuple[str, ...]], ...] = (
        (
            "corehir*.ark",
            ("use mir", "use mir::", "use mir_lower", "use emitter", "use emit_", "use component", "use driver"),
        ),
        ("mir_lower*.ark", ("use emitter", "use emit_", "use component", "use driver")),
        (
            "emit*.ark",
            ("use mir_lower", "use component", "use driver", "use parser", "use typechecker"),
        ),
        (
            "emitter.ark",
            ("use mir_lower", "use component", "use driver", "use parser", "use typechecker"),
        ),
        ("component*.ark", ("use mir_lower", "use driver", "use parser", "use typechecker")),
    )
    compiler_root = root / "src/compiler"
    violations: list[tuple[str, int, str]] = []
    for pattern, forbidden in rules:
        paths = list(compiler_root.glob(pattern))
        if pattern == "corehir*.ark":
            paths.extend((compiler_root / "corehir").rglob("*.ark"))
        for path in sorted(set(paths)):
            if not path.is_file():
                continue
            rel = str(path.relative_to(root))
            for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                stripped = line.strip()
                if any(stripped.startswith(prefix) for prefix in forbidden):
                    violations.append((rel, line_no, stripped))
    return violations


def _compiler_production_test_reachability_violations(root: Path) -> list[str]:
    compiler_root = root / "src/compiler"
    graph = compiler_import_graph(root)
    entry = compiler_root / "main.ark"
    seen: set[Path] = set()
    stack = [entry]
    while stack:
        path = stack.pop()
        if path in seen or not path.is_file():
            continue
        seen.add(path)
        stack.extend(graph.get(path, []))
    return [
        str(path.relative_to(root))
        for path in sorted(seen)
        if any(token in path.name for token in ("smoke", "fixture", "self_check"))
    ]


def _compiler_root_layout_violations(root: Path) -> list[str]:
    compiler_root = root / "src/compiler"
    return [
        str(path.relative_to(root))
        for path in sorted(compiler_root.glob("*.ark"))
        if path.name not in ALLOWED_COMPILER_ROOT_FILES
    ]


def _compiler_namespace_layout_violations(root: Path) -> list[str]:
    compiler_root = root / "src/compiler"
    violations: list[str] = []
    for rel in REQUIRED_NAMESPACES:
        directory = compiler_root / rel
        if not directory.is_dir():
            violations.append(f"missing directory: src/compiler/{rel}/")
        elif not any(directory.glob("*.ark")):
            violations.append(f"empty namespace: src/compiler/{rel}/")
    return violations


def _compiler_public_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    violations: list[tuple[str, int, str]] = []
    for rel in REQUIRED_BOUNDARY_MODULES:
        if not (root / rel).is_file():
            violations.append((rel, 1, "missing public boundary mod.ark"))

    public_mod_only_dirs = (
        "src/compiler/component",
        "src/compiler/wasm",
        "src/compiler/mir",
        "src/compiler/corehir",
        "src/compiler/fmt",
        "src/compiler/resolver",
        "src/compiler/typechecker",
    )
    for rel_dir in public_mod_only_dirs:
        subsystem_root = root / rel_dir
        if not subsystem_root.is_dir():
            continue
        for path in sorted(subsystem_root.rglob("*.ark")):
            if path.name == "mod.ark":
                continue
            rel = str(path.relative_to(root))
            for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                stripped = line.strip()
                if stripped.startswith("pub fn "):
                    violations.append((rel, line_no, stripped))

    forbidden_imports = (
        "use component::emit",
        "use component::wit_text",
        "use component::contract",
        "use wasm::wasm",
        "use wasm::wat",
        "use mir::lower",
        "use mir::input",
        "use mir::fallback_source",
        "use mir::legacy_decl",
        "use mir::reachability",
        "use mir::dump_core",
        "use corehir::frontend_checked",
        "use corehir::mir_view",
        "use component_emit",
        "use component_emitter",
        "use emit_wat",
        "use emitter",
        "use mir_dump",
        "use mir_lower",
    )
    compiler_root = root / "src/compiler"
    for path in sorted(compiler_root.rglob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        inside = str(path.relative_to(compiler_root))
        if inside.startswith(("component/", "corehir/", "wasm/", "mir/")):
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(stripped.startswith(prefix) for prefix in forbidden_imports):
                violations.append((rel, line_no, stripped))
    return violations


def _ci_jobs(workflow: str) -> set[str]:
    in_jobs = False
    jobs: set[str] = set()
    for line in workflow.splitlines():
        if line == "jobs:":
            in_jobs = True
            continue
        if in_jobs and line and not line.startswith(" "):
            break
        match = re.match(r"^  ([A-Za-z0-9_-]+):\s*$", line) if in_jobs else None
        if match:
            jobs.add(match.group(1))
    return jobs


def _ci_job_body(workflow: str, job: str) -> str:
    match = re.search(rf"(?ms)^  {re.escape(job)}:\s*$\n(.*?)(?=^  [A-Za-z0-9_-]+:\s*$|\Z)", workflow)
    return match.group(1) if match else ""


def _manager_command_exists(root: Path, canonical: str) -> bool:
    tokens = shlex.split(canonical)
    if len(tokens) < 3 or tokens[0] not in {"python", "python3"}:
        return True
    if tokens[1] != "scripts/manager.py":
        return (root / tokens[1]).is_file()
    manager = root / tokens[1]
    if not manager.is_file():
        return False
    result = subprocess.run(
        [sys.executable, str(manager), *tokens[2:], "--help"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
        timeout=20,
    )
    return result.returncode == 0


def quality_contract_findings(root: Path) -> list[Finding]:
    findings: list[Finding] = []
    try:
        rules = _load_toml(root / "docs/data/code-quality-rules.toml").get("rules", [])
        inventory = _load_toml(root / "docs/data/tooling-inventory.toml").get("families", [])
        commands = _load_toml(root / "docs/data/verification-commands.toml").get("commands", [])
    except ValueError as exc:
        return [_finding("CQ-STRUCT-009", "docs/data", str(exc), "tooling")]

    command_ids: set[str] = set()
    canonical_commands: dict[str, str] = {}
    for command in commands:
        command_id = command.get("id", "")
        if not command_id or command_id in command_ids:
            findings.append(_finding("CQ-STRUCT-009", "docs/data/verification-commands.toml", f"duplicate or empty command id: {command_id}", "tooling"))
        command_ids.add(command_id)
        canonical = command.get("canonical", "")
        canonical_commands[command_id] = canonical
        if not canonical or not _manager_command_exists(root, canonical):
            findings.append(_finding("CQ-STRUCT-009", "docs/data/verification-commands.toml", f"canonical command does not exist: {canonical or command_id}", "tooling"))

    workflow_path = root / ".github/workflows/ci.yml"
    workflow = workflow_path.read_text(encoding="utf-8") if workflow_path.is_file() else ""
    jobs = _ci_jobs(workflow)
    required_rule_fields = {
        "id", "scope", "category", "enforcer", "severity", "gate", "autofix",
        "rationale", "exception_policy", "owner", "adr",
    }
    rule_ids: set[str] = set()
    for index, rule in enumerate(rules, 1):
        rule_id = rule.get("id", "")
        missing = required_rule_fields - rule.keys()
        if missing:
            findings.append(_finding("CQ-STRUCT-009", "docs/data/code-quality-rules.toml", f"rule {index} missing fields: {', '.join(sorted(missing))}", "tooling"))
        if not rule_id or rule_id in rule_ids:
            findings.append(_finding("CQ-STRUCT-009", "docs/data/code-quality-rules.toml", f"duplicate or empty rule id: {rule_id}", "tooling"))
        rule_ids.add(rule_id)
        if rule.get("severity") not in ALLOWED_RULE_SEVERITIES:
            findings.append(_finding("CQ-STRUCT-009", "docs/data/code-quality-rules.toml", f"{rule_id}: invalid severity {rule.get('severity')}", "tooling"))
        if rule.get("gate") not in ALLOWED_RULE_GATES:
            findings.append(_finding("CQ-STRUCT-009", "docs/data/code-quality-rules.toml", f"{rule_id}: invalid gate {rule.get('gate')}", "tooling"))
        for command_id in rule.get("commands", []):
            if command_id not in command_ids:
                findings.append(_finding("CQ-STRUCT-009", "docs/data/code-quality-rules.toml", f"{rule_id}: unknown command id {command_id}", "tooling"))
        for job in rule.get("ci_jobs", []):
            if job not in jobs:
                findings.append(_finding("CQ-STRUCT-009", ".github/workflows/ci.yml", f"{rule_id}: referenced CI job does not exist: {job}", "tooling"))
        adr_number = rule.get("adr", "")
        adr_matches = list((root / "docs/adr").glob(f"{adr_number}-*.md"))
        if not adr_matches or "ステータス: **ACCEPTED**" not in adr_matches[0].read_text(encoding="utf-8"):
            findings.append(_finding("CQ-STRUCT-009", "docs/adr", f"{rule_id}: accepted ADR does not exist: {adr_number}", "docs"))

    extensions: set[str] = set()
    for family in inventory:
        ext = family.get("ext", "")
        if not ext or ext in extensions:
            findings.append(_finding("CQ-STRUCT-009", "docs/data/tooling-inventory.toml", f"duplicate or empty tooling family: {ext}", "tooling"))
        extensions.add(ext)
        if not family.get("owner"):
            findings.append(_finding("CQ-STRUCT-009", "docs/data/tooling-inventory.toml", f"{ext}: missing owner", "tooling"))
        if family.get("status") == "enforced" and not (family.get("canonical_fmt") or family.get("canonical_lint")):
            findings.append(_finding("CQ-STRUCT-009", "docs/data/tooling-inventory.toml", f"{ext}: enforced tool has no canonical entrypoint", "tooling"))
        if family.get("status") not in ALLOWED_TOOL_STATUSES:
            findings.append(_finding("CQ-STRUCT-009", "docs/data/tooling-inventory.toml", f"{ext}: invalid status {family.get('status')}", "tooling"))
        for entrypoint in (family.get("canonical_fmt", ""), family.get("canonical_lint", "")):
            if family.get("status") == "enforced" and entrypoint and not _manager_command_exists(root, entrypoint):
                findings.append(_finding("CQ-STRUCT-009", "docs/data/tooling-inventory.toml", f"{ext}: canonical entrypoint does not exist: {entrypoint}", "tooling"))

    for job, command in (
        ("quality-format", "python3 scripts/manager.py fmt --check"),
        ("quality-lint", "python3 scripts/manager.py lint"),
    ):
        body = _ci_job_body(workflow, job)
        if not body:
            findings.append(_finding("CQ-STRUCT-009", ".github/workflows/ci.yml", f"required CI job does not exist: {job}", "tooling"))
            continue
        run_lines = [line.strip() for line in body.splitlines() if line.strip().startswith(("run:", "python3 ", "bash "))]
        if command not in body:
            findings.append(_finding("CQ-STRUCT-009", ".github/workflows/ci.yml", f"{job}: canonical manager command is missing: {command}", "tooling"))
        quality_commands = [line for line in run_lines if "scripts/check/" in line or "scripts/manager.py fmt" in line or "scripts/manager.py lint" in line]
        if any("scripts/check/" in line for line in quality_commands):
            findings.append(_finding("CQ-STRUCT-009", ".github/workflows/ci.yml", f"{job}: quality policy bypasses manager.py", "tooling"))

    formatter_baseline_path = root / "docs/data/ark-formatter-baseline.toml"
    try:
        formatter_baseline = _load_toml(formatter_baseline_path)
        for field in ("owner", "issue", "removal_condition", "recheck_after"):
            if not formatter_baseline.get(field):
                findings.append(_finding("CQ-STRUCT-009", str(formatter_baseline_path.relative_to(root)), f"formatter baseline missing metadata: {field}", "compiler-tooling"))
        issue = formatter_baseline.get("issue")
        if issue and not any(any((root / state).glob(f"{issue}-*.md")) for state in ("issues/open", "issues/done")):
            findings.append(_finding("CQ-STRUCT-009", str(formatter_baseline_path.relative_to(root)), f"formatter baseline issue does not exist: {issue}", "compiler-tooling"))
        exception_paths: set[str] = set()
        for exception in formatter_baseline.get("exceptions", []):
            path = exception.get("path", "")
            if not path or path in exception_paths:
                findings.append(_finding("CQ-STRUCT-009", str(formatter_baseline_path.relative_to(root)), f"duplicate or empty formatter exception: {path}", "compiler-tooling"))
                continue
            exception_paths.add(path)
            source = root / path
            if not source.is_file():
                findings.append(_finding("CQ-STRUCT-009", path, "formatter exception path does not exist", "compiler-tooling"))
            elif hashlib.sha256(source.read_bytes()).hexdigest() != exception.get("sha256"):
                findings.append(_finding("CQ-STRUCT-009", path, "formatter exception hash is stale", "compiler-tooling"))
    except ValueError as exc:
        findings.append(_finding("CQ-STRUCT-009", "docs/data/ark-formatter-baseline.toml", str(exc), "compiler-tooling"))

    ruleset_path = root / ".github/rulesets/master-quality.json"
    if ruleset_path.is_file():
        try:
            ruleset = json.loads(ruleset_path.read_text(encoding="utf-8"))
            contexts = {
                check.get("context")
                for rule in ruleset.get("rules", [])
                if rule.get("type") == "required_status_checks"
                for check in rule.get("parameters", {}).get("required_status_checks", [])
            }
            for context in ("quality-format", "quality-lint", "verify-quick", "Final gate"):
                if context not in contexts:
                    findings.append(_finding("CQ-STRUCT-009", str(ruleset_path.relative_to(root)), f"required ruleset context is missing: {context}", "tooling"))
        except (OSError, json.JSONDecodeError) as exc:
            findings.append(_finding("CQ-STRUCT-009", str(ruleset_path.relative_to(root)), f"invalid ruleset JSON: {exc}", "tooling"))

    required_checks_path = root / "docs/process/ci-required-checks.md"
    required_checks = required_checks_path.read_text(encoding="utf-8") if required_checks_path.is_file() else ""
    for marker in (
        "python3 scripts/manager.py fmt --check",
        "python3 scripts/manager.py lint",
        "quality-format",
        "quality-lint",
        "verify-quick",
        "Final gate",
    ):
        if marker not in required_checks:
            findings.append(_finding("CQ-STRUCT-009", str(required_checks_path.relative_to(root)), f"required-checks contract is missing: {marker}", "docs"))

    adr_contract_path = root / "docs/adr/ADR-047-code-quality-tooling-and-gates.md"
    adr_contract = adr_contract_path.read_text(encoding="utf-8") if adr_contract_path.is_file() else ""
    for marker in ("quality structure", "quality quick", "quality full", "quality report"):
        if marker not in adr_contract:
            findings.append(_finding("CQ-STRUCT-009", str(adr_contract_path.relative_to(root)), f"ADR-047 command contract is missing: {marker}", "docs"))

    tracked = subprocess.run(
        ["git", "ls-files", "-z"], cwd=root, capture_output=True, check=False,
    )
    if tracked.returncode == 0:
        tracked_exts = {
            Path(raw.decode("utf-8", errors="surrogateescape")).suffix.lower()
            for raw in tracked.stdout.split(b"\0")
            if raw and Path(raw.decode("utf-8", errors="surrogateescape")).suffix
        }
        for ext in sorted(tracked_exts - extensions):
            findings.append(_finding("CQ-STRUCT-009", "docs/data/tooling-inventory.toml", f"tracked file family is missing: {ext}", "tooling"))
    return findings


def _external_check(root: Path, rule_id: str, path: str, owner: str, command: tuple[str, ...]) -> list[Finding]:
    result = subprocess.run(command, cwd=root, capture_output=True, text=True, check=False)
    if result.returncode == 0:
        return []
    output = (result.stdout + result.stderr).strip()
    message = output.splitlines()[-1] if output else f"command failed: {' '.join(command)}"
    return [_finding(rule_id, path, message, owner)]


def collect_structure_report(root: Path, include_external: bool = True) -> StructureReport:
    findings: list[Finding] = []
    for cycle in _compiler_import_cycle_violations(root):
        findings.append(_finding("CQ-STRUCT-002", cycle[0], "import cycle: " + " -> ".join(cycle)))
    for path, line, source in _compiler_dependency_direction_violations(root):
        findings.append(_finding("CQ-STRUCT-003", path, f"reverse pipeline dependency: {source}", line=line))
    for path in _compiler_production_test_reachability_violations(root):
        findings.append(_finding("CQ-STRUCT-004", path, "test-only module is reachable from production entry"))
    for path in _compiler_root_layout_violations(root):
        findings.append(_finding("CQ-STRUCT-005", path, "role-specific implementation bypasses compiler namespace"))
    for message in _compiler_namespace_layout_violations(root):
        findings.append(_finding("CQ-STRUCT-005", "src/compiler", message))
    for path, line, message in _compiler_public_boundary_violations(root):
        findings.append(_finding("CQ-STRUCT-005", path, message, line=line))
    findings.extend(quality_contract_findings(root))

    if include_external:
        checks = (
            ("CQ-STRUCT-006", "scripts/", "tooling", ("bash", "scripts/check/check-repo-structure.sh")),
            ("CQ-STRUCT-007", ".generated-files", "docs", ("bash", "scripts/check/check-generated-files.sh")),
            ("CQ-STRUCT-008", "src/compiler/", "compiler", (sys.executable, "scripts/check/check-compiler-boundaries.py")),
            ("CQ-STRUCT-009", "docs/data/ci-jobs.md", "tooling", (sys.executable, "scripts/gen/generate-ci-jobs-doc.py", "--check")),
            ("CQ-STRUCT-009", "docs/data/project-state.toml", "docs", (sys.executable, "scripts/gen/generate-structured-state-docs.py", "--check")),
            ("CQ-STRUCT-009", "docs/", "docs", (sys.executable, "scripts/gen/generate-docs.py", "--check")),
        )
        for rule_id, path, owner, command in checks:
            findings.extend(_external_check(root, rule_id, path, owner, command))
    return StructureReport(tuple(sorted(set(findings))))


def render_structure_text(report: StructureReport) -> str:
    lines = [
        f"repository structure: {report.status.upper()}",
        f"errors={report.summary['errors']} warnings={report.summary['warnings']} advisories={report.summary['advisories']}",
    ]
    for item in report.findings:
        lines.append(f"{item.severity.upper()} {item.rule_id} {item.path}:{item.line}: {item.message} (owner={item.owner})")
    return "\n".join(lines)


def run_structure(root: Path, json_output: bool = False, include_external: bool = True) -> int:
    report = collect_structure_report(root, include_external=include_external)
    if json_output:
        print(json.dumps(report.to_dict(), ensure_ascii=False, sort_keys=True))
    else:
        print(render_structure_text(report))
    return 1 if report.status == "fail" else 0
