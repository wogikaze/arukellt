# Wasm Branch Hinting: カスタムセクションによるブランチ予測ヒント

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 064
**Depends on**: —
**Track**: wasm-feature
**Blocks v4 exit**: no

## Summary

WebAssembly Branch Hinting 提案 (`docs/spec/spec-3.0.0/proposals/branch-hinting/Overview.md`) を使い、
コンパイラが「likely / unlikely」ブランチを wasmtime に伝えることで、
JIT コンパイラのコードレイアウト最適化を促進する。
Arukellt のパターンマッチ (enum dispatch) と `if let` のエラーパス検出に活用できる。

## 受け入れ条件

1. MIR に `BranchHint::Likely` / `BranchHint::Unlikely` アノテーションを追加
2. T3 emitter がカスタムセクション `metadata.code.branch_hint` を生成
3. `@likely` / `@unlikely` 組み込みアノテーション構文のサポート (後半 ADR-004 P4 依存)
4. ヒントなし時と同一のセマンティクス (ヒントは pure hint)
5. wasmtime が branch hint カスタムセクションを認識することを確認

## 参照

- `docs/spec/spec-3.0.0/proposals/branch-hinting/Overview.md`
