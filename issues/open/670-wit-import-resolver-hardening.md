---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 670
Track: language-design
Parent: 124
Depends on: "653 (wit-import-resolver-mir, done)"
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: P0 WIT import resolver checklist audit 2026-06-17
---

# 670 — WIT import resolver hardening (duplicates, spans, collisions)

## Summary

Issue #653 wired WIT imports through resolver, typechecker, and MIR for fixture-backed
surfaces. Production hardening items remain: duplicate alias detection, source-span
diagnostics, import-name collision policy, and developer dumps for WIT lowering.

## Acceptance

- [ ] Duplicate WIT import **aliases** in one module → structured diagnostic with span
- [ ] Duplicate WIT **package/interface** imports → structured diagnostic with span
- [ ] Unresolved WIT import alias at call site → span diagnostic (not plain string)
- [ ] Unknown WIT imported function → span diagnostic naming interface + symbol
- [ ] Unknown WIT imported type → span diagnostic
- [ ] Collision policy between `std::…` modules and WIT alias namespaces documented
      and enforced (reject or mangle with diagnostic)
- [ ] Collision policy between local `import foo` and WIT `import "…" as foo`
- [ ] `ARUKELLT_DUMP_PHASES=backend-plan` includes WIT import lowering summary
- [ ] Typechecker display names for WIT-generated types (hover-friendly paths)
- [ ] Gate script `scripts/check/gate-670-wit-import-resolver-hardening.py`
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `src/compiler/resolver/register_wit.ark`, `wit_register.ark`, `wit_surface.ark`
- `src/compiler/resolver/wit_import_bind.ark`
- `docs/adr/ADR-025-use-paths-vs-wit-package-identifiers.md`
