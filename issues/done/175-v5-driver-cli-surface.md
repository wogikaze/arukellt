---
Status: done
Created: 2026-03-29
Updated: 2026-03-30
ID: 175
Track: main
Depends on: 162
Orchestration class: implementation-ready
Blocks v1 exit: False
# v5 Driver/CLI: command surface and exit behavior
---
# v5 Driver/CLI: command surface and exit behavior

## Summary

selfhost compiler の CLI entrypoint を整理し、`parse` / `compile` の surface、引数処理、exit code 契約を定義する。debug dumping は #167 で追う。

## Acceptance

- [x] `parse` / `compile` などの command surface が定義されている
- [x] 引数解釈 and usage / failure path の責務が明確になっている
- [x] 正常系 0 / 失敗系 1 の exit behavior を追跡できる

## References

- `issues/open/162-v5-phase1-parser.md`
- `crates/arukellt/src/main.rs`
- `crates/arukellt/src/commands.rs`