#!/usr/bin/env python3
"""Generate Markdown views from Phase-2 structured state TOML files (#770)."""

from __future__ import annotations

import sys
from pathlib import Path

try:
    import tomllib
except ImportError:
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "docs" / "data"


def load(name: str) -> dict:
    return tomllib.loads((DATA / name).read_text(encoding="utf-8"))


def yn(v) -> str:
    if v is True:
        return "yes"
    if v is False:
        return "no"
    return str(v)


def write(path: Path, content: str, check: bool, stale: list[Path]) -> None:
    if check:
        if not path.is_file() or path.read_text(encoding="utf-8") != content:
            stale.append(path)
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def render_capabilities(data: dict) -> str:
    lines = [
        "# Arukellt Capability Surface",
        "",
        "> **Generated** from `docs/data/capabilities.toml` by `scripts/gen/generate-structured-state-docs.py`.",
        "> Do not hand-edit the matrix. Edit the TOML instead.",
        ">",
        "> **Do not treat “registered” or “compiles” as “user-reachable”.**",
        "",
        "## Status axes",
        "",
        "| Axis | Meaning |",
        "|------|---------|",
        "| `declared` | Named in ADR-011 / design surface |",
        "| `registered` | Present in `std/manifest.toml` |",
        "| `compiles` | Selfhost compile path accepts the module for at least one target |",
        "| `links` | Emitted Wasm links against required host imports |",
        "| `runs` | At least one runtime fixture exercises the module end-to-end |",
        "| `user_reachable` | End users can import and call it on a supported public path |",
        "| `grant_required` | Runtime capability grant / deny flags needed |",
        "| `verified_on` | Targets / host profiles with evidence |",
        "",
        "## Module matrix",
        "",
        "| Module | Path | declared | registered | compiles | links | runs | user_reachable | grant_required | verified_on | Notes |",
        "|--------|------|:--------:|:----------:|:--------:|:-----:|:----:|:--------------:|:--------------:|-------------|-------|",
    ]
    for m in data.get("modules", []):
        verified = ", ".join(f"`{t}`" for t in m.get("verified_on", [])) or "—"
        ur = yn(m.get("user_reachable"))
        if m.get("user_reachable") is False:
            ur = "**no**"
        lines.append(
            "| `{module}` | `{path}` | {d} | {r} | {c} | {l} | {runs} | {ur} | {g} | {v} | {notes} |".format(
                module=m["module"],
                path=m["path"],
                d=yn(m.get("declared")),
                r=yn(m.get("registered")),
                c=yn(m.get("compiles")),
                l=yn(m.get("links")),
                runs=yn(m.get("runs")),
                ur=ur,
                g=m.get("grant_required", ""),
                v=verified,
                notes=m.get("notes", ""),
            )
        )
    lines.extend(
        [
            "",
            "## Deny enforcement (structured)",
            "",
            "| Module | Flag | Current enforcement | Intended | Transitive | Applies to |",
            "|--------|------|---------------------|----------|:----------:|------------|",
        ]
    )
    for m in data.get("modules", []):
        if not m.get("deny_flag"):
            continue
        intended = m.get("deny_intended_enforcement") or m.get("deny_enforcement") or "—"
        lines.append(
            f"| `{m['module']}` | `{m['deny_flag']}` | `{m.get('deny_enforcement')}` | "
            f"`{intended}` | {yn(m.get('deny_transitive'))} | `{m.get('deny_applies_to')}` |"
        )
    lines.extend(
        [
            "",
            "## Runtime verification / evidence (not a reachability claim)",
            "",
            "1. **`wasm32` / `wasm32-gc` fixtures** — runnable programs under `tests/fixtures/` for modules marked `runs=yes`.",
            "2. **`wasm32-gc` WASM validation** — `scripts/check/check-t3-wasm-validate.py` (historical script name).",
            "3. **Selfhost fixpoint** — compiler uses `stdio` / `fs` under real workloads.",
            "4. **Gate-136** — `scripts/check/gate-136-std-host-rollout.py` checks ADR-011 module presence/docs.",
            "",
            "Further user-reachability work: issue #675.",
            "",
            "## See also",
            "",
            "- [`docs/data/capabilities.toml`](data/capabilities.toml) — SSOT",
            "- [`docs/current-state.md`](current-state.md)",
            "- [`docs/platform/target-runtime-and-surfaces.md`](platform/target-runtime-and-surfaces.md)",
            "",
        ]
    )
    return "\n".join(lines)


