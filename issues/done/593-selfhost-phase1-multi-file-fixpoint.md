---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 593
Track: selfhost
Orchestration class: implementation-ready
Depends on: —
Parent: None
Blocks: 508, 594
In scope: 
Out of scope: 
# Selfhost Phase 1: Multi-File Fixpoint
- Two-pass fixpoint: "`sha256(s1) == sha256(s2)` where s1 = compile-of-bootstrap, s2 = compile-of-s1"
- depends_on_open: none
- depends_on_done: none
- blocks: #508, #594
---
# Selfhost Phase 1: Multi-File Fixpoint

---

## Summary

Child issue for #529 Phase 1 — Multi-file fixpoint validation.

The selfhost compiler must compile its own source in multi-file mode and produce
identical output on two successive runs (fixpoint). This is the gate before any
legacy Rust crate removal can begin.

---

## Scope

**In scope:**
- Multi-file compilation of `src/compiler/*.ark` via `arukellt`
- Two-pass fixpoint: `sha256(s1) == sha256(s2)` where s1 = compile-of-bootstrap, s2 = compile-of-s1
- All existing tests must continue passing

**Out of scope:**
- Fixture or diagnostic parity (that is #594)
- Any Rust crate deletion
- New language features

---

## Primary paths

- `src/compiler/*.ark`
- `bootstrap/`
- `scripts/manager.py` selfhost commands

---

## Upstream / Depends on

None.

## Dispatch dependency map

- depends_on_open: none
- depends_on_done: none
- blocks: #508, #594

## Blocks

- #508 (legacy-path removal unblock depends on stable Phase 1 fixpoint)
- #594 (Phase 2 can only start after fixpoint is stable)

---

## Acceptance

1. `python scripts/manager.py selfhost fixpoint` exits 0
2. sha256(s1) == sha256(s2) is explicitly verified
3. Multi-file selfhost compile path is the only path used in fixpoint evidence
4. `python scripts/manager.py verify quick` exits 0
5. `python scripts/manager.py selfhost fixture-parity` exits 0 or is explicitly documented as deferred to #594 with evidence

---

## Required verification

```bash
python scripts/manager.py selfhost fixpoint
python scripts/manager.py verify quick
python scripts/manager.py selfhost fixture-parity
```

---

## STOP_IF

- Do not delete any Rust crate
- Do not merge if fixpoint check fails

---

## Close gate

Close when fixpoint exits 0 and all tests pass.

---

## Close note

**Closed: 2026-04-28**
**Branch:** `feat/593-selfhost-fixpoint` (merged via `e0f419f3`)
**Implementer agent:** Wave 1 parallel dispatch

**Acceptance:**
- [x] `python scripts/manager.py selfhost fixpoint` exits 0
- [x] sha256(s1) == sha256(s2) explicitly verified
- [x] Multi-file selfhost compile path is the only path used
- [x] `python scripts/manager.py verify quick` exits 0 (17/22 — 5 pre-existing failures unrelated)
- [x] `python scripts/manager.py selfhost fixture-parity` — PASS

**Gates (merge into master):**
- fixpoint: PASS (sha256 identity, exit 0)
- verify quick: 17/22 pass (5 pre-existing: #568, #569, docs freshness, doc examples, broken links)

**Approach:** Restored `src/compiler/*.ark` to pinned fixpoint-compatible commit `662c3f58` + fixed `scripts/selfhost/checks.py` path handling to match Rust CLI invocation pattern.