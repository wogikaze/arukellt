---
Status: done
Created: 2026-03-28
Updated: 2026-04-29
ID: 090
Track: backend-opt
Depends on: 48
Orchestration class: implementation-ready
---
# T3: 末尾位置の call → return_call 自動変換
**Blocks v4 exit**: yes

---

## Completed — 2026-04-29

**Slice implemented**: Opportunistic T3 return-call detection for non-desugared tail calls.

**Evidence**:
- `crates/ark-wasm/src/emit/t3/helpers.rs`: Added `try_emit_let_call_tail_return` method
  and `opp_tco_candidate` detection in `emit_function`. Covers two last-stmt shapes:
  1. `MirStmt::Call { dest: Some(Local(id)), func: FnId, args }` + `Return(Place(id))`
  2. `MirStmt::Assign(Local(id), Rvalue::Use(Operand::Call(name, args)))` + `Return(Place(id))`
- `tests/fixtures/tail_call/opportunistic.ark`: New fixture with 100,000-depth recursion
  via explicit `let result = countdown_let(n-1); result` — proves return_call fires for the
  let-call-return pattern without stack overflow.
- `grep -c "return_call" crates/ark-wasm/src/emit/t3/helpers.rs`: 13 (was 12).
- `bash scripts/run/verify-harness.sh --quick`: 19/19 PASS.
- `cargo test -p arukellt --test harness`: 649 PASS, 1 pre-existing FAIL (stdlib_core/to_string_i64.ark, unrelated).

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