def render_component_availability(data: dict) -> str:
    meta = data.get("meta", {})
    arts = data.get("artifacts", {})
    lines = [
        "# Component availability (structured)",
        "",
        "> **Generated** from `docs/data/component-availability.toml`.",
        "> Do not flatten to a single `available: true/false`.",
        "",
        f"- Target: `{meta.get('target', 'wasm32-gc')}`",
        f"- Public contract: {meta.get('public_contract', '')}",
        f"- Implementation: {meta.get('implementation_note', '')}",
        "",
        "## Active compiler artifacts",
        "",
        f"| Role | Path |",
        f"|------|------|",
        f"| Pinned bootstrap | `{arts.get('pinned_bootstrap')}` |",
        f"| Recommended for library exports | `{arts.get('recommended_library_compiler')}` |",
        f"| Env override | `{arts.get('env_override')}` |",
        "",
        "## Surfaces",
        "",
        "| ID | Label | Status | Active compiler | External tools | Notes |",
        "|----|-------|--------|-----------------|----------------|-------|",
    ]
    for s in data.get("surfaces", []):
        lines.append(
            f"| `{s['id']}` | {s['label']} | `{s['status']}` | `{s['active_compiler']}` | "
            f"{s.get('external_tool_dependency', '')} | {s.get('notes', '')} |"
        )
    lines.append("")
    return "\n".join(lines)


def render_cli_surface(data: dict) -> str:
    binary = data.get("binary", {})
    lines = [
        "# CLI surface (structured)",
        "",
        "> **Generated** from `docs/data/cli-surface.toml`.",
        f"> Binary: `{binary.get('name')}` — alias policy: `{binary.get('alias_policy')}`.",
        f"> Wrapper: `{binary.get('wrapper')}`. Usage source: `{binary.get('usage_source')}`.",
        "",
        "| Command | Status | Guarantee tier | Summary |",
        "|---------|--------|----------------|---------|",
    ]
    for c in data.get("commands", []):
        lines.append(
            f"| `arukellt {c['id']}` | `{c['status']}` | `{c['guarantee_tier']}` | {c['summary']} |"
        )
    lines.extend(["", "Human guide: [`../cli-reference.md`](../cli-reference.md).", ""])
    return "\n".join(lines)


def render_bootstrap_contract(data: dict) -> str:
    trust = data.get("trust", {})
    lines = [
        "# Bootstrap contract (structured)",
        "",
        "> **Generated** from `docs/data/bootstrap-contract.toml` (ADR-029).",
        "",
        f"- Trust base: `{trust.get('base')}` → `{trust.get('artifact')}`",
        f"- Rust Stage 0: `{trust.get('rust_stage0')}`",
        f"- Entrypoint: `{trust.get('entrypoint')}`",
        f"- ADR: `{trust.get('adr')}`",
        "",
    ]
    order = trust.get("wasm_resolution_order") or []
    if order:
        lines.append("- Wasm resolution order:")
        for i, cand in enumerate(order, 1):
            lines.append(f"  {i}. `{cand}`")
        lines.append("")
    lines.extend(
        [
            "## Stages",
            "",
            "| ID | Name | Description | Artifact | Comparison |",
            "|----|------|-------------|----------|------------|",
        ]
    )
    for s in data.get("stages", []):
        lines.append(
            f"| `{s['id']}` | `{s['name']}` | {s['description']} | `{s.get('artifact', '')}` | `{s.get('comparison', '')}` |"
        )
    lines.extend(
        [
            "",
            "## Gates",
            "",
            "| ID | Command | CI job |",
            "|----|---------|--------|",
        ]
    )
    for g in data.get("gates", []):
        lines.append(f"| `{g['id']}` | `{g['command']}` | `{g['ci_job']}` |")
    lines.extend(
        [
            "",
            "## Retired",
            "",
            "| ID | Path | Reason | Archive |",
            "|----|------|--------|---------|",
        ]
    )
    for r in data.get("retired", []):
        lines.append(
            f"| `{r['id']}` | `{r['path']}` | {r['reason']} | `{r.get('archive', '')}` |"
        )
    lines.append("")
    return "\n".join(lines)


