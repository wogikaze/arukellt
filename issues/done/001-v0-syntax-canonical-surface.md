# v0 syntax surface is not canonical enough for LLM-oriented design

**Status**: done
**Created**: 2026-03-24
**Updated**: 2026-04-03

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: syntax.md is canonical, v0/v1 boundaries documented, all 24 items verified by docs/language/syntax.md

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/001-v0-syntax-canonical-surface.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

v0 の仕様文書間で自己矛盾があり、設計思想と表面構文が一致していない。LLM が安定して書くための canonical surface が崩れている。

### 問題 1: 仕様文書間の自己矛盾

- `impl` は v1 予約キーワードなのに syntax.md の構文例で使われている
- 文字列リテラル `"hello"` に `// str` コメントがあるが、使用例は `String::from("hello")`

### 問題 2: 設計思想と表面の乖離

- ADR-004 は trait なし・演算子オーバーロードなしを明記しているのに、syntax では通常の演算子とプリミティブメソッド（`.sqrt()` など）が前面に出ている
- 「method はあるが trait はない」という半端な世界になっている

### 問題 3: 正規形が多すぎる

- `[]` の多義性: generics (`Vec[T]`)、配列型 (`[i32; 3]`)、スライス型 (`[i32]`)、添字 (`arr[0]`)
- `!` の多義性: 論理否定と整数ビット否定
- `match` の高機能化: guard, or-pattern, struct destructuring が v0 に入っている
- 重い `main` 例: `main(caps: Capabilities) -> Result[(), AppError]` が構文入門の冒頭

## Acceptance Criteria

- [x] syntax.md 単体で v0 / v1 の境界が矛盾しない
- [x] 文字列リテラルの型が一意に決まる（v0 では `String` に固定）
- [x] `[]` の役割が 2 個以下に減る（generics は `<T>` に変更、`[]` は配列・スライス・添字のみ）
- [x] `match` の v0 機能が明文化される（guard, or-pattern, struct destructuring は v1 送り）
- [x] 最小 `main` 例 (`fn main() {}`) が最初に来る
- [x] ADR-004 と syntax.md の演算子方針が一致する（built-in 演算子の適用可能型を列挙）

## Implementation Phases

### Phase 1: 矛盾除去

`impl` を v0 から落とす。メソッド構文も v1 送り。

- `Point::new(...)` → `point_new(...)`
- `p.distance(...)` → `point_distance(p, q)`
- プリミティブメソッド (`.sqrt()`, `.len()`) → 組み込み関数 (`sqrt(x)`, `len(s)`)

根拠: trait なし、解決規則最小、LLM フレンドリを本気でやるなら、関数呼び出しの方が安定。

### Phase 2: 文字列の一本化

v0 では `"..."` の型を `String` に固定。`str` はユーザー表面に出さない。

### Phase 3: 記号の一意性を増やす

generics を `<T>` に変更:

- `Vec<T>`, `Result<T, E>`, `Option<T>`
- `fn identity<T>(x: T) -> T`

`[]` は配列・スライス・添字のみに閉じ込める:

- 配列型: `[i32; 3]`
- スライス型: `[i32]`
- 添字: `arr[0]`

### Phase 4: match を v0 サイズに削る

v0 の `match`:

- リテラルパターン
- enum variant パターン
- ワイルドカード `_`
- 変数束縛

v1 送り:

- guard (`n if n > 0`)
- or-pattern (`1 | 2`)
- struct destructuring (`Point { x, y }`)

### Phase 5: 演算子の位置づけの明記

built-in 演算子の適用可能型を v0 で列挙:

| 演算子 | 適用可能型 |
|--------|-----------|
| `+`, `-`, `*`, `/`, `%` | `i32`, `i64`, `f32`, `f64` |
| `==`, `!=` | 全プリミティブ型, `String` |
| `<`, `<=`, `>`, `>=` | 数値型, `char` |
| `&&`, `\|\|`, `!` (論理) | `bool` |
| `&`, `\|`, `^`, `!` (ビット), `<<`, `>>` | `i32`, `i64` |

`!` の曖昧性は型で解決: `bool` なら論理否定、整数なら bit NOT。

### Phase 6: 入門面の軽量化

最小 `main` 例を冒頭に:

```
fn main() {
    // 最小のエントリポイント
}
```

`Capabilities` と `Result` は後ろの「モジュール」「エラー処理」節に移動。

## Notes

### 関連文書

- `docs/language/syntax.md`: 主な修正対象
- `docs/language/type-system.md`: generics 記法の変更
- `docs/language/memory-model.md`: String の扱いを確認
- `docs/adr/ADR-004-trait-strategy.md`: 設計方針の根拠

### 背景

LLM にとって最も重要なのは簡潔さより正規形の少なさ。「Rust っぽさを借りているが、Rust の厳密さは借りていない」状態は、LLM が既存 Rust 分布に引っ張られてもっともらしいが仕様違反のコードを書く原因になる。

---

## 2026-03-24 第3段階レビュー

Phase 1-6 およびフォローアップ修正後、API 境界とセマンティクス境界の canonicalization を実施:

### 実施済み修正

1. **API 境界の明確化** ✅
   - Prelude / stdlib / capability 依存を 3 層で明示
   - Vec 基本操作を Prelude に追加（vec_new, vec_push, vec_pop, vec_get）
   - I/O は io.Caps 経由と明記

2. **mutation surface の固定** ✅
   - vec_push が in-place 変更であることを明示
   - struct フィールド更新は v0 では不可（v1 予定）と明記
   - 不変更新パターン（新規作成）を例示

3. **`[]` の意味の絞り込み** ✅
   - `[]` = 空固定長配列（型注釈必須）
   - 空 Vec は `vec_new()` を使用
   - スライス作成は `vec.as_slice` (import 必要)

4. **参照型の強調** ✅
   - syntax.md 冒頭に「重要: 参照型について」セクション追加
   - struct/enum/String/Vec/[T] が参照型であることを繰り返し強調
   - 代入 = オブジェクト共有（値コピーではない）を明示

5. **変数パターンの注意** ✅
   - catch-all であることを明記
   - 最後のアームでのみ使用するよう推奨

6. **未定義名の除去** ✅
   - error-handling.md の例を io.Caps, io.Error に統一
   - DirCap, RelPath, fs_read_file などの未定義名を削除

### 優先度: 高（v0 正規形を濁らせる）

1. **メソッド漏れ**: type-system に `arr.as_slice()` が残存。v0 メソッドなしと矛盾。
2. **String mutability 矛盾**: type-system は「可変長文字列」、memory-model は「immutable」。
3. **prelude 境界なし**: `sqrt`, `len`, `println`, `Some`, `None`, `Ok`, `Err` が暗黙出現。
4. **参照型エイリアシング未明記**: `point_distance(p, other)` が共有参照コピーなのか値コピーなのか。

### 優先度: 中

1. **v1 例の混在**: match の guard/or-pattern/struct destructuring がコード例として本文に残存。
2. **`==` の意味**: String の `==` が内容比較か参照同一性か未明記。
3. **タプルパターン境界**: `let (a, b) = ...` は OK だが `match` でのタプルパターン可否が曖昧。
4. **空コレクション `[]`**: スライスか配列か Vec 略記か未固定。

### 優先度: 低

1. **ネスト generic 禁止の表面反映**: 制限が見た目に出ていない。
2. **`?` 演算子の詳細**: Option 可否、エラー型変換の有無。
3. **`...` プレースホルダ**: コード例に無効トークン。
4. **`mut` の内部可変性**: 参照型フィールド更新との関係。

### 追加 Acceptance Criteria

- [x] type-system からメソッド呼び出し例を除去
- [x] String の mutability を一貫させる（immutable に統一）
- [x] prelude で自動的に見える名前を明記
- [x] 参照型のコピーセマンティクスを syntax で明記
- [x] v1 パターン例を別セクションまたは別ファイルに隔離
- [x] `==` が内容比較であることを明記
- [x] generics 記法を `<T>` に全文書で統一
- [x] error-handling の旧記法とメソッド呼び出しを除去
- [x] Option/Result Prelude 例外を明記
- [x] `?` 演算子の v0 制約を syntax に追記
- [x] `[T]` スライスを参照型一覧に追加

---

## 2026-03-24 Phase 2 完了 - LLM Surface Canonicalization

全 10 項目の Critical 矛盾を解消:

### 実施内容

**Phase A: Critical 矛盾解消**

1. `clone` を v0 正式採用（shallow clone、ネスト参照は共有）
2. `[]` を固定長配列専用に固定（スライスは `as_slice()` で作成）
3. `unwrap` を Prelude に追加
4. `Capabilities`/`AppError` を `io.Caps`/`io.Error` に統一

**Phase B: API 形の統一**
5. `vec.as_slice` → `as_slice`（裸関数に統一）
6. `string_push_char` → `string_append_char`（不変性を反映）

**Phase C: 説明の正規化**
7. field update コメント削除（Vec 例で共有を実例説明）
8. mutation boundary 明文化（Vec のみ in-place 変更可能）
9. struct-like variant 注意書き追加（定義 OK、match 分解は v1）

**Phase D: コード品質向上**
10. `// ...` プレースホルダを全削除（実行可能コードに置換）
11. generic ネスト禁止を強調（`Vec<Vec<T>>` 等の禁止例を明示）


### Phase 2 受け入れ条件達成状況

- [x] `clone` の v0/v1 矛盾が解消
- [x] `[]` の意味が一つに固定
- [x] Prelude/stdlib の全名前が定義済み
- [x] Vec/slice API の呼び出し形が統一
- [x] 存在しない構文を説明に使用していない
- [x] mutation 可能な型が明文化
- [x] コードブロックが全て型検査可能

---

## 次のステップ

Phase 2 で「LLM が最初に書きたくなる形と実際に許される形のズレ」は大幅に縮小。

今後の canonicalization 方向:

1. **診断設計** (search.md の原則に従う)
   - 構造化診断、expected/actual、fix-it hint
   - 生成→型検査→自己修正ループの安定化

2. **stdlib API の完全性**
   - v0 で必要な Vec 操作の全列挙
   - String 操作の正規形確定

3. **実装との同期**
   - コンパイラ側で v0 制限を強制
   - 禁止構文に対する明確なエラーメッセージ
