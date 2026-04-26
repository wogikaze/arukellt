# Error Handling Convergence: Compiler Structured Diagnostics

**Status**: open
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 614
**Parent**: #592
**Depends on**: —
**Track**: compiler / selfhost
**Orchestration class**: implementation-ready

---

## Summary

Child issue for #592 Stream 2 — Compiler Diagnostics.

The Arukellt selfhost compiler currently uses ad-hoc string vectors for diagnostic
messages. This issue delivers a minimal structured diagnostic model so that:

- error messages carry: code, span (file, line, col), severity, human message
- tooling (e.g. CLI, IDE) can process machine-readable diagnostics without string parsing
- fixture expectations can match on diagnostic codes, not just raw message text

---

## Scope

**In scope:**
- Define a `Diagnostic` data type in `src/compiler/` with: code, severity, span, message
- Migrate at least one compiler phase (parser or typechecker) to emit `Diagnostic` structs
- Update fixture expectations for migrated phase to match on diagnostic codes
- Ensure JSON diagnostic output is machine-readable from the CLI (`--output json`)

**Out of scope:**
- Panic / ICE policy (that is #615)
- IDE/LSP diagnostic protocol mapping (that is #xxx)
- Full migration of all compiler phases in one shot — start with one phase, prove the model

---

## Primary paths

- `src/compiler/diagnostics.ark` (new or existing)
- `src/compiler/parser.ark` (first migration target)
- `src/compiler/typechecker.ark` (second migration target if phase 1 is clean)
- `tests/fixtures/diagnostics/` (fixture expectations)
- `crates/arukellt/src/cli.rs` (JSON output mode)

## Allowed adjacent paths

- `crates/ark-diagnostics/` (Rust-side diagnostic structs — must stay in sync)
- `docs/compiler/` (diagnostic format docs)

---

## Upstream / Depends on

None.

## Blocks

- Closes the compiler stream of #592

---

## Acceptance

1. `Diagnostic` type has code, severity, span, message fields
2. At least one compiler phase emits structured `Diagnostic` values
3. `arukellt compile --output json` includes a structured `diagnostics` key with code and span
4. Fixture expectations test at least one diagnostic code, not just message text
5. Existing fixtures do not regress

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python scripts/manager.py selfhost diag-parity
```

---

## STOP_IF

- Do not migrate all phases at once before the model is proven with one phase
- Do not invent a new panic-handling system in this issue (#615 owns that)
- Do not modify the IDE LSP protocol

---

## Close gate

Close when: `Diagnostic` struct is defined, at least one phase emits it, CLI JSON output
includes structured diagnostics, and at least one fixture tests a diagnostic code.
