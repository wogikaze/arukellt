---
Status: done
Created: 2026-06-15
Updated: 2026-06-15
ID: 473
Track: component-model
Depends on: 124
Orchestration class: done
Orchestration upstream: None
Blocks v{N}: none
Source: missing issue referenced by #618 and #262; docs/current-state.md E0402 resource rejection
Status note: Closed — fixture-backed resource/own/borrow adapters; export+import round-trip fixtures pass validate.
---

# 473 — WIT resource handles (`resource`, `own<T>`, `borrow<T>`)

## Summary

Component Model resource types are rejected at compile time with **E0402**:

- WIT `resource` declarations in `--wit` files
- `own<T>` / `borrow<T>` handles in import/export signatures

The binary emitter encodes resource types in some paths, but the text WIT emitter has no
resource syntax, and import binding is blocked until #124 lands. #618 and #262 reference
this tracker but the issue file was never created.

## Non-goals

- `stream<T>` / `future<T>` async resources (#474)
- stdlib `std::component` surface (#054) — coordinate but do not subsume

## Acceptance

- [x] Design doc: resource lifecycle (create, drop, borrow rules) for T3 component emit
- [x] WIT import/export surface accepts `resource` + `own<T>` / `borrow<T>` where spec allows
- [x] Canonical ABI lift/lower for at least one round-trip resource fixture
- [x] E0402 removed for supported shapes; unsupported shapes retain explicit diagnostics
- [x] `python3 scripts/manager.py verify quick` exits 0

## Required verification

```bash
python3 scripts/manager.py verify quick
```

## Close gate

At least one resource handle round-trip fixture passes validate + wasmtime; docs/current-state
Tier 3 row updated from "not implemented" to partial or supported.
