---
Status: open
Created: 2026-06-15
Updated: 2026-06-15
ID: 651
Track: component-model
Depends on: 074, 124
Orchestration class: design-ready
Orchestration upstream: None
Blocks v{N}: none
Source: docs/current-state.md — WIT flags rejected with E0090; #618 cites "Separate feature"
---

# 651 — WIT `flags` type support (remove E0090 rejection)

## Summary

WIT `flags { ... }` declarations and flags-typed function signatures are rejected with
**E0090** at component preflight. The Component Model spec defines flags as a first-class
type; Arukellt v2 intentionally deferred them (#028, #028b). #618 notes flags as a
separate feature with no dedicated tracker.

## Evidence

- `docs/current-state.md` CLI integration: unsupported WIT import shapes such as `flags` → E0090
- `issues/open/618-wit-bindings-round-trip.md`: flags row → "Separate feature"
- `tests/fixtures/component/import_flags_type.ark` — E0090 guard fixture

## Non-goals

- Bitflags in Ark source language (stdlib concern, not WIT import)
- Resource/async types (#473, #474)

## Acceptance

- [ ] WIT parser accepts `flags` declarations in `--wit` files
- [ ] Canonical ABI lift/lower for flags in at least one export and one import shape
- [ ] E0090 removed for supported flags fixtures; unsupported combinations retain diagnostics
- [ ] Text WIT emitter can emit `flags` declarations round-trip
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Required verification

```bash
python3 scripts/manager.py verify quick
```

## Close gate

At least one flags round-trip fixture passes validate; current-state and #618 matrix updated.
