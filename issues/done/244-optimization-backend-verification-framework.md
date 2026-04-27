---
Status: done
Created: 2026-03-30
Updated: 2026-03-31
ID: 244
Track: main
Depends on: 241, 242
Orchestration class: implementation-ready
Blocks v1 exit: no
---
# 最適化・backend の「無効でも使える/有効でも壊さない」検証体制を構築する

## Summary

MIR 最適化パスや複数 backend は「あるから有効」になりやすく、
「無効にしても正しく動く」「有効にしても既存の動作を壊さない」ことが保証されていない。
この issue では最適化・backend の検証体制を構築し、安全な拡張を可能にする。

## Acceptance

- [x] 各最適化パスを無効化した場合と有効化した場合で、出力の意味論的等価性が CI でテストされている
- [x] 新しい最適化パスの追加には「opt なし/opt あり の等価性テスト」が必須になっている
- [x] experimental backend が本線 target の CI に影響を与えない分離が確認されている
- [x] 最適化パスの on/off が CLI フラグで制御でき、デバッグに使える

## Scope

### 等価性テストの設計

- 最適化前後の出力が意味論的に等価であることを確認するテストフレームワーク
- 既存の fixture をベースにした「opt あり/なし 比較テスト」の追加

### 最適化パスの個別制御

- `--opt-pass=<name>` または `--no-opt` フラグの実装
- 各パスの名前・目的・対象の一覧文書化

### backend 分離の確認

- experimental backend（LLVM scaffold など）が本線 CI を汚染していないことの確認
- `--exclude ark-llvm` 相当の分離が Makefile / CI スクリプトで明示されていることの確認

### 回帰テスト

- 最適化バグが起きた場合に即座にテストケースを追加するワークフロー

## References

- `crates/ark-mir/src/opt/`
- `issues/open/241-define-primary-target-and-tier-others.md`
- `issues/open/242-ci-layer-structure.md`