def render_release_guarantees(data: dict) -> str:
    lines = [
        "# Release guarantees (structured)",
        "",
        "> **Generated** from `docs/data/release-guarantees.toml`.",
        "> Normative prose: [`../release-criteria.md`](../release-criteria.md). "
        "Checklist: [`../release-checklist.md`](../release-checklist.md).",
        "",
        "| ID | Tier | Summary | Check | CI job | Blocker | Known limitation |",
        "|----|------|---------|-------|--------|:-------:|------------------|",
    ]
    for g in data.get("guarantees", []):
        lines.append(
            "| `{id}` | `{tier}` | {summary} | `{check}` | `{job}` | {blocker} | {lim} |".format(
                id=g["id"],
                tier=g["tier"],
                summary=g["summary"],
                check=g.get("check", ""),
                job=g.get("ci_job", ""),
                blocker="yes" if g.get("release_blocker") else "no",
                lim=g.get("known_limitation", "") or "—",
            )
        )
    lines.append("")
    return "\n".join(lines)


def render_release_blockers(data: dict) -> str:
    lines = ["<!-- Generated from docs/data/release-guarantees.toml; do not edit this block. -->"]
    for guarantee in data.get("guarantees", []):
        if not guarantee.get("release_blocker"):
            continue
        lines.append(
            f"- [ ] **CI `{guarantee['id']}`** — `{guarantee['check']}` "
            f"(job: `{guarantee['ci_job']}`)"
        )
    return "\n".join(lines)


def replace_block(path: Path, marker: str, content: str, check: bool, stale: list[Path]) -> None:
    text = path.read_text(encoding="utf-8")
    start = f"<!-- BEGIN GENERATED:{marker} -->"
    end = f"<!-- END GENERATED:{marker} -->"
    pattern = __import__("re").compile(__import__("re").escape(start) + r".*?" + __import__("re").escape(end), __import__("re").DOTALL)
    replacement = f"{start}\n{content}\n{end}"
    updated = pattern.sub(replacement, text, count=1)
    if updated == text and replacement not in text:
        raise ValueError(f"missing generated block {marker} in {path}")
    write(path, updated, check, stale)


def main() -> int:
    check = "--check" in sys.argv
    stale: list[Path] = []
    mapping = [
        ("capabilities.toml", ROOT / "docs" / "capability-surface.md", render_capabilities),
        ("component-availability.toml", DATA / "component-availability.md", render_component_availability),
        ("cli-surface.toml", DATA / "cli-surface.md", render_cli_surface),
        ("bootstrap-contract.toml", DATA / "bootstrap-contract.md", render_bootstrap_contract),
        ("release-guarantees.toml", DATA / "release-guarantees.md", render_release_guarantees),
    ]
    for toml_name, out, render in mapping:
        data = load(toml_name)
        write(out, render(data), check, stale)
    release_data = load("release-guarantees.toml")
    replace_block(
        ROOT / "docs" / "release-checklist.md",
        "release-blockers",
        render_release_blockers(release_data),
        check,
        stale,
    )
    if check:
        if stale:
            for p in stale:
                print(p.relative_to(ROOT), file=sys.stderr)
            print("structured state docs stale; run scripts/gen/generate-structured-state-docs.py", file=sys.stderr)
            return 1
        print("structured state docs up to date")
        return 0
    print(f"wrote {len(mapping)} structured state docs")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
