---
Status: done
Updated: 2026-03-30
ID: 163
Track: main
Depends on: 175, 176
Orchestration class: implementation-ready
Blocks v1 exit: False
Status note: Parent issue for the selfhost driver / CLI surface. Debug dumping is tracked separately in
# v5 Phase 1: Driver + CLI epic
---
# v5 Phase 1: Driver + CLI epic


## Summary

Phase 1 の driver/CLI は、コマンド surface とコンパイルパイプライン接続を分けて追う必要がある。特に `ARUKELLT_DUMP_PHASES` は継続的に拡張されるため、この issue から切り離して #167 に集約する。

## Acceptance

- [x] #175, #176 が完了している
- [x] `parse` / `compile` / exit code / file loading の責務が child issue に分解されている
- [x] debug dump の責務が #167 にのみ存在し、本文間で重複していない

## References

- `issues/open/162-v5-phase1-parser.md`
- `issues/open/167-v5-debug-dump-phases.md`
- `crates/ark-driver/src/session.rs`
- `crates/arukellt/src/main.rs`