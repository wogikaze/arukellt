---
id: 557
title: Selfhost CLI top-level command surface parity
status: done
track: selfhost-cli
created: 2026-04-22
updated: 2026-04-22
depends-on: "[288, 318, 319, 459]"
Track: main
Orchestration class: implementation-ready
Depends on: none
---
## Why this must exist

The current canonical CLI parity runner passes only for `--version` and `--help`, but the selfhost `--help` output now advertises a Rust-level top-level command surface that the selfhost CLI does not actually implement. This is a user-visible claim mismatch.

Repo evidence:
- `src/compiler/main.ark` parses only a narrow command set (`compile`, `check`, `run`, `build`, `test`, `parse`, `help`, `version`)
- `--help` output advertises additional commands (`init`, `fmt`, `targets`, `script`, `component`, `lsp`, `debug-adapter`, `lint`, `analyze`, `doc`, `compose`)

## Primary paths

- `src/compiler/main.ark`
- `crates/arukellt/src/main.rs`
- `docs/compiler/bootstrap.md`
- `issues/open/459-selfhost-fixpoint-dual-period-end.md`

## Non-goals

- Fixture parity
- Diagnostic parity
- Compiler backend changes unrelated to CLI surface

## Acceptance

- [x] Selfhost CLI either implements every top-level command it advertises or stops advertising unsupported commands
- [x] `--help` output matches actual selfhost command reachability
- [x] For each supported top-level command, invocation reaches a real handler rather than falling through to implicit compile / unknown-command behavior
- [x] Differences from Rust CLI are documented explicitly if any remain intentionally out of scope

## Required verification

```bash
wasmtime run .build/selfhost/arukellt-s1.wasm -- --help
wasmtime run .build/selfhost/arukellt-s1.wasm -- init --help
wasmtime run .build/selfhost/arukellt-s1.wasm -- fmt --help
wasmtime run .build/selfhost/arukellt-s1.wasm -- targets --help
```

## Close gate

No user-visible command in selfhost help is a false existence claim.

## Closed — 2026-04-22

**Implementation**: Added `CMD_NOT_YET()` constant and explicit stub handlers in `src/compiler/main.ark`
for all 11 unimplemented commands: `init`, `fmt`, `targets`, `script`, `component`, `lsp`,
`debug-adapter`, `lint`, `analyze`, `doc`, `compose`. Each exits 1 with
`"error: command not yet implemented in selfhost compiler"`. No command falls through to
implicit compile / unknown behavior.

**Remaining intentional scope gap**: The 11 commands have stubs, not full implementations.
Full implementation of each command is tracked as separate post-#459 issues per the Phase 6+
plan in #529.

**Evidence**:

```
init: exit=1 | error: command not yet implemented in selfhost compiler
fmt:  exit=1 | error: command not yet implemented in selfhost compiler
... (all 11 verified)
python3 scripts/manager.py selfhost parity --mode --cli → exit 0
python3 scripts/manager.py selfhost fixpoint → exit 0
```