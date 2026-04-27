---
Status: done
Created: 2026-03-30
Updated: 2026-04-15
ID: 170
Track: main
Depends on: 165, 166, 169
Orchestration class: implementation-ready
---
# v5 Migration guide
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: Guide claims selfhost as primary path but docs/current-state.md says Stage 2 fixpoint not reached.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Completed — 2026-04-15

**Evidence review**:
- `docs/migration/v4-to-v5.md` no longer claims selfhost is already the primary compilation path.
- The guide now states the dual period explicitly and points readers to
  `docs/current-state.md` and `docs/compiler/bootstrap.md` for the verified status.
- `python3 scripts/check/check-docs-consistency.py` passed for the correction slice.

**Close gate**: satisfied by current-first docs alignment.

## Summary

`docs/migration/v4-to-v5.md` に v4→v5 移行ガイドを整理する。特に、デフォルトコンパイラ切り替えの有無、bootstrap 手順の位置づけ、Rust 版と selfhost 版の二重メンテナンス方針を明文化する。

## Acceptance

- [x] v5 で追加された selfhost compiler / bootstrap workflow の説明がある
- [x] デフォルトコンパイラ切り替えの有無が曖昧さなく記述されている
- [x] 開発者向けに Rust 版と selfhost 版の併走方針が記述されている

## References

- `issues/open/165-v5-phase3-wasm-emitter.md`
- `issues/open/166-v5-bootstrap-verification.md`
- `issues/open/169-v5-bootstrap-doc.md`
- `docs/current-state.md`