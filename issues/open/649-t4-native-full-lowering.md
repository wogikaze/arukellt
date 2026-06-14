---
Status: open
Created: 2026-06-15
Updated: 2026-06-15
ID: 649
Track: wasm-feature
Depends on: 641
Orchestration class: design-ready
Orchestration upstream: None
Blocks v{N}: none
Source: docs/current-state.md Known Limitations — T4 scaffold-only (#641); full lowering deferred
---

# 649 — T4 native full lowering (beyond scaffold #641)

## Summary

`wasm32-native` (T4) is **scaffold-only** per #641: `native::emit_native_scaffold` produces
a compile-only GNU assembler stub; `run_supported=false`. `docs/current-state.md` notes
full selfhost-native lowering as follow-up work with no open tracker after #586 removed
`ark-llvm`.

## Evidence

- `docs/current-state.md` Targets + Known Limitations: T4 scaffold, #641, #529 Phase 7 follow-up
- `issues/done/641-t4-selfhost-native-backend-scaffold.md` — close gate was scaffold registration only
- `src/compiler/native.ark` — `emit_native_scaffold` only

## Non-goals

- LLVM / ark-llvm revival (#586 deleted the Rust crate)
- T2 freestanding execution (#645 playground lane)

## Acceptance

- [ ] Design: selfhost-native lowering strategy (asm backend scope, host ABI, GC model)
- [ ] `run_supported=true` for a minimal fixture set OR honest perpetual compile-only boundary in docs
- [ ] Driver `implemented` / `run_supported` flags match reality
- [ ] At least one `tests/fixtures/t4/` entry proves compile+run OR compile-only contract
- [ ] `docs/current-state.md` and `docs/target-contract.md` T4 rows synced
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Required verification

```bash
python3 scripts/manager.py verify quick
python3 scripts/check/check-docs-consistency.py
```

## Close gate

T4 status in current-state is honest and backed by fixtures; scaffold-only language removed
unless compile-only remains the deliberate permanent boundary (then documented as such).
