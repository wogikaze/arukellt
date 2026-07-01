# Docs-to-Runtime Contract Audit — 2026-06-17

Audit orchestrator: issue **#679** (docs-audit umbrella)  
Audit scope: README generated status, `docs/current-state.md`, `docs/target-contract.md`, `docs/capability-surface.md`, stdlib manifest-backed docs, legacy doc links, false-done hygiene vs landing claims.  
**Status: complete (2026-06-17).**  
Verification gate at audit close: `python3 scripts/manager.py verify quick` — **166/166** checks pass.

## Executive summary

| Verdict | Count | Notes |
|---------|------:|-------|
| **OK** | 3 | README/current-state SoT sync (#679); legacy README links; false-done baseline |
| **gap** | 5 | Component/WIT CI, P2 doc split, host reachability, stdlib reachability docs |
| **deferred** | 0 | — |

Evidence baseline (2026-06-17):

- `python3 scripts/manager.py verify quick` → **166/166** passed
- `tests/fixtures/manifest.txt` → **1118** non-comment entries
- `python3 scripts/check/check-docs-consistency.py` → green
- `python3 scripts/check/check-false-done-hygiene.py` → PASS
- `skip-doc-check` in `docs/` → **213** occurrences across 29 files (delegated to **#683**)
- README → no links to `docs/migration/` or `docs/archive/`

### SoT contract (fixed by #679)

| Layer | Path | Role |
|-------|------|------|
| Structured state | `docs/data/project-state.toml` | `[verification]`, `[contract_audit]`, `project.updated` |
| Generator | `scripts/gen/generate-docs.py` | Emits `README_STATUS` + `CURRENT_STATE_*` markers |
| Behavioral contract | `docs/current-state.md` | Reader-facing snapshot (generated sections) |
| Tier vocabulary | `docs/target-contract.md` | guaranteed / smoke / scaffold (manual; P2 native text stale → **#680**) |

## 1. README / Quickstart claims (checklist §1 + §6)

| # | Check | Verdict | Evidence | Tracking |
|---|-------|---------|----------|----------|
| 1 | README `Wasm-first` backed through component/host/interop | **gap** | README status block lists targets only; `target-contract.md` L158–162: component-compile **smoke**, skip-on-CI when `wasm-tools` absent; no CI wasm-tools install | [#682](../../issues/open/682-component-wit-product-claim-audit.md) |
| 2 | README `Component/WIT target` CI-guaranteed | **gap** | `docs/target-contract.md` L61–62, L158–162: emit component = **smoke**; fixtures skipped without `wasm-tools` | [#682](../../issues/open/682-component-wit-product-claim-audit.md) |
| 3 | README status block ↔ `current-state.md` | **OK** | `docs/data/project-state.toml` `[verification]` updated 2026-06-17; `generate-docs.py` regenerates `README_STATUS` + `CURRENT_STATE_*`; `check_docs_runtime_contract()` enforces parity | fixed in **#679** |
| 4 | `current-state.md` ↔ `target-contract.md` (P2 native / component tier) | **gap** | `current-state.md` Component Model: gate_074 green, P2 native path described; `target-contract.md` L151–155: P2 native **deferred to v5+** | [#680](../../issues/open/680-target-tier-honesty-audit.md), [#668](../../issues/open/668-p2-native-component-polish.md) |
| 5 | `current-state.md` ↔ `capability-surface.md` (host reachability) | **gap** | `capability-surface.md`: `std::host::http|sockets|udp` **not user-reachable**; `src/compiler/wasm/call_host.ark` dispatches `call_host_network.ark` | [#675](../../issues/open/675-host-capability-reachability-flags.md), [#681](../../issues/open/681-stdlib-manifest-reachability-audit.md) |
| 6 | Generated stdlib docs ↔ `std/manifest.toml` availability | **gap** | `std/manifest.toml` lists `std::host::http` (6 fns), `sockets` (5), `udp` (1); generated reference has reachability badges but summary table weak vs manifest | [#681](../../issues/open/681-stdlib-manifest-reachability-audit.md) |
| 7 | Legacy / archived docs do not mislead from README | **OK** | `rg` on root `README.md`: no `docs/migration/` or `docs/archive/` links; archived content reachable only via explicit doc paths | inventory in **#679**; follow-up **#685** |
| 8 | `false-done-prevention.md` vs landing claims | **OK** | `check-false-done-hygiene.py` PASS; verify quick 166/166; no FD-03/FD-04 violation detected on README status claims in this pass | ongoing monitor **#684** |

## 6. Generated / manual source of truth

| Surface | Generated? | SoT | Drift found | Action |
|---------|------------|-----|-------------|--------|
| README `README_STATUS` | yes | `project-state.toml` + `generate-docs.py` | was stale (`Updated: 2026-05-14`, verify 22/22, manifest 838) | **fixed #679** |
| `current-state.md` markers | partial | same | same drift on Test Health / Updated | **fixed #679** |
| `target-contract.md` tier table | manual | ADR + fixture tiers | P2 native deferred text vs gate_074 reality | **#680** |
| `capability-surface.md` host table | manual | manifest + runtime dispatch audit | not-reachable vs `call_host_network` dispatch | **#675**, **#681** |
| Stdlib reference pages | generated | `std/manifest.toml` | reachability summary gaps | **#681** |
| Quickstart examples | manual | fixture / examples paths | 213 `skip-doc-check` in `docs/` | **#683** |

## Gap → issue matrix

| Gap ID | Description | Owner issue | Gate / fix status |
|--------|-------------|-------------|-------------------|
| G-679-01 | Wasm-first / component-host interop not CI-guaranteed | [#682](../../issues/open/682-component-wit-product-claim-audit.md) | open audit |
| G-679-02 | Component/WIT target not blocking CI | [#682](../../issues/open/682-component-wit-product-claim-audit.md) | wasm-tools CI → **#682** |
| G-679-03 | P2 native tier docs split (current-state vs target-contract) | [#680](../../issues/open/680-target-tier-honesty-audit.md), [#668](../../issues/open/668-p2-native-component-polish.md) | deferred text intentionally not fixed in #679 |
| G-679-04 | Host http/sockets/udp reachability docs vs runtime | [#675](../../issues/open/675-host-capability-reachability-flags.md) | implementation |
| G-679-05 | Stdlib manifest reachability / generated docs | [#681](../../issues/open/681-stdlib-manifest-reachability-audit.md) | audit |
| G-679-06 | Quickstart / skip-doc-check executable examples | [#683](../../issues/open/683-user-facing-executable-example-audit.md) | audit |
| G-679-07 | IDE/playground/extension claim cross-check | [#685](../../issues/open/685-ide-playground-extension-claim-audit.md) | audit |
| G-679-08 | False-done limited-scope re-audit program | [#684](../../issues/open/684-false-done-limited-scope-reaudit.md) | monitor |

All checklist rows (§1) map to OK verdict or a row above. **Unmapped gaps: 0.**

## Verification commands (audit close)

```bash
python3 scripts/manager.py verify quick          # 166/166
python3 scripts/check/check-docs-consistency.py  # includes check_docs_runtime_contract()
python3 scripts/check/gate-679-docs-runtime-contract-audit.py
python3 scripts/gen/generate-docs.py
```
