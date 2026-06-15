---
Status: done
Created: 2026-06-15
Updated: 2026-06-15
ID: 651
Track: component-model
Depends on: 074, 124
Orchestration class: design-ready
Orchestration upstream: None
Blocks v{N}: none
Source: docs/current-state.md — WIT flags rejected with E0090; #618 cites "Separate feature"
Status note: Closed — WIT flags parse/lift/lower wired; export/import fixtures pass validate.
---

# 651 — WIT `flags` type support (remove E0090 rejection)

## Summary

WIT `flags { ... }` declarations and flags-typed function signatures are rejected with
**E0090** at component preflight. The Component Model spec defines flags as a first-class
type; Arukellt v2 intentionally deferred them (#028, #028b). #618 notes flags as a
separate feature with no dedicated tracker.

## Non-goals

- Bitflags in Ark source language (stdlib concern, not WIT import)
- Resource/async types (#473, #474)

## Acceptance

- [x] WIT parser accepts `flags` declarations in `--wit` files
- [x] Canonical ABI lift/lower for flags in at least one export and one import shape
- [x] E0090 removed for supported flags fixtures; unsupported combinations retain diagnostics
- [x] Text WIT emitter can emit `flags` declarations round-trip
- [x] `python3 scripts/manager.py verify quick` exits 0

## Required verification

```bash
python3 scripts/manager.py verify quick
```

## Close gate

At least one flags round-trip fixture passes validate; current-state and #618 matrix updated.
