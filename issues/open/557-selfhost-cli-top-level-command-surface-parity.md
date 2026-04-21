---
id: 557
title: "Selfhost CLI top-level command surface parity"
status: open
track: selfhost-cli
created: 2026-04-22
updated: 2026-04-22
depends-on: [288, 318, 319, 459]
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

- [ ] Selfhost CLI either implements every top-level command it advertises or stops advertising unsupported commands
- [ ] `--help` output matches actual selfhost command reachability
- [ ] For each supported top-level command, invocation reaches a real handler rather than falling through to implicit compile / unknown-command behavior
- [ ] Differences from Rust CLI are documented explicitly if any remain intentionally out of scope

## Required verification

```bash
wasmtime run .build/selfhost/arukellt-s1.wasm -- --help
wasmtime run .build/selfhost/arukellt-s1.wasm -- init --help
wasmtime run .build/selfhost/arukellt-s1.wasm -- fmt --help
wasmtime run .build/selfhost/arukellt-s1.wasm -- targets --help
```

## Close gate

No user-visible command in selfhost help is a false existence claim.
