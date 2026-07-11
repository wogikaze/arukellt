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

# Valid enum values — generators must reject unknown values.
CLI_PRESENCE_VALUES = ("present", "absent")
CLI_CONTRACT_STABILITY_VALUES = ("stable", "provisional", "experimental")
CLI_IMPLEMENTATION_STATE_VALUES = ("functional", "limited", "scaffold", "unavailable", "unknown")
CHECK_CURRENT_STATUS_VALUES = ("pass", "fail", "stale", "not-run")
CHECK_EVIDENCE_TYPE_VALUES = ("smoke", "static-scan", "fixture-set", "exhaustive", "manual")
GUARANTEE_TIER_VALUES = ("guaranteed", "provisional", "experimental", "not_guaranteed")


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
        "> Axes (do not overload a single `status` field):",
        "> - **Presence**: `present` | `absent` — whether the subcommand exists in the binary",
        "> - **Contract stability**: `stable` | `provisional` | `experimental` — CLI contract maturity",
        "> - **Implementation**: `functional` | `limited` | `scaffold` | `unavailable` | `unknown` — runtime behavior",
        "",
        "| Command | Presence | Contract stability | Implementation | Guarantee IDs | Summary |",
        "|---------|----------|--------------------|----------------|---------------|---------|",
    ]
    for c in data.get("commands", []):
        presence = c.get("presence", "present")
        contract = c.get("contract_stability", "unknown")
        impl = c.get("implementation_state", "unknown")
        lines.append(
            f"| `arukellt {c['id']}` | `{presence}` | `{contract}` | "
            f"`{impl}` | "
            f"{', '.join(f'`{gid}`' for gid in c.get('guarantee_ids', [])) or '—'} | {c['summary']} |"
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


def _status_badge(status: str) -> str:
    if status == "pass":
        return "✅ pass"
    if status == "fail":
        return "❌ fail"
    if status == "stale":
        return "⏰ stale"
    if status == "not-run":
        return "⬜ not-run"
    return status


def render_release_guarantees(data: dict) -> str:
    guarantees = data.get("guarantees", [])
    checks = data.get("checks", [])
    checks_by_guarantee: dict[str, list[dict]] = {}
    for ch in checks:
        gid = ch.get("guarantee_id", "")
        checks_by_guarantee.setdefault(gid, []).append(ch)

    lines = [
        "# Release guarantees (structured)",
        "",
        "> **Generated** from `docs/data/release-guarantees.toml`.",
        "> Normative prose: [`../release-criteria.md`](../release-criteria.md). "
        "Checklist: [`../release-checklist.md`](../release-checklist.md).",
        "",
        "> **Contract vs current state:** A guarantee is a release-time contract.",
        "> `current_status` shows the latest observed verification result, which may be `fail`.",
        "> A `fail` status means the guarantee is not yet met — it does not remove the guarantee.",
        "",
        "## Guarantee matrix",
        "",
        "| ID | Tier | Summary | Evidence scope | Current status | Evidence type | Last verified | Known limitation |",
        "|----|------|---------|----------------|----------------|---------------|---------------|------------------|",
    ]
    for g in guarantees:
        g_checks = checks_by_guarantee.get(g["id"], [])
        if g_checks:
            # Aggregate current status: fail dominates stale dominates pass
            statuses = [ch.get("current_status", "not-run") for ch in g_checks]
            if "fail" in statuses:
                current = "fail"
            elif "stale" in statuses:
                current = "stale"
            elif "not-run" in statuses:
                current = "not-run"
            elif "pass" in statuses:
                current = "pass"
            else:
                current = "not-run"
            evidence_types = sorted({ch.get("evidence_type", "manual") for ch in g_checks})
            evidence = ", ".join(evidence_types)
            last_commits = sorted({ch.get("last_verified_commit", "") for ch in g_checks if ch.get("last_verified_commit")})
            last_verified = ", ".join(f"`{c}`" for c in last_commits) if last_commits else "—"
        else:
            current = "not-run"
            evidence = "—"
            last_verified = "—"
        lines.append(
            "| `{id}` | `{tier}` | {summary} | {coverage} | {current} | {evidence} | {last_verified} | {lim} |".format(
                id=g["id"],
                tier=g["tier"],
                summary=g["summary"],
                coverage=", ".join(f"`{item}`" for item in g.get("coverage", [])) or "—",
                current=_status_badge(current),
                evidence=evidence,
                last_verified=last_verified,
                lim=g.get("known_limitation", "") or "—",
            )
        )

    lines.extend([
        "",
        "## Check catalogue",
        "",
        "The release-blocker set is exactly the checks with `release_blocking = true` below.",
        "No supplemental lists.",
        "",
        "**Checks vs incidents:** A check is an executable verification command.",
        "An incident is a distinct failure event. Multiple checks may track the",
        "same incident (linked via `incident_id`). Count blockers by distinct",
        "incidents, not by individual checks.",
        "",
        "| Check ID | Guarantee | Blocking | In full | In quick | Current | Evidence | Affected | Incident | Last verified | Command |",
        "|----------|-----------|:--------:|:-------:|:--------:|---------|----------|---------:|----------|---------------|---------|",
    ])
    for ch in checks:
        blocking = "🔴 yes" if ch.get("release_blocking") else "no"
        in_full = "✓" if ch.get("included_in_full") else "—"
        in_quick = "✓" if ch.get("included_in_quick") else "—"
        affected = str(ch.get("affected_count", "—"))
        incident = f"`{ch['incident_id']}`" if ch.get("incident_id") else "—"
        lines.append(
            "| `{id}` | {gid} | {blocking} | {in_full} | {in_quick} | {current} | `{evidence}` | {affected} | {incident} | `{lvc}` | `{cmd}` |".format(
                id=ch["id"],
                gid=f"`{ch['guarantee_id']}`" if ch.get("guarantee_id") else "—",
                blocking=blocking,
                in_full=in_full,
                in_quick=in_quick,
                current=_status_badge(ch.get("current_status", "not-run")),
                evidence=ch.get("evidence_type", "manual"),
                affected=affected,
                incident=incident,
                lvc=ch.get("last_verified_commit", "") or "—",
                cmd=ch.get("command", ""),
            )
        )
    lines.append("")
    return "\n".join(lines)


def render_release_blockers(data: dict) -> str:
    """Generate the release-checklist.md blocker block from checks with release_blocking=true."""
    lines = ["<!-- Generated from docs/data/release-guarantees.toml; do not edit this block. -->"]
    for ch in data.get("checks", []):
        if not ch.get("release_blocking"):
            continue
        status_tag = f" [FAIL]" if ch.get("current_status") == "fail" else ""
        lines.append(
            f"- [ ] **CI `{ch['id']}`**{status_tag} — `{ch.get('command', '')}` "
            f"(job: `{ch.get('ci_job', 'none')}`)"
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


def validate_cli_surface(cli_data: dict) -> list[str]:
    errors: list[str] = []
    for c in cli_data.get("commands", []):
        cid = c.get("id", "<unknown>")
        presence = c.get("presence")
        if presence is not None and presence not in CLI_PRESENCE_VALUES:
            errors.append(f"{cid}: invalid presence '{presence}'; must be one of {list(CLI_PRESENCE_VALUES)}")
        contract = c.get("contract_stability")
        if contract is not None and contract not in CLI_CONTRACT_STABILITY_VALUES:
            errors.append(f"{cid}: invalid contract_stability '{contract}'; must be one of {list(CLI_CONTRACT_STABILITY_VALUES)}")
        impl = c.get("implementation_state")
        if impl is not None and impl not in CLI_IMPLEMENTATION_STATE_VALUES:
            errors.append(f"{cid}: invalid implementation_state '{impl}'; must be one of {list(CLI_IMPLEMENTATION_STATE_VALUES)}")
        # Reject legacy fields
        if "status" in c:
            errors.append(f"{cid}: obsolete 'status' field — use presence + contract_stability")
        if "presence_stability" in c:
            errors.append(f"{cid}: obsolete 'presence_stability' field — use contract_stability")
    return errors


def validate_release_guarantees(release_data: dict) -> list[str]:
    errors: list[str] = []
    guarantee_ids = {g["id"] for g in release_data.get("guarantees", [])}
    for g in release_data.get("guarantees", []):
        if g.get("tier") not in GUARANTEE_TIER_VALUES:
            errors.append(f"guarantee {g['id']}: invalid tier '{g.get('tier')}'")
        if g.get("tier") == "guaranteed" and not g.get("coverage"):
            errors.append(f"guarantee {g['id']}: guaranteed row lacks evidence coverage")
    for ch in release_data.get("checks", []):
        cid = ch.get("id", "<unknown>")
        gid = ch.get("guarantee_id", "")
        if gid and gid not in guarantee_ids:
            errors.append(f"check {cid}: unknown guarantee_id '{gid}'")
        status = ch.get("current_status")
        if status is not None and status not in CHECK_CURRENT_STATUS_VALUES:
            errors.append(f"check {cid}: invalid current_status '{status}'; must be one of {list(CHECK_CURRENT_STATUS_VALUES)}")
        etype = ch.get("evidence_type")
        if etype is not None and etype not in CHECK_EVIDENCE_TYPE_VALUES:
            errors.append(f"check {cid}: invalid evidence_type '{etype}'; must be one of {list(CHECK_EVIDENCE_TYPE_VALUES)}")
        if ch.get("release_blocking") and (not ch.get("command") or ch.get("ci_job") == "none"):
            errors.append(f"check {cid}: release-blocking check lacks executable command/CI job")
    return errors


def main() -> int:
    check = "--check" in sys.argv
    stale: list[Path] = []
    cli_data = load("cli-surface.toml")
    release_data = load("release-guarantees.toml")
    guarantee_ids = {g["id"] for g in release_data.get("guarantees", [])}

    contract_errors: list[str] = []
    # CLI surface validation
    for command in cli_data.get("commands", []):
        if "guarantee_tier" in command:
            contract_errors.append(f"{command['id']}: obsolete guarantee_tier field")
        if "status" in command:
            contract_errors.append(f"{command['id']}: obsolete 'status' field — use presence + contract_stability")
        if "presence_stability" in command:
            contract_errors.append(f"{command['id']}: obsolete 'presence_stability' field — use contract_stability")
        for guarantee_id in command.get("guarantee_ids", []):
            if guarantee_id not in guarantee_ids:
                contract_errors.append(f"{command['id']}: unknown guarantee id {guarantee_id}")
    contract_errors.extend(validate_cli_surface(cli_data))
    contract_errors.extend(validate_release_guarantees(release_data))

    if contract_errors:
        for error in contract_errors:
            print(f"structured contract error: {error}", file=sys.stderr)
        return 1

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
