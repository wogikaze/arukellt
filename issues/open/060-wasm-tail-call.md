# Wasm tail-call: return_call / return_call_ref 実装

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 060
**Depends on**: —
**Track**: wasm-feature
**Blocks v4 exit**: yes


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/060-wasm-tail-call.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

WebAssembly 3.0 の Tail Call 提案 (`docs/spec/spec-3.0.0/proposals/tail-call/Overview.md`) が定義する
`return_call` / `return_call_indirect` / `return_call_ref` 命令を T3 emitter で使用する。
これにより末尾再帰関数のスタックオーバーフローを防ぎ、深い再帰パターンの実行時間を劇的に改善する。

## 背景

現在の T3 emitter (`crates/ark-wasm/src/emit/t3_wasm_gc.rs`) は末尾位置の `Call` を通常 `call` + `return`
の2命令に展開している。Wasm の `return_call` は1命令で末尾位置呼び出しを表現でき、
ランタイム (wasmtime 等) が末尾呼び出し最適化 (TCO) を適用できる。
`fib`・`loop` 変換後の CPS・状態機械パターンで特に効果大。

## 受け入れ条件

1. MIR レベルで「末尾位置呼び出し」を識別するフラグを `MirTerminator` に追加
2. T3 emitter が末尾位置 `Call` を `return_call` に変換
3. `call_ref` の末尾位置版も `return_call_ref` に変換
4. `call_indirect` の末尾位置版も `return_call_indirect` に変換
5. wasmtime が `return_call` をサポートしていることを確認し、fixture で深さ 100,000 の末尾再帰が成功
6. `--opt-level 0` では TCO 無効 (デバッグ用スタックトレース保持)

## 実装タスク

1. `ark-mir/src/mir.rs`: `MirTerminator::TailCall` バリアント追加
2. `ark-mir/src/lower.rs`: 末尾位置判定ロジック (`return` 直前の `Call` を検出)
3. `ark-wasm/src/emit/t3_wasm_gc.rs`: `TailCall` → `return_call` emit
4. `tests/fixtures/opt/tail_call_deep.ark`: 深さ 100k の末尾再帰テスト

## 検証方法

```bash
# 深い末尾再帰がスタックオーバーフローしないこと
wasmtime run tests/fixtures/opt/tail_call_deep.wasm
# return_call 命令がバイナリに存在すること
wasm-objdump -d tail_call_deep.wasm | grep return_call
```

## 完了条件

- 末尾再帰 depth=100,000 が wasmtime で実行完了する
- `return_call` 命令が emit されていることを wasm-objdump で確認
- 既存 fixture が --opt-level 1/2 で regression なし

## 参照

- `docs/spec/spec-3.0.0/proposals/tail-call/Overview.md`
- `docs/spec/spec-3.0.0/OVERVIEW.md` §tail-call
