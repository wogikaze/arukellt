---
Status: open
Created: 2026-04-03
Updated: 2026-04-03
ID: 475
Track: cli
Depends on: "035 (v2-verification-cleanup, done), 074 (wasi-p2-native-component)"
Orchestration class: blocked-by-upstream
Orchestration upstream: None
Blocks v4 exit: no
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
---

# arukellt component subcommand
---
# `arukellt component` サブコマンド (v3 候補)


---

## Decomposition note — 2026-04-03

この issue を 2 層に分解した。

| Layer | Issue | Scope |
|-------|-------|-------|
| CLI implementation | **#475 (this issue)** | build/inspect/validate subcommands 実装 |
| docs | #485 | CLI リファレンス docs 追加 |


475 の close gate から docs 更新を除外した。docs は #485 が担当する。

---

## Reopened by audit — 2026-04-03



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

- [x] `arukellt component --help` is reachable and shows subcommands
- [ ] `arukellt component build <file.ark>` compiles to `<file>.component.wasm`
- [ ] `arukellt component inspect <file.component.wasm>` prints the WIT world
- [ ] `arukellt component validate <file.component.wasm>` exits 0 for a valid component
- [ ] CLI reference docs updated with `component` subcommand
- [ ] `python scripts/manager.py verify` passes

## Required verification

- `arukellt component --help` in CI
- Integration test in `crates/arukellt/tests/`

## Close gate

All acceptance items checked; `arukellt component --help` is live in CI output.