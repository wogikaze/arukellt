# セルフホストを「使える」状態へ引き上げる

**Status**: open
**Created**: 2026-03-31
**ID**: 282
**Depends on**: —
**Track**: main
**Blocks v1 exit**: no
**Priority**: 2

## Summary

Stage 1→Stage 2 の fixpoint は達成済みだが、セルフホストはまだ「ある」状態であり「使える」状態ではない。fixture parity・CLI parity・diagnostic parity がすべて未着手。`docs/current-state.md` の bootstrap 節も fixpoint 達成を反映していない（stale）。自己ホストを1個の大機能として扱うのではなく、明確な一本道で詰める。

## Current state

- ✅ Stage 0: Rust compiler が全 `src/compiler/*.ark` を個別コンパイル成功
- ✅ Stage 1: `arukellt-s1.wasm` が自身のソースをコンパイル成功
- ✅ Stage 2: `sha256(s1) == sha256(s2)` fixpoint 達成
- 🔴 Fixture parity: セルフホストコンパイラが 588 harness fixture を pass するか未検証
- 🔴 CLI parity: セルフホスト CLI と Rust CLI の出力一致が未検証
- 🔴 Diagnostic parity: エラーメッセージ一致が未検証
- 🔴 `docs/current-state.md` §Self-Hosting Bootstrap Status が stale（fixpoint 達成を反映していない）

## Acceptance

- [ ] `docs/current-state.md` の bootstrap 節が fixpoint 達成を正確に反映する
- [ ] fixture parity テスト: selfhost コンパイラで代表 fixture (少なくとも 50 個) をコンパイル＆実行し、Rust コンパイラの出力と一致
- [ ] CLI parity テスト: `arukellt compile`, `arukellt check`, `arukellt run` の基本フローが selfhost 版で動作
- [ ] diagnostic parity: 代表的なエラーケース（未定義変数、型不一致、構文エラー）で Rust 版と同等のメッセージ
- [ ] `scripts/verify-bootstrap.sh` が fixture parity を含む拡張検証モードを持つ
- [ ] CI で bootstrap 検証が定期的に走る（または手動トリガー可能）
- [ ] dual-period end condition の進捗が `docs/compiler/bootstrap.md` に記録される

## Approach

1. `docs/current-state.md` を fixpoint 達成に合わせて更新（Stage 1, 2 を ✅ に）
2. selfhost コンパイラで fixture サブセットをコンパイル＆実行するスクリプトを作成
3. Rust 出力との diff をとり、不一致箇所を特定
4. parser / resolver / typechecker / emitter の差分を順次修正
5. CLI エントリポイントの parity を確認
6. diagnostic 出力の format parity を確認
7. `verify-bootstrap.sh --fixture-parity` モードを追加

## References

- `docs/current-state.md` §Self-Hosting Bootstrap Status
- `docs/compiler/bootstrap.md`
- `scripts/verify-bootstrap.sh`
- `src/compiler/*.ark`
