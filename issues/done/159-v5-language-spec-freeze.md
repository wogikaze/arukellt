---
Depends on: なし
Priority: P0 — v5 着手の前提条件
Track: main
Orchestration class: implementation-ready
---
# 159: 言語仕様凍結版の作成

## 概要

`docs/language/spec.md` に Arukellt の型システム、構文、stdlib API の完全仕様を記述し、凍結コミットを行う。凍結後の仕様変更は ADR 必須とする。

## タスク

1. 型システム仕様の記述: プリミティブ型 (i32, i64, f64, bool, String), 複合型 (struct, enum, Vec, Option, Result), ジェネリクス制約 (最大2型パラメータ)
2. 構文仕様の記述: 式、文、パターンマッチ、関数定義、struct/enum 定義、import、モジュール
3. stdlib API 一覧: std/manifest.toml から全公開 API をリファレンス形式で記述
4. 演算子と優先順位表
5. エラーコード体系

## 完了条件

- `docs/language/spec.md` が存在し、上記5項目をすべて含む
- 凍結コミットが作成されている
- 仕様から Arukellt のパーサー・型検査器を再実装するのに十分な情報がある

## 備考

セルフホスト実装中の仕様変更は Rust 版と Arukellt 版の両方への反映が必要になるため、凍結前に仕様の安定性を確認する。