# ADR-003: generics 戦略

ステータス: **ACCEPTED** — Monomorphization（型ごとのコード生成）を採用する

決定日: 2026-03-24  
改訂日: 2026-07-11 — 理想形を決定本文とし、一時的な実装制限を決定から除去

---

## 文脈

ADR-002 により Wasm GC を採用した。generics の実装戦略を決定する。

選択肢:

- **Monomorphization**: 型ごとに関数/構造体を生成（Rust 方式）
- **Type Erasure**: 統一表現 + 型情報を実行時に渡す（Java generics 方式）
- **Hybrid**: 基本 erasure、値型のみ specialization

---

## 選択肢の詳細

### 選択肢 A: Monomorphization

各 generic 型に対して専用コードを生成する。

利点:

- 値型の性能が最適
- 実装が単純
- LLM が理解しやすい（生成されるコードが直接的）

欠点:

- コードサイズが増大しうる
- コンパイル時間が増加しうる
- 深くネストした generics でコード量が膨らみうる（実装側の最適化・制限で制御する関心事であり、言語理想を狭めない）

### 選択肢 B: Type Erasure（GC 参照による統一）

GC 参照を統一表現として使用する。

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

**選択肢 A: Monomorphization を採用する。**

言語としての理想形（本 ADR の正）は次のとおり。

1. **ネストしたジェネリクスを許可する**
   - `Vec[Vec[T]]`、`Vec[Option[T]]`、ユーザー定義型の入れ子を含む
2. **ユーザー定義の generic struct / enum を許可する**
   - `Vec[T]`、`Option[T]`、`Result[T, E]` は標準ライブラリの例であり、組み込み特権に限定しない
3. **ジェネリック関数・メソッドを第一級とする**
   - 型パラメータ個数に言語上の上限を設けない
   - 将来の trait bounds（`fn foo[T: Eq](x: T)`）と整合する道を残す
4. **高階型（`F[_]`）は本 ADR の必須範囲外**
   - 必要になった時点で別 ADR / RFC で設計する（禁止を理想として固定しない）

Monomorphization のコードサイズ・コンパイル時間は、実装・最適化・診断の問題として扱い、
言語面を狭める「禁止事項」として本 ADR に固定しない。

### 根拠

1. **LLM フレンドリ性** — 生成コードが予測しやすい
2. **Wasm GC との整合性** — プリミティブは specialized、参照型は GC 参照として自然
3. **成長余地** — 標準ライブラリとユーザーコードが同じ generics モデルを共有できる

---

## 構文（理想）

```
// ジェネリック関数
fn identity<T>(x: T) -> T {
    x
}

// 複数の型パラメータ
fn pair<A, B>(a: A, b: B) -> (A, B) {
    (a, b)
}

// ユーザー定義 generic
struct Box[T] {
    value: T,
}

// ネスト
fn nested(v: Vec[Option[i32]]) { ... }

// 使用時（型推論が効く場合は省略可）
let x = identity(42)           // T = i32
let y = identity[String]("hi") // 明示指定
```

---

## Wasm GC での実現（イメージ）

```wasm
;; identity[i32] の生成結果
(func $identity_i32 (param $x i32) (result i32)
  (local.get $x))

;; identity[String] の生成結果
(func $identity_string (param $x (ref $string)) (result (ref $string))
  (local.get $x))
```

---

## 関連

- ADR-002: メモリモデル（Wasm GC 採用）
- ADR-000: ADR には理想形を書き、暫定制限・現状報告を決定に混ぜない
- `docs/language/type-system.md`: 型システム詳細
