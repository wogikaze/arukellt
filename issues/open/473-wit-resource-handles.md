---
Status: open
Created: 2026-06-15
Updated: 2026-06-15
ID: 473
Track: component-model
Depends on: 074, 124
Orchestration class: design-ready
Orchestration upstream: None
Blocks v{N}: none
Source: missing issue referenced by #618 and #262; docs/current-state.md E0402 resource rejection
---

# 473 — WIT resource handles (`resource`, `own<T>`, `borrow<T>`)

## Summary

Component Model resource types are rejected at compile time with **E0402**:

- WIT `resource` declarations in `--wit` files
- `own<T>` / `borrow<T>` handles in import/export signatures

The binary emitter encodes resource types in some paths, but the text WIT emitter has no
resource syntax, and import binding is blocked until #124 lands. #618 and #262 reference
this tracker but the issue file was never created.

## Evidence

- `docs/current-state.md` Tier 3 table: resource → E0402, not implemented
- `issues/open/618-wit-bindings-round-trip.md` rows cite #473
- `issues/done/121-wasi-p2-canonical-abi-hardening.md`: resource handle lowering rejected pending full WASI P2 resource model

## Non-goals

- `stream<T>` / `future<T>` async resources (#474)
- stdlib `std::component` surface (#054) — coordinate but do not subsume

## Acceptance

- [ ] Design doc: resource lifecycle (create, drop, borrow rules) for T3 component emit
- [ ] WIT import/export surface accepts `resource` + `own<T>` / `borrow<T>` where spec allows
- [ ] Canonical ABI lift/lower for at least one round-trip resource fixture
- [ ] E0402 removed for supported shapes; unsupported shapes retain explicit diagnostics
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Required verification

```bash
python3 scripts/manager.py verify quick
```

## Close gate

At least one resource handle round-trip fixture passes validate + wasmtime; docs/current-state
Tier 3 row updated from "not implemented" to partial or supported.
