---
Status: done
Created: 2026-03-30
Updated: 2026-03-30
ID: 233
Track: main
Depends on: 231
Orchestration class: implementation-ready
---
# module / package / workspace / dependency の解決規則を仕様として固定する
**Blocks v1 exit**: yes

## Summary

module・package・workspace・dependency の解決規則が実装に埋まっており、仕様として文書化されていない。
「なぜこの import が解決されるのか」「workspace root はどこか」「dependency はどこから来るのか」が
コードを読まないと分からない状態では、大規模コードが組めない。

## Acceptance

- [x] module 解決規則（ファイルパス・名前空間・再エクスポート）が仕様として文書化されている
- [x] package 境界の定義（何が 1 package を構成するか）が明確になっている
- [x] workspace 発見アルゴリズムが仕様として記述されている
- [x] dependency 解決の優先順位（local path / workspace / registry の順）が定義されている

## Scope

### module 解決仕様

- ファイル名 → モジュール名のマッピング規則
- `import` の探索順序と優先順位
- re-export・wildcard import の扱い

### package / workspace 仕様

- package の単位（`ark.toml` を持つディレクトリ）の定義
- workspace root の発見アルゴリズム（上方探索・`[workspace]` セクションの有無）
- workspace メンバーの列挙方法

### dependency 解決仕様

- local path dependency の解決方法
- workspace dependency の解決優先順位
- 将来のレジストリ依存のための拡張ポイント設計

## References

- `docs/language/spec.md`
- `issues/open/231-ark-toml-as-project-model-entry-point.md`
- `issues/open/234-visibility-and-api-boundary-as-language-feature.md`