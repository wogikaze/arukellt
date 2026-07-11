---
Status: open
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

- [ ] `cli-surface.toml` / `bootstrap-contract.toml` / `capabilities.toml` / release guarantees data
- [ ] Fixture accounting invariant (manifest vs selected/passed/failed/skipped)
- [ ] Accepted ADR not wholesale archive-excluded from vocab gates
- [ ] Structured `skip-doc-check` migration + decreasing budget
- [ ] `syntax-v1-preview.md` retired or reduced to unlanded items
- [ ] Component availability split: command vs library vs active artifact
- [ ] Manual split plan connected + first chapter slices

## References

- Docs re-audit 2026-07-11 §9–§11
