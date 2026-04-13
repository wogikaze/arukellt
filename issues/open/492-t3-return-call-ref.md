# 492 — T3 tail-call: implement return_call_ref

**Track:** wasm-backend
**Status:** open
**Created:** 2026-04-14
**Updated:** 2026-04-14
**Source:** audit — issues/done/060-wasm-tail-call.md AC3

## Summary

Issue #060 closed with `return_call_ref` explicitly marked as future work
because there was no `call_ref` usage in the test suite at the time.
No open issue tracks this remaining gap.

## Primary paths

- `crates/ark-wasm/src/`
- `tests/fixtures/`

## Non-goals

- Changing tail-call proposal support for `return_call` (already implemented)
- General call_ref feature work beyond tail-call context

## Acceptance

- [ ] T3 emitter emits `return_call_ref` for indirect tail calls when the source uses a function-reference tail position
- [ ] At least one positive fixture exercises `return_call_ref`
- [ ] `cargo test` passes

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes

## Close gate

Acceptance items checked; fixture proves `return_call_ref` emission.
