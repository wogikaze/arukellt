# Selfhost Phase 1: Multi-File Fixpoint

**Status**: open
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 593
**Parent**: #529
**Depends on**: —
**Blocks**: 594, 563
**Track**: selfhost
**Orchestration class**: implementation-ready

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

## Blocks

- #594 (Phase 2 can only start after fixpoint is stable)
- #563 (selfhost Phase 5 deletion gate depends on Phase 1+2)

---

## Acceptance

1. `python scripts/manager.py selfhost fixpoint` exits 0
2. sha256(s1) == sha256(s2) is explicitly verified
3. All pre-existing tests pass

---

## Required verification

```bash
python scripts/manager.py selfhost fixpoint
python scripts/manager.py verify quick
```

---

## STOP_IF

- Do not delete any Rust crate
- Do not merge if fixpoint check fails

---

## Close gate

Close when fixpoint exits 0 and all tests pass.
