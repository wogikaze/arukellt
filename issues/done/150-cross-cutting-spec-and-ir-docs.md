# 横断 docs: `docs/language/spec.md` 凍結版と `docs/compiler/ir-spec.md` を整備

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 150
**Depends on**: —
**Track**: cross-cutting
**Blocks v1 exit**: no

## Summary

`docs/process/roadmap-cross-cutting.md` §6.4 / §6.6 は、v5 着手前に
`docs/language/spec.md` と `docs/compiler/ir-spec.md` が揃っていることを要求している。
現状は `docs/language/syntax.md`, `docs/language/type-system.md`, `docs/compiler/pipeline.md` はあるが、
「凍結対象の言語仕様」と「CoreHIR / MIR の正規仕様書」が欠けている。

## 受け入れ条件

1. `docs/language/spec.md` が追加され、v5 着手前の凍結対象として扱う範囲が明記される
2. `docs/compiler/ir-spec.md` が追加され、CoreHIR / MIR の主要 struct / enum / invariant / phase 境界が文書化される
3. `docs/compiler/pipeline.md` と `docs/language/type-system.md` から新規文書へ辿れる
4. `scripts/run/verify-harness.sh` または docs consistency check で上記 2 ファイルの存在が検証される

## 実装タスク

1. 既存の `docs/language/*.md`, `docs/compiler/pipeline.md`, `docs/current-state.md` を棚卸しし、凍結対象と current-first 説明を分離する
2. `docs/language/spec.md` に syntax / module / import / type / control flow / error model の凍結境界を書く
3. `docs/compiler/ir-spec.md` に CoreHIR / MIR のデータ構造、phase dump 名、validation invariant、pass 前後の前提を書く
4. 関連文書からのリンクを更新し、docs drift check に組み込む

## 参照

- `docs/process/roadmap-cross-cutting.md` §6.4, §6.6
- `docs/language/syntax.md`
- `docs/language/type-system.md`
- `docs/compiler/pipeline.md`
- `docs/current-state.md`
