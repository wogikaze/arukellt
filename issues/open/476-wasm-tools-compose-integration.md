---
Status: open
Created: 2026-04-03
Updated: 2026-04-03
ID: 476
Track: wasm-feature
Depends on: "035 (v2-verification-cleanup, done), 074 (wasi-p2-native-component)"
Orchestration class: blocked-by-upstream
Orchestration upstream: #74
---

# `wasm-tools compose` 統合 (v3 候補)
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Source**: Future-work gap extracted from `issues/done/035-v2-verification-cleanup.md`.

**Quote**: "Composition tooling (`wasm-tools compose` integration, v3+)"

**Action**: New open issue created per audit rule — deferred items in done issues must
have a corresponding open issue.

---

## Summary

`wasm-tools compose` allows linking multiple `.component.wasm` files together according
to a composition YAML spec. Arukellt v2 does not support this workflow. This issue tracks:

1. Adding a `wasm-tools compose` integration path to the Arukellt build toolchain
2. Documenting how to compose Arukellt-produced components with third-party components
3. Adding a CI smoke test that produces two components and composes them

## Non-goals

- `arukellt component` CLI subcommand (separate issue 475)
- Async component support (separate issue 474)
- Native runtime composition without wasm-tools (out of scope)

## Primary paths

- `scripts/` — build/compose helper scripts
- `crates/arukellt/src/` — possible `arukellt compose` subcommand
- `tests/component-interop/` — composition smoke tests
- `docs/platform/wasm-features.md` — composition documentation

## Acceptance

- [ ] A `wasm-tools compose` round-trip smoke test exists in `tests/component-interop/compose/`
- [ ] Two Arukellt-produced components are composed successfully and run with wasmtime
- [ ] `docs/platform/wasm-features.md` documents the compose workflow
- [ ] CI gate (optional/`ARUKELLT_TEST_COMPOSE=1`) runs the compose test
- [ ] `python scripts/manager.py verify` passes

## Required verification

- Smoke test at `tests/component-interop/compose/run.sh` exits 0
- `wasmtime run` on the composed component produces expected output

## Close gate

All acceptance items checked; compose smoke test passes in CI (gated or ungated).
