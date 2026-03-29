# 169: ブートストラップ手順ドキュメント

**Version**: v5
**Priority**: P2
**Depends on**: #166 (Bootstrap verification)

## 概要

`docs/compiler/bootstrap.md` にセルフホストのブートストラップ手順を記述する。

## 内容

1. ブートストラップの概念説明 (Stage 0/1/2)
2. 前提条件 (Rust toolchain, wasmtime, wasm-tools)
3. ステップバイステップの手順
4. fixpoint 検証方法
5. fixpoint 未達時のデバッグ方法
6. CI 統合方法
7. Rust 版と Arukellt 版の二重メンテナンス方針

## 完了条件

- `docs/compiler/bootstrap.md` が存在する
- 上記7項目をすべて含む
- 第三者がこのドキュメントだけでブートストラップを再現できる
