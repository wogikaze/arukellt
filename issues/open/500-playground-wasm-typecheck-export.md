
## Reopened by audit

- **Date**: 2026-04-21
- **Reason**: typecheck() implemented in Rust source and wired in worker.ts but wasm never compiled so API not actually exposed
- **Root cause**: The playground wasm binary (ark-playground-wasm) has never been compiled. crates/ark-playground-wasm/pkg/ does not exist. docs/playground/wasm/ is empty. All playground user-visible functionality depends on this binary.
- **Evidence**: `find . -name '*.wasm' -path '*playground*'` returns nothing; `ls crates/ark-playground-wasm/pkg/` fails; `ls docs/playground/wasm/` is empty.

# 500 — Playground: Wasm typecheck export

> **Status:** Implementation-ready
> **Track:** playground
> **Type:** Implementation

**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.
**Created:** 2026-04-14
**Updated:** 2026-04-14
**Source:** audit — issues/open/472-playground-type-checker-product-claim.md

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
