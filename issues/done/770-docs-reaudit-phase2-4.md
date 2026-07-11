---
Status: done
Created: 2026-07-11
Updated: 2026-07-11
ID: 770
Track: docs-audit
Depends on: 766, 767, 768, 769
Orchestration class: design-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: Docs re-audit 2026-07-11 (Phase 2–4)
Blocks: none
---

# 770 — Docs re-audit Phase 2–4 (structured state + lifecycle)

## Summary

After Phase 1 P0s (#766–#769): structure CLI/bootstrap/capability/release data,
make Accepted ADR strict-gated, ratchet skip-doc-check, retire
`syntax-v1-preview.md`, shrink `io.md` duplication, account fixture
manifest vs harness totals, and split huge manuals.

## Acceptance (incremental)

### Phase 2 (structured state) — done 2026-07-11

- [x] `cli-surface.toml` / `bootstrap-contract.toml` / `capabilities.toml` / `release-guarantees.toml` / `component-availability.toml`
- [x] Fixture accounting invariant (manifest registry vs observed harness pass/fail/skip; not same unit)
- [x] Component availability split: command vs library vs active artifact (multi-axis; no flat `available`)
- [x] Generated views via `scripts/gen/generate-structured-state-docs.py` (wired into `generate-docs.py` + verify)

### Phase 3 (lifecycle / gates) — done 2026-07-11

- [x] Accepted ADR not wholesale archive-excluded from vocab gates (`docs/adr/**` removed from allowlist; only Accepted scanned)
- [x] Structured `skip-doc-check` required (`reason` / `owner` / `kind` / `expires`) on budgeted paths
- [x] Budget decrease ratchet (`max` ≤ `ratchet_max`; guide 45→44, cookbook 38→37)

### Phase 4 (dedupe / shrink) — done 2026-07-11 (first slices)

- [x] `syntax-v1-preview.md` retired to `docs/history/language/`
- [x] `stdlib/modules/io.md` overview-only (cross-links; no duplicated API dump)
- [x] Manual split plan connected with owner/status/first slices (`docs/plans/docs-manual-split-plan.md`)
- [x] Release criteria/checklist point at shared `release-guarantees.toml` SSOT

### Follow-ups (tracked separately; not open acceptance)

- Further manual splits: `spec.md` / `cookbook.md` / `ir-spec.md` chapter cuts
- Continue skip budget decreases as #683 makes examples fixture-backed
- Optional: generate release-criteria/checklist bodies fully from TOML

## References

- Docs re-audit 2026-07-11 §9–§11
- Generator: `scripts/gen/generate-structured-state-docs.py`
- Gate: `scripts/check/gate-765-docs-ci-hard-gates.py`
