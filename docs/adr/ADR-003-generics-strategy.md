# ADR-003: generics 戦略

ステータス: **DECIDED**

決定日: 2026-03-24

---

## 文脈

ADR-002 により Wasm GC を採用した。generics の実装戦略を決定する。

選択肢:

- **Monomorphization**: 型ごとに関数/構造体を生成（Rust 方式）
- **Type Erasure**: 統一表現 + 型情報を実行時に渡す（Java generics 方式）
- **Hybrid**: 基本 erasure、値型のみ specialization

---

## 選択肢の詳細

### 選択肢 A: 制限付き Monomorphization

各 generic 型に対して専用コードを生成。

利点:

- 値型の性能が最適
- 実装が単純
- LLM が理解しやすい（生成されるコードが直接的）

欠点:

- コードサイズが増大
- コンパイル時間が増加
- 深くネストした generics で爆発的に増加

### 選択肢 B: Type Erasure（GC 参照による統一）

GC 参照を統一表現として使用。

利点:

- コードサイズが小さい
- コンパイル時間が短い
- Wasm GC の参照型と相性が良い

欠点:

- 値型のボクシングオーバーヘッド
- 参照型と値型の扱いが複雑

### 選択肢 C: Hybrid

参照型は erasure、値型は specialization。

利点:

- バランスの取れた性能
- コードサイズを抑制

欠点:

- 実装が複雑
- LLM が生成するコードの予測が難しい

---

## 決定

**選択肢 A: 制限付き Monomorphization を採用する**

### 制限事項（v0）

1. **ネストしたジェネリクスの禁止**
   - `Vec[Vec[T]]` は使用不可
   - `Vec[Option[T]]` は使用不可
   - ネストが必要な場合は専用型を定義

2. **ジェネリック構造体は標準ライブラリのみ**
   - `Vec[T]`, `Option[T]`, `Result[T, E]` は組み込み
   - ユーザー定義の generic struct は v1 以降

3. **ジェネリック関数は制限付き**
   - 型パラメータは 2 個まで
   - 高階型（`F[_]`）は使用不可

### 根拠

1. **LLM フレンドリ性**
   - monomorphization は生成されるコードが予測しやすい
   - erasure は間接参照が増え、デバッグが難しい

2. **Wasm GC との整合性**
   - GC 環境でもプリミティブ型は specialized が有利
   - 参照型は自然に GC 参照として統一

3. **v0 スコープの妥当性**
   - 制限を設けることでコード生成の爆発を防止
   - 実用的な範囲はカバー（Vec, Option, Result）

---

## 構文

```
// ジェネリック関数
fn identity<T>(x: T) -> T {
    x
}

// 型パラメータの複数指定
fn pair<A, B>(a: A, b: B) -> (A, B) {
    (a, b)
}

// 使用時（型推論が効く場合は省略可）
let x = identity(42)           // T = i32
let y = identity[String]("hi") // 明示指定
```

---

## Wasm GC での実現

```wasm
;; identity[i32] の生成結果
(func $identity_i32 (param $x i32) (result i32)
  (local.get $x))

;; identity[String] の生成結果
(func $identity_string (param $x (ref $string)) (result (ref $string))
  (local.get $x))
```

---

## 将来の拡張（v1 以降）

- ネスト制限の緩和
- ユーザー定義 generic struct
- trait bounds（`fn foo[T: Eq](x: T)`）

---

## 関連

- ADR-002: メモリモデル（Wasm GC 採用）
- ADR-004: trait 戦略（bounds 導入時期）
- `docs/language/type-system.md`: 型システム詳細
