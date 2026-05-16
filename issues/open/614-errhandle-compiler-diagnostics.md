---
Status: open
Created: 2026-04-22
Updated: 2026-05-15
ID: 614
Track: compiler / selfhost
Orchestration class: implementation-ready
Depends on: —
---

# Error Handling Convergence: Compiler Structured Diagnostics

## Reopened by issue-health audit — 2026-05-14

This file was under `issues/done/` while its frontmatter still said
`Status: open`, and it contains no close note or checked acceptance evidence.
Keep the issue open until the structured diagnostic model, CLI JSON output, and
fixture evidence below are implemented.

## Progress — 2026-05-14

- Confirmed `src/compiler/diagnostics.ark` already defines structured
  `Diagnostic` and `DiagnosticSpan` values with code, severity, span, and
  message fields.
- Confirmed parser diagnostics are carried into `CompileResult.diagnostics`
  in `src/compiler/driver.ark`.
- Confirmed JSON formatting includes a top-level `diagnostics` array with
  `code`, `severity`, `span`, and `message`.
- Added `--output json` as a compatibility alias for existing `--json` mode in
  `src/compiler/main.ark`; `-o json` remains available for a literal output file.

Verification completed:

- `python3 scripts/check/check-docs-consistency.py` passed
- `python3 scripts/manager.py verify` passed
- `git diff --check` passed

Not closed yet: required `python3 scripts/manager.py selfhost diag-parity` is
currently blocked by failure to bootstrap current selfhost wasm from the pinned
bootstrap wasm, so the issue close gate cannot be honestly satisfied in this
workspace state.

## Recheck — 2026-05-14

- The previous selfhost diagnostic-parity blocker is cleared:
  `python scripts/manager.py selfhost diag-parity` now passes.
- `wasmtime run --dir . bootstrap/arukellt-selfhost.wasm -- check
  tests/fixtures/selfhost/json_diag_code_presence.ark --output json` emits a
  machine-readable `diagnostics` array with `code`, `severity`, `span`, and
  `message` fields.
- Fixture evidence exists for the structured diagnostic value path
  (`tests/fixtures/diagnostics/structured_value.ark`) and coded diagnostic
  presence (`tests/fixtures/selfhost/json_diag_code_presence.ark`).

Not closed yet: the required `python scripts/manager.py verify fixtures` gate is
still red in this workspace (`PASS: 398 FAIL: 421 SKIP: 20`). Until the fixture
gate is either repaired or the close gate is intentionally narrowed, this issue
remains open.

## Recheck — 2026-05-15

- Re-ran the issue close gate with `python scripts/manager.py verify fixtures`.
- The fixture gate is still red in this workspace: `PASS: 417 FAIL: 410 SKIP: 20`.
- Early failures include `scalar/f32_local.ark`, `component/import_flags_type.ark`,
  and `component/import_scalar_func.ark`, followed by broad diagnostic/module/run
  fixture failures.

Not closed yet: structured diagnostic evidence is present, but the issue's
current close gate still requires a green full fixture pass.
---
# Error Handling Convergence: Compiler Structured Diagnostics

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
