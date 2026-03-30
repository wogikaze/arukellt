# Debug metadata + DAP adapter foundation

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-03-30
**ID**: 199
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: no

## Summary

source mapping metadata、`arukellt debug-adapter`、launch / attach 契約、標準 runtime path を定義し、Arukellt debug UX の最小土台を作る。debug 系 child issue の起点。

## Acceptance

- [x] source mapping / debug metadata の責務が追跡できる
- [x] DAP adapter と launch/attach contract が定義されている
- [x] 標準 runtime path ととの接続責務が issue queue 上で追跡できる

## References

- `issues/open/187-debug-surface-dap-and-source-level-debugging.md`
- `crates/arukellt/src/main.rs`
- `docs/current-state.md`
