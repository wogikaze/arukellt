# `arukellt component` サブコマンド (v3 候補)

**Status**: open
**Created**: 2026-04-03
**Updated**: 2026-04-03
**ID**: 475
**Depends on**: 035 (v2-verification-cleanup, done), 074 (wasi-p2-native-component)
**Track**: cli
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Source**: Future-work gap extracted from `issues/done/035-v2-verification-cleanup.md`.

**Quote**: "`arukellt component` subcommand (CLI convenience, v3 candidate)"

**Action**: New open issue created per audit rule — deferred items in done issues must
have a corresponding open issue.

---

## Summary

Arukellt CLI currently emits `.component.wasm` via `arukellt compile --emit component`.
This issue tracks adding a dedicated `arukellt component` subcommand that provides a
higher-level, more user-friendly interface for component operations:

- `arukellt component build` — compile to component WASM with better defaults
- `arukellt component inspect` — print WIT interface of a compiled component
- `arukellt component validate` — validate a `.component.wasm` against its WIT world

## Non-goals

- `wasm-tools compose` integration (separate issue 476)
- Async component support (separate issue 474)
- Changes to existing `arukellt compile` behavior

## Primary paths

- `crates/arukellt/src/main.rs` — CLI entry point, Commands enum
- `crates/arukellt/src/cmd_component.rs` — new command file (to be created)
- `docs/cli-reference.md` — CLI reference docs
- `crates/arukellt/tests/` — CLI integration tests

## Acceptance

- [ ] `arukellt component --help` is reachable and shows subcommands
- [ ] `arukellt component build <file.ark>` compiles to `<file>.component.wasm`
- [ ] `arukellt component inspect <file.component.wasm>` prints the WIT world
- [ ] `arukellt component validate <file.component.wasm>` exits 0 for a valid component
- [ ] CLI reference docs updated with `component` subcommand
- [ ] `bash scripts/run/verify-harness.sh` passes

## Required verification

- `arukellt component --help` in CI
- Integration test in `crates/arukellt/tests/`

## Close gate

All acceptance items checked; `arukellt component --help` is live in CI output.
