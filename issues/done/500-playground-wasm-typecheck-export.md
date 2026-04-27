---
Created: 2026-04-14
Updated: 2026-04-14
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
Source: audit — issues/open/472-playground-type-checker-product-claim.md
Track: main
Orchestration class: implementation-ready
Depends on: none
---

- [x] `crates/ark-playground-wasm/src/lib.rs` exports `pub fn typecheck(source: &str) -> String` backed by a real `ark-typecheck` call
- Native test exercising `ark_playground_wasm: ":typecheck()` passes"
## Reopened by audit

## Summary

Issue #472 audited the playground type-checker product claim and found that
`crates/ark-playground-wasm/src/lib.rs` exports only `parse`, `format`,
`tokenize`, and `version`. There is no `typecheck` export backed by
`ark-typecheck`. The playground browser entrypoint and TypeScript source
(`playground/src/`) have no typecheck invocation.

This issue tracks adding the `typecheck` wasm binding and wiring it into the
playground frontend, which is the prerequisite for closing #472.

## Primary paths

- `crates/ark-playground-wasm/src/`
- `playground/src/`

## Non-goals

- Changing compiler architecture
- Full IDE-quality diagnostics in playground (incremental work)

## Acceptance

- [x] `crates/ark-playground-wasm/src/lib.rs` exports `pub fn typecheck(source: &str) -> String` backed by a real `ark-typecheck` call
- [x] `playground/src/` calls the typecheck export and surfaces the result
- [x] At least one native-target test exercises the typecheck WASM export
- [x] `python scripts/manager.py verify quick` passes

## Required verification

- `python scripts/manager.py verify quick` passes
- Native test exercising `ark_playground_wasm::typecheck()` passes

## Close gate

Acceptance items checked; typecheck export is present in lib.rs and invoked from playground/src/.

## Note

Closing this issue is a prerequisite for closing #472 (playground type-checker product claim).