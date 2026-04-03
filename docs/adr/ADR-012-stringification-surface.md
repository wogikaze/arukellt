# ADR-012: stringification surface は `to_string(x)` を canonical にする

ステータス: **DECIDED**

決定日: 2026-03-29

## 文脈

Arukellt には文字列化の入口が複数ある。

- 明示 helper: `i32_to_string`, `i64_to_string`, `f64_to_string`, `bool_to_string`, `char_to_string`
- 文字列補間: `f"..."`
- trait / impl ベースの `Display`
- メソッド構文による `.to_string()`

一方で、実装は既に `f"..."` を `to_string(expr)` に desugar している。
つまりコンパイラ内部では generic な `to_string(x)` が基準に近い。

ここでユーザー向けにどの表記を第一に教えるかを固定しないと、docs / fixtures / LSP / 将来の selfhost code で surface がぶれる。特に LLM にとっては、型ごとに `i32_to_string` などを覚え分けるより `to_string(x)` の方が生成しやすい。

## 選択肢

### 選択肢 A: primitive helper を canonical のまま維持

`i32_to_string(x)` などを主導線のまま残す。

利点:

- 現状の docs/benchmarks と一致しやすい
- バックエンド helper と 1:1 で分かりやすい

欠点:

- LLM に型別 helper 名を覚えさせる必要がある
- `f"..."` の内部 lowering とユーザー向け guidance がずれる
- trait / Display / method syntax が入った現在の surface に対して不自然

### 選択肢 B: `to_string(x)` を canonical にし、helper は互換として残す

free-function 形式の `to_string(x)` を docs と examples の第一表記にする。
primitive helper は backend / compatibility surface として維持する。

利点:

- LLM が最も生成しやすい
- `f"..."` desugar と一致する
- `.to_string()` だけに依存しないので method syntax の安定性に引きずられにくい
- 既存 helper を即削除せずに移行できる

欠点:

- 内部では型ごとの helper へ dispatch する実装が残る
- 古い docs の置換が必要

### 選択肢 C: `.to_string()` を canonical にする

trait / method syntax を主導線にする。

利点:

- 一般的な言語体験に近い
- `Display` と整合する

欠点:

- method syntax / trait resolution の安定性に依存する
- LLM には receiver method と free function のどちらが安定か判断しづらい
- 現行 repo の「まずは関数呼び出し形式を基準にする」という guidance と衝突する

## 決定

**選択肢 B: `to_string(x)` を canonical にする。**

### 決定内容

1. ユーザー向けに最初に教える文字列化 surface は `to_string(x)` とする。
2. `f"..."` は `to_string(expr)` + `concat` の sugar として扱う。
3. `i32_to_string` などの primitive helper は backend / compatibility surface として残すが、docs の第一候補にはしない。
4. builtin scalar と text values (`i32` / `i64` / `f64` / `bool` / `char` / `String`) を `to_string(x)` の保証対象とする。
5. `.to_string()` は secondary sugar とみなし、method syntax / trait resolution の stable guidance が固まるまでは canonical 扱いしない。
6. user-defined `Display` 相当の説明でも、まず `to_string(x)` が portable な形であることを優先する。

## Non-goals

このADRが**扱わない**スコープを明示する。

1. **compiler の `to_string()` dispatch 実装** (#484 スコープ) — ADR は設計決定のみ; `to_string(x)` が型ごとのバックエンド helper にどう dispatch されるかの実装変更は #484 で行う。
2. **`i32_to_string` / `i64_to_string` 等 primitive helper の削除** — これらは互換 surface として残す。このADRは canonical guidance を変えるのみであり、helper を廃止または削除しない。
3. **`.to_string()` method syntax の canonical 採用** — メソッド構文による `.to_string()` を primary path とすることは、trait / method syntax resolution が stable になるまで保留 (ADR-004 および #157 参照)。これは除外スコープ。
4. **Display trait / stdlib モジュール構造の変更** — `Display` trait の設計やstdlib モジュールの再編はこのADRの範囲外。
5. **docs/quickstart・cookbook・fixture の更新** — 既存ドキュメントや fixture への `to_string(x)` 表記の横断反映は #171 スコープ。

## 根拠

1. **LLM friendliness**
   - 型ごとの helper 名を暗記するより `to_string(x)` の方が一貫して書きやすい
   - `.to_string()` よりも free-function 形式の方が current docs の safe baseline と整合する
2. **実装との一致**
   - parser の f-string lowering は既に `to_string(expr)` を使う
   - emitter 側も `to_string` を scalar helper へ dispatch する構造を持っている
3. **移行コストの低さ**
   - primitive helper を残せば既存コードを壊さず guidance だけ先に統一できる

## 結果

この決定により、以後の docs / cookbook / quickstart / fixtures では:

- まず `to_string(x)` を使う
- helper 名は reference / compatibility 文脈でのみ列挙する
- `.to_string()` は trait/method 機能の説明では扱うが primary path にはしない

## 関連

- `docs/adr/ADR-004-trait-strategy.md`
- `issues/open/157-adr004-method-syntax-evaluation.md`
- `crates/ark-parser/src/parser/expr.rs`
- `std/manifest.toml`
