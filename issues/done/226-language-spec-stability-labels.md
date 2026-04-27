---
Status: done
Created: 2026-03-30
Updated: 2026-03-30
ID: 226
Track: main
Depends on: 223
Orchestration class: implementation-ready
Blocks v1 exit: yes
---
# 言語仕様に stable/provisional/experimental/removed の4段階ラベルを定義・適用する

## Summary

現状の言語仕様には機能の安定性ラベルが存在せず、利用者は「今書いたコードがいつまで動くのか」を判断できない。
この issue では、仕様書の全機能に `stable / provisional / experimental / removed` の4段階ラベルを定義し、適用する。

## Acceptance

- [x] 4段階ラベルの定義・判断基準が ADR として文書化されている
- [x] `docs/language/spec.md` の全機能セクションにラベルが付与されている
- [x] ラベルが付いていない機能が spec に残っていない
- [x] 利用者が「stable な機能だけを使う」選択ができる導線がある

## Scope

### ラベル定義

- `stable`：仕様として固定。破壊的変更は migration guide 付きでのみ許可
- `provisional`：概ね固定だが細部が変わる可能性がある
- `experimental`：実験中。破壊的変更の事前通知なし
- `removed`：廃止済み。使用するとエラーになる

### spec への適用

- `docs/language/spec.md` の各機能（構文・型・式・文・パターン・item）にラベル付与
- ラベルの表示形式の統一（各セクション冒頭にバッジ）

### ツール連携準備

- コンパイラが `experimental` 機能使用時に警告を出す仕組みの設計

## References

- `docs/language/spec.md`
- `docs/adr/`
- `issues/open/223-unified-terminology-for-implementation-status.md`
- `issues/open/227-document-language-contract.md`