---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 674
Track: component-composition
Parent: 443
Depends on: "443, 663, 665 (done)"
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: P1 component composition checklist audit 2026-06-17
---

# 674 — Component composition: dependency wasm, lockfile, and external interop

## Summary

Issues #443/#665 delivered `arukellt compose` with WIT validation and Rust provider
round-trip. Remaining composition checklist items: resolve **component wasm**
dependencies from `ark.toml`, lockfile/cache prototype, missing-dependency
diagnostics, and external-language fixture matrix beyond Rust + jco.

## Acceptance

- [ ] `ark.toml` `[dependencies]` resolves **component `.wasm`** artifacts from vendor
      paths (not only WIT sidecars)
- [ ] Diagnostic when dependency wasm is missing or package id mismatches WIT
- [ ] Diagnostic when imported world is incompatible with app world
- [ ] Component dependency lockfile prototype (checked-in format + generator script)
- [ ] Component dependency cache directory convention under `.build/` or `.arukellt/`
- [ ] Compose output validation gate extended for multi-language graphs
- [ ] Fixtures (each with `run.sh` or verify hook):
  - [ ] Arukellt calls Go component
  - [ ] Arukellt calls C component
  - [ ] Arukellt calls Zig component
  - [ ] Python host calls Arukellt component
- [ ] Extract WIT from component binary (`wasm-tools component wit` equivalent) for
      dependency validation (ADR-034 gap)
- [ ] Gate `scripts/check/gate-674-component-composition-deps.py`
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Out of scope

- jco browser interop (#030 open)
- Full package registry / version-string dependency resolution

## References

- `docs/ark-toml.md` — `[dependencies]` section
- `tests/fixtures/wit_import/compose_roundtrip/`
- `issues/done/443-component-composition-linking-model.md`
