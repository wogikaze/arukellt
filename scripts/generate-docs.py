#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import sys
import tomllib
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
DATA = DOCS / "data"
PROJECT_STATE = DATA / "project-state.toml"
SECTIONS_FILE = DATA / "sections.toml"
STDLIB_MANIFEST = ROOT / "std" / "manifest.toml"
FIXTURE_MANIFEST = ROOT / "tests" / "fixtures" / "manifest.txt"


@dataclass(frozen=True)
class DocEntry:
    rel_path: str
    title: str
    summary: str


def load_toml(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def fixture_count() -> int:
    return sum(
        1
        for line in FIXTURE_MANIFEST.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.strip().startswith("#")
    )


def escape_table(text: str) -> str:
    stripped = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", text)
    stripped = re.sub(r"`([^`]+)`", r"\1", stripped)
    stripped = stripped.replace("**", "").replace("*", "")
    stripped = re.sub(r"\s+", " ", stripped).strip()
    return stripped.replace("|", r"\|")


def extract_doc_entry(path: Path, base_dir: Path) -> DocEntry:
    lines = path.read_text(encoding="utf-8").splitlines()
    title = path.stem
    summary = ""
    in_code = False
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("# ") and title == path.stem:
            title = stripped[2:].strip()
            continue
        if stripped.startswith("```"):
            in_code = not in_code
            continue
        if in_code or not stripped:
            continue
        if stripped.startswith("<!--") or stripped == "---":
            continue
        if stripped.startswith("#") or stripped.startswith("|"):
            continue
        if stripped.startswith("- ") or stripped.startswith("* "):
            continue
        if re.match(r"\d+\.\s", stripped):
            continue
        if stripped.startswith(">"):
            stripped = stripped.lstrip(">").strip()
        summary = escape_table(stripped)
        break
    if not summary:
        summary = "See the document for details."
    return DocEntry(
        rel_path=path.relative_to(base_dir).as_posix(),
        title=title,
        summary=summary,
    )


def collect_markdown_entries(section_dir: Path) -> list[DocEntry]:
    entries: list[DocEntry] = []
    for path in sorted(section_dir.rglob("*.md")):
        if path.name == "README.md":
            continue
        entries.append(extract_doc_entry(path, section_dir))
    return entries


def humanize_slug(value: str) -> str:
    return value.replace("-", " ").replace("_", " ").title()


def collect_examples(state: dict) -> list[dict]:
    baseline_cases = {Path(case).name for case in state["perf"]["baseline_cases"]}
    entries: list[dict] = []
    for path in sorted((DOCS / "examples").glob("*.ark")):
        expected_path = path.with_suffix(".expected")
        entries.append(
            {
                "file": path.name,
                "title": humanize_slug(path.stem),
                "expected": "yes" if expected_path.exists() else "no",
                "baseline": "yes" if path.name in baseline_cases else "no",
                "run": f"`target/release/arukellt run docs/examples/{path.name}`",
            }
        )
    return entries


def collect_sample_files() -> list[str]:
    sample_dir = DOCS / "sample"
    return [path.name for path in sorted(sample_dir.iterdir()) if path.is_file()]


def load_stdlib_manifest() -> dict:
    return load_toml(STDLIB_MANIFEST)


def stdlib_stats(manifest: dict) -> dict:
    types = manifest.get("types", [])
    values = manifest.get("values", [])
    functions = manifest.get("functions", [])
    public_functions = [entry for entry in functions if not entry["name"].startswith("__intrinsic_")]
    prelude_functions = [entry for entry in public_functions if entry.get("prelude")]
    category_counts = Counter(entry.get("doc_category", "misc") for entry in public_functions)
    return {
        "types": types,
        "values": values,
        "functions": functions,
        "public_functions": public_functions,
        "prelude_functions": prelude_functions,
        "category_counts": category_counts,
    }


def join_pipeline(parts: list[str]) -> str:
    return " -> ".join(parts)


def render_target_table(state: dict) -> str:
    rows = [
        "| Target | Tier | Status | Run | Notes |",
        "|--------|------|--------|-----|-------|",
    ]
    for profile in state["target_profiles"]:
        rows.append(
            "| `{}` | {} | {} | {} | {} |".format(
                profile["id"],
                profile["tier"],
                "Implemented" if profile["implemented"] else "Not implemented",
                "Yes" if profile["run_supported"] else "No",
                escape_table(profile["role"]),
            )
        )
    return "\n".join(rows)


def render_current_state_updated(state: dict) -> str:
    return f"> Updated: {state['project']['updated']}."


def render_current_state_targets(state: dict) -> str:
    return "\n".join(
        [
            "## Targets",
            "",
            render_target_table(state),
        ]
    )


def render_current_state_test_health(state: dict, fixture_total: int) -> str:
    verification = state["verification"]
    return "\n".join(
        [
            "## Test Health",
            "",
            f"- Unit tests: {verification['unit_tests_note']}",
            f"- Fixture harness: {fixture_total} passed, {verification['fixture_failures']} failed (manifest-driven)",
            f"- Fixture manifest: {fixture_total} entries",
            "- Wasm validation is a hard error (W0004)",
            f"- Verification entry point: `{state['project']['verification_command']}` — **{verification['checks_passed']}/{verification['checks_total']} checks pass**",
        ]
    )


def render_current_state_perf(state: dict) -> str:
    perf = state["perf"]
    lines = [
        "## Baseline and Perf Gates",
        "",
        "- Baselines are materialized under `tests/baselines/`",
        "- Compile-time baseline cases:",
    ]
    lines.extend(f"  - `{case}`" for case in perf["baseline_cases"])
    lines.extend(
        [
            "- Current thresholds:",
            f"  - `arukellt check`: median compile time regression must stay within {perf['check_budget_pct']}%",
            f"  - `arukellt compile`: median compile time regression must stay within {perf['compile_budget_pct']}%",
            f"- {perf['heavy_note']}",
        ]
    )
    return "\n".join(lines)


def render_current_state_diagnostics(state: dict) -> str:
    lines = [
        "## Diagnostics and Validation",
        "",
        "- Canonical diagnostics registry lives in `crates/ark-diagnostics`",
        "- Diagnostics are tracked by code, severity, and phase origin",
    ]
    for diagnostic in state["diagnostics"]:
        lines.append(
            f"- `{diagnostic['code']}`: {diagnostic['summary']} ({diagnostic['severity']}, `{diagnostic['phase']}`)"
        )
    lines.append("- Structured diagnostic snapshots are available for tests/docs via `ARUKELLT_DUMP_DIAGNOSTICS=1`")
    return "\n".join(lines)


def render_readme_status(state: dict, fixture_total: int, manifest_stats: dict) -> str:
    verification = state["verification"]
    targets = state["targets"]
    return "\n".join(
        [
            "## Status",
            "",
            f"- Updated: {state['project']['updated']}",
            f"- CLI default target: `{targets['cli_default']}`",
            f"- Canonical target: `{targets['canonical']}`",
            f"- Component/WIT target: `{targets['component_target']}`",
            f"- Unit tests: {verification['unit_tests_note']}",
            f"- Fixture harness: {fixture_total} passed / {fixture_total} entries",
            f"- Verification: `{state['project']['verification_command']}` — {verification['checks_passed']}/{verification['checks_total']} checks pass",
            f"- Stdlib manifest-backed public API: {len(manifest_stats['public_functions'])} functions",
        ]
    )


def render_root_docs_readme(sections: list[dict], state: dict, fixture_total: int, manifest_stats: dict) -> str:
    lines = [
        "# Arukellt Documentation",
        "",
        "> This file is generated by `python3 scripts/generate-docs.py`.",
        f"> Source of truth: current behavior is [`current-state.md`](current-state.md); structured state lives in [`data/project-state.toml`](data/project-state.toml) and [`../std/manifest.toml`](../std/manifest.toml).",
        "",
        "## Current Snapshot",
        "",
        f"- Updated: {state['project']['updated']}",
        f"- CLI default target: `{state['targets']['cli_default']}`",
        f"- Canonical target: `{state['targets']['canonical']}`",
        f"- Component emit: {'available' if state['targets']['component_emit'] else 'not available'} on `{state['targets']['component_target']}` ({state['targets']['component_note']})",
        f"- Fixture harness: {fixture_total} passed / {fixture_total} entries",
        f"- Verification: `{state['project']['verification_command']}` — {state['verification']['checks_passed']}/{state['verification']['checks_total']} checks pass",
        f"- Stdlib manifest-backed public API: {len(manifest_stats['public_functions'])} functions",
        "",
        "## First Reads",
        "",
        "- [Current state](current-state.md)",
        "- [Quickstart](quickstart.md)",
        "- [コンパイラ](compiler/README.md)",
        "- [標準ライブラリ](stdlib/README.md)",
        "- [Contributing](contributing.md)",
    ]
    category_labels = {
        "current": "Current Docs",
        "supporting": "Supporting Docs",
        "archive": "Archive / History",
    }
    for category in ("current", "supporting", "archive"):
        category_sections = [section for section in sections if section["category"] == category]
        if not category_sections:
            continue
        lines.extend(["", f"## {category_labels[category]}", "", "| Section | Entry | Notes |", "|---------|-------|-------|"])
        for section in category_sections:
            lines.append(
                "| {} | [{}]({}/README.md) | {} |".format(
                    section["title"],
                    section["dir"],
                    section["dir"],
                    escape_table(section["description"]),
                )
            )
    return "\n".join(lines) + "\n"


def render_sidebar(sections: list[dict]) -> str:
    category_labels = {
        "current": "Current Docs",
        "supporting": "Supporting Docs",
        "archive": "Archive / History",
    }
    lines = [
        "- **Arukellt**",
        "  - [ホーム](/)",
        "  - [Docs Overview](README.md)",
        "  - [Current state](current-state.md)",
        "  - [クイックスタート](quickstart.md)",
        "  - [Contributing](contributing.md)",
    ]
    for category in ("current", "supporting", "archive"):
        category_sections = [section for section in sections if section["category"] == category]
        if not category_sections:
            continue
        lines.extend(["", f"- **{category_labels[category]}**"])
        for section in category_sections:
            lines.append(f"  - [{section['title']}]({section['dir']}/README.md)")
    return "\n".join(lines) + "\n"


def render_generic_section_readme(section: dict, entries: list[DocEntry], snapshot_lines: list[str]) -> str:
    lines = [
        f"# {section['title']}",
        "",
        "> This file is generated by `python3 scripts/generate-docs.py`.",
        section["description"],
        "",
        "## Current Snapshot",
        "",
    ]
    lines.extend(snapshot_lines or ["- Current source of truth: [../current-state.md](../current-state.md)"])
    lines.extend(["", "## Documents", "", "| File | Title | Summary |", "|------|-------|---------|"])
    for entry in entries:
        lines.append(f"| [{entry.rel_path}]({entry.rel_path}) | {escape_table(entry.title)} | {entry.summary} |")
    return "\n".join(lines) + "\n"


def render_stdlib_readme(section: dict, entries: list[DocEntry], state: dict, manifest_stats: dict) -> str:
    types = ", ".join(f"`{entry['name']}`" for entry in manifest_stats["types"] if entry.get("prelude"))
    values = ", ".join(f"`{entry['name']}`" for entry in manifest_stats["values"] if entry.get("prelude"))
    category_summary = ", ".join(
        f"`{name}` {count}" for name, count in sorted(manifest_stats["category_counts"].items())
    )
    lines = [
        f"# {section['title']}",
        "",
        "> This file is generated by `python3 scripts/generate-docs.py`.",
        section["description"],
        "",
        "## Current Snapshot",
        "",
        "- Current source of truth: [../current-state.md](../current-state.md), [`../../std/manifest.toml`](../../std/manifest.toml), and [`reference.md`](reference.md)",
        f"- Manifest-backed public functions: {len(manifest_stats['public_functions'])}",
        f"- Prelude wrappers: {len(manifest_stats['prelude_functions'])}",
        f"- Prelude types: {types}",
        f"- Prelude values: {values}",
        f"- Categories: {category_summary}",
        "",
        "## Recommended Reads",
        "",
        "- [reference.md](reference.md)",
        "- [std.md](std.md)",
        "- [cookbook.md](cookbook.md)",
        "",
        "## Documents",
        "",
        "| File | Title | Summary |",
        "|------|-------|---------|",
    ]
    for entry in entries:
        lines.append(f"| [{entry.rel_path}]({entry.rel_path}) | {escape_table(entry.title)} | {entry.summary} |")
    return "\n".join(lines) + "\n"


def render_examples_readme(section: dict, examples: list[dict], state: dict) -> str:
    baseline_cases = {Path(case).name for case in state["perf"]["baseline_cases"]}
    lines = [
        f"# {section['title']}",
        "",
        "> This file is generated by `python3 scripts/generate-docs.py`.",
        section["description"],
        "",
        "## Current Snapshot",
        "",
        f"- Executable examples: {len(examples)}",
        f"- `.expected` pairs present: {sum(1 for entry in examples if entry['expected'] == 'yes')}",
        f"- Baseline-tracked examples: {sum(1 for entry in examples if entry['file'] in baseline_cases)}",
        "- These files serve as both documentation and runnable integration examples.",
        "",
        "## Run",
        "",
        "```bash",
        "target/release/arukellt run docs/examples/hello.ark",
        "```",
        "",
        "## Examples",
        "",
        "| File | Description | Expected | Baseline |",
        "|------|-------------|----------|----------|",
    ]
    for entry in examples:
        lines.append(
            f"| [{entry['file']}]({entry['file']}) | {entry['title']} | {entry['expected']} | {entry['baseline']} |"
        )
    return "\n".join(lines) + "\n"


def render_sample_readme(section: dict, files: list[str]) -> str:
    lines = [
        f"# {section['title']}",
        "",
        "> This file is generated by `python3 scripts/generate-docs.py`.",
        section["description"],
        "",
        "## Current Snapshot",
        "",
        "- Current source of truth for runnable behavior remains [../current-state.md](../current-state.md).",
        "- This directory contains implementation-oriented sample files rather than narrative docs.",
        "",
        "## Files",
        "",
        "| File | Notes |",
        "|------|-------|",
    ]
    for filename in files:
        lines.append(f"| [{filename}]({filename}) | Sample artifact |")
    return "\n".join(lines) + "\n"


def render_archive_snapshot() -> list[str]:
    return [
        "- These documents are historical or design references, not the current behavior contract.",
        "- Current source of truth: [../current-state.md](../current-state.md).",
    ]


def section_snapshot(section: dict, state: dict, fixture_total: int, manifest_stats: dict, examples: list[dict]) -> list[str]:
    snapshot = section["snapshot"]
    if snapshot == "compiler":
        return [
            f"- Current path: `{join_pipeline(state['pipeline']['current'])}`",
            f"- Refactor target: `{join_pipeline(state['pipeline']['refactor_target'])}`",
            f"- Shared orchestration entry point: `{state['pipeline']['session_entry']}`",
            f"- Dump phases: `{', '.join(state['pipeline']['dump_phases'])}`",
        ]
    if snapshot == "language":
        return [
            "- Current user-visible behavior is described by [../current-state.md](../current-state.md).",
            f"- Fixture-backed verification covers {fixture_total} manifest entries.",
            f"- Canonical target for current docs: `{state['targets']['canonical']}`",
        ]
    if snapshot == "platform":
        return [
            f"- CLI default target: `{state['targets']['cli_default']}`",
            f"- Canonical target: `{state['targets']['canonical']}`",
            f"- Component emit: {'available' if state['targets']['component_emit'] else 'not available'} on `{state['targets']['component_target']}`",
            "- Backend validation failure (`W0004`) is a hard error.",
        ]
    if snapshot == "process":
        return [
            f"- Verification command: `{state['project']['verification_command']}`",
            f"- Current verification gate: {state['verification']['checks_passed']}/{state['verification']['checks_total']} checks pass",
            f"- Fixture manifest size: {fixture_total} entries",
            "- Generated docs pull state from `docs/data/project-state.toml`, `std/manifest.toml`, and fixture manifests.",
        ]
    if snapshot == "stdlib":
        return [
            f"- Manifest-backed public functions: {len(manifest_stats['public_functions'])}",
            f"- Prelude wrappers: {len(manifest_stats['prelude_functions'])}",
            f"- Prelude types: {', '.join(entry['name'] for entry in manifest_stats['types'] if entry.get('prelude'))}",
            f"- Prelude values: {', '.join(entry['name'] for entry in manifest_stats['values'] if entry.get('prelude'))}",
        ]
    if snapshot == "examples":
        return [
            f"- Executable examples: {len(examples)}",
            f"- `.expected` coverage: {sum(1 for entry in examples if entry['expected'] == 'yes')}/{len(examples)}",
            "- Baseline-tracked examples are shared with perf gates.",
        ]
    if snapshot == "migration":
        return [
            f"- CLI default target remains `{state['targets']['cli_default']}`.",
            f"- Canonical path for current docs is `{state['targets']['canonical']}`.",
            f"- Component emit lives on `{state['targets']['component_target']}`.",
        ]
    if snapshot == "sample":
        return [
            "- This directory is intentionally code-first.",
            "- Use the sample files as reference artifacts, not as the current behavior contract.",
        ]
    if snapshot == "archive":
        return render_archive_snapshot()
    return ["- Current source of truth: [../current-state.md](../current-state.md)"]


def format_signature(params: list[str], returns: str) -> str:
    joined = ", ".join(params)
    return f"({joined}) -> {returns}" if params else f"() -> {returns}"


def render_stdlib_reference(manifest: dict) -> str:
    types = manifest.get("types", [])
    values = manifest.get("values", [])
    functions = [entry for entry in manifest.get("functions", []) if not entry["name"].startswith("__intrinsic_")]
    grouped: dict[str, list[dict]] = defaultdict(list)
    for entry in functions:
        grouped[entry.get("doc_category", "misc")].append(entry)

    lines = [
        "# stdlib API Reference",
        "",
        "> This file is generated by `python3 scripts/generate-docs.py` from [`../../std/manifest.toml`](../../std/manifest.toml).",
        "> It reflects the current implemented public API, not roadmap-only or archived design notes.",
        "",
        "## Prelude Types",
        "",
        "| Name | Generic Params | Prelude |",
        "|------|----------------|---------|",
    ]
    for entry in types:
        generic = ", ".join(entry.get("generic_params", [])) or "-"
        lines.append(
            f"| `{entry['name']}` | {generic} | {'yes' if entry.get('prelude') else 'no'} |"
        )

    lines.extend(["", "## Prelude Values", "", "| Name | Prelude |", "|------|---------|"])
    for entry in values:
        lines.append(f"| `{entry['name']}` | {'yes' if entry.get('prelude') else 'no'} |")

    for category in sorted(grouped):
        lines.extend(
            [
                "",
                f"## {humanize_slug(category)}",
                "",
                "| Name | Signature | Kind | Prelude | Intrinsic |",
                "|------|-----------|------|---------|-----------|",
            ]
        )
        for entry in sorted(grouped[category], key=lambda item: item["name"]):
            intrinsic = f"`{entry['intrinsic']}`" if entry.get("intrinsic") else "-"
            lines.append(
                "| `{name}` | `{signature}` | `{kind}` | {prelude} | {intrinsic} |".format(
                    name=entry["name"],
                    signature=format_signature(entry.get("params", []), entry.get("returns", "()")),
                    kind=entry.get("kind", "builtin"),
                    prelude="yes" if entry.get("prelude") else "no",
                    intrinsic=intrinsic,
                )
            )

    return "\n".join(lines) + "\n"


def replace_generated_block(text: str, marker: str, content: str) -> str:
    start = f"<!-- BEGIN GENERATED:{marker} -->"
    end = f"<!-- END GENERATED:{marker} -->"
    pattern = re.compile(re.escape(start) + r".*?" + re.escape(end), re.DOTALL)
    replacement = f"{start}\n{content.rstrip()}\n{end}"
    if not pattern.search(text):
        raise ValueError(f"missing marker {marker}")
    return pattern.sub(replacement, text, count=1)


def ensure_parent(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def write_file(path: Path, desired: str, check: bool, stale: list[Path]) -> None:
    ensure_parent(path)
    normalized = desired.rstrip() + "\n"
    current = path.read_text(encoding="utf-8") if path.exists() else None
    if current == normalized:
        return
    if check:
        stale.append(path)
        return
    path.write_text(normalized, encoding="utf-8")


def apply_marker_updates(path: Path, replacements: dict[str, str], check: bool, stale: list[Path]) -> None:
    text = path.read_text(encoding="utf-8")
    updated = text
    for marker, content in replacements.items():
        updated = replace_generated_block(updated, marker, content)
    write_file(path, updated, check, stale)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="Fail if generated docs are out of date.")
    args = parser.parse_args()

    state = load_toml(PROJECT_STATE)
    sections = load_toml(SECTIONS_FILE)["sections"]
    manifest = load_stdlib_manifest()
    manifest_stats = stdlib_stats(manifest)
    examples = collect_examples(state)
    fixture_total = fixture_count()
    stale: list[Path] = []

    apply_marker_updates(
        ROOT / "README.md",
        {
            "README_STATUS": render_readme_status(state, fixture_total, manifest_stats),
        },
        args.check,
        stale,
    )
    apply_marker_updates(
        DOCS / "current-state.md",
        {
            "CURRENT_STATE_UPDATED": render_current_state_updated(state),
            "CURRENT_STATE_TARGETS": render_current_state_targets(state),
            "CURRENT_STATE_TEST_HEALTH": render_current_state_test_health(state, fixture_total),
            "CURRENT_STATE_PERF": render_current_state_perf(state),
            "CURRENT_STATE_DIAGNOSTICS": render_current_state_diagnostics(state),
        },
        args.check,
        stale,
    )

    write_file(DOCS / "README.md", render_root_docs_readme(sections, state, fixture_total, manifest_stats), args.check, stale)
    write_file(DOCS / "_sidebar.md", render_sidebar(sections), args.check, stale)
    write_file(DOCS / "stdlib" / "reference.md", render_stdlib_reference(manifest), args.check, stale)

    for section in sections:
        section_dir = DOCS / section["dir"]
        entries = collect_markdown_entries(section_dir)
        if section["dir"] == "stdlib":
            content = render_stdlib_readme(section, entries, state, manifest_stats)
        elif section["dir"] == "examples":
            content = render_examples_readme(section, examples, state)
        elif section["dir"] == "sample":
            content = render_sample_readme(section, collect_sample_files())
        else:
            content = render_generic_section_readme(
                section,
                entries,
                section_snapshot(section, state, fixture_total, manifest_stats, examples),
            )
        write_file(section_dir / "README.md", content, args.check, stale)

    if stale:
        for path in stale:
            print(path.relative_to(ROOT), file=sys.stderr)
        print("generated docs are out of date; run `python3 scripts/generate-docs.py`", file=sys.stderr)
        return 1

    print("generated docs are up to date")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
