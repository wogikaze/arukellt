---
Status: open
Created: 2026-06-15
Updated: 2026-06-15
ID: 647
Track: selfhost-retirement
Depends on: 585
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Source: user request — legacy MIR selector no longer needed post selfhost-native transition
---

# 647 — Remove `--mir-select legacy` and legacy-path documentation

## Summary

`--mir-select legacy` was a Rust-era opt-in for the pre-CoreHIR AST→MIR lowering path.
The selfhost-native CLI (`src/compiler/main.ark` via `scripts/run/arukellt-selfhost.sh`)
no longer exposes `--mir-select` at all; CoreHIR is the only pipeline. Docs still claim
the legacy path is available via `--mir-select legacy` (`docs/current-state.md`,
`docs/compiler/legacy-path-migration.md`, `docs/compiler/ir-spec.md`,
`docs/design/INTERFACE-COREHIR.md`).

Retire the legacy selector surface entirely: docs, migration guides, and any stale
references in verification scripts or examples.

## Evidence

- `rg 'mir-select' docs/` — hits in current-state, legacy-path-*, ir-spec, INTERFACE-COREHIR
- `rg 'mir-select|mir_select' src/compiler scripts/` — no live CLI flag in selfhost driver
- Historical retirement context: #285, #508, #561, ADR-028, ADR-029

## Non-goals

- Renaming internal `mir::legacy_decl` / `legacy_body` adapter modules (separate hygiene if desired)
- Removing `driver/cli.ark` legacy standalone compile entry (unrelated `--mir-select` flag)

## Acceptance

- [ ] `docs/current-state.md` Pipeline section describes CoreHIR-only path (no legacy opt-in)
- [ ] `docs/compiler/legacy-path-migration.md` archived or deleted with pointer in retention policy
- [ ] `docs/compiler/legacy-path-status.md` archived (already marked historical; remove from active nav)
- [ ] `docs/compiler/ir-spec.md` and `docs/design/INTERFACE-COREHIR.md` no longer document `--mir-select legacy`
- [ ] `rg 'mir-select' docs/` returns only archived/historical paths (or zero hits)
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Required verification

```bash
rg 'mir-select' docs/
python3 scripts/check/check-docs-consistency.py
python3 scripts/manager.py verify quick
```

## Close gate

Docs and examples no longer advertise a legacy MIR selector that does not exist on the
selfhost CLI; archived migration material is clearly labeled historical.
