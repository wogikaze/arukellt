# T3: 末尾位置の call → return_call 自動変換

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 090
**Depends on**: 060
**Track**: backend-opt
**Blocks v4 exit**: yes


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/090-t3-return-call-detection.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

T3 emitter レベルで「`call` の直後に `return` が来るパターン」を検出し、
`return_call` に自動変換するバックエンドレベル peephole を追加する。
MIR レベルの `TailCall` 変換 (#060) の補完として、
バックエンド生成コードでも末尾位置を見逃さないようにする。

## 受け入れ条件

1. `call X` + `return` を `return_call X` に変換
2. `call_ref $type` + `return` を `return_call_ref $type` に変換
3. `call_indirect (type $i)` + `return` を `return_call_indirect (type $i)` に変換
4. `--opt-level 0` では無効

## 参照

- `docs/spec/spec-3.0.0/proposals/tail-call/Overview.md`
- issue #060 (MIR level TCO)
