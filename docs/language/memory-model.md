# メモリモデル

ADR-002 により **Wasm GC 前提** で設計する。

---

## 概要

arukellt のメモリモデルは Wasm GC 命令セットを基盤とする。

- 参照型（`ref`）と GC 管理型（`struct`, `array`）を使用
- ホスト環境の GC に管理を委ねる
- ライフタイム管理・所有権モデルは言語仕様に含まない

---

## 値型と参照型

### 値型（value types）

スタック上にコピーされる型。GC ヒープに配置しない。

| 型 | Wasm 表現 |
|----|----------|
| `i32` | `i32` |
| `i64` | `i64` |
| `f32` | `f32` |
| `f64` | `f64` |
| `bool` | `i32` (0 or 1) |
| `char` | `i32` (Unicode scalar) |

### 参照型（reference types）

GC ヒープ上に配置される型。参照として渡される。

| 型 | Wasm 表現 |
|----|----------|
| `struct` | `(ref $struct_type)` |
| `enum` | `(ref $enum_type)` |
| `String` | `(ref $string)` |
| `Vec<T>` | `(ref $vec_T)` |
| `[T]` (slice) | `(ref $array_T)` + length |
| closure | `(ref $closure)` |

---

## GC 型の定義

### struct

```wasm
(type $Point (struct
  (field $x (mut f64))
  (field $y (mut f64))))
```

arukellt 構文:
```
struct Point {
    x: f64,
    y: f64,
}
```

### enum（tagged union）

```wasm
;; discriminant + 各バリアントのフィールド
(type $Option_i32 (struct
  (field $tag i32)           ;; 0 = None, 1 = Some
  (field $value (mut i32)))) ;; Some の場合のみ有効
```

より効率的な表現として、Wasm GC の `rec` 型を検討:
```wasm
(rec
  (type $Option_i32 (sub (struct (field $tag i32))))
  (type $Some_i32 (sub $Option_i32 (struct (field $tag i32) (field $value i32))))
  (type $None_i32 (sub $Option_i32 (struct (field $tag i32)))))
```

### array

```wasm
(type $i32_array (array (mut i32)))
(type $string (array (mut i8)))  ;; UTF-8 bytes
```

---

## String の表現

```wasm
(type $string_struct (struct
  (field $data (ref $u8_array))
  (field $len i32)))

(type $u8_array (array (mut i8)))
```

- UTF-8 エンコード
- 長さは文字数ではなくバイト数
- 不変（immutable）—変更には新規作成

---

## Vec の表現

```wasm
(type $vec_i32 (struct
  (field $data (mut (ref null $i32_array)))
  (field $len (mut i32))
  (field $cap (mut i32))))

(type $i32_array (array (mut i32)))
```

- 動的配列
- capacity 超過時に grow
- GC なので古い配列は自動回収

---

## クロージャの表現

```wasm
;; クロージャ = 関数参照 + キャプチャ環境
(type $closure_env_0 (struct
  (field $captured_x i32)
  (field $captured_y (ref $string))))

(type $closure_0 (struct
  (field $func (ref func))
  (field $env (ref $closure_env_0))))
```

- 各クロージャにキャプチャ環境の構造体を生成
- キャプチャは値コピー（GC 参照のコピー）
- mutable capture は ref として持つ

---

## Option / Result の最適化

### Option[T] where T is reference

null 許容参照を使用:
```wasm
(type $option_string (ref null $string))
;; None = ref.null, Some(s) = s
```

### Option[T] where T is value type

tagged union:
```wasm
(type $option_i32 (struct
  (field $is_some i32)
  (field $value i32)))
```

### Result[T, E]

常に tagged union:
```wasm
(type $result_i32_err (struct
  (field $tag i32)      ;; 0 = Ok, 1 = Err
  (field $ok i32)
  (field $err (ref $error_type))))
```

---

## linear memory との関係

Wasm GC 前提でも linear memory は以下に使用:

- WASI 呼び出し（`fd_write` 等の iovec）
- FFI 境界でのデータ交換
- 低レベルバッファ操作

GC ヒープと linear memory 間のコピーは明示的に行う。

---

## コピーセマンティクス

### 値型

暗黙コピー。`copy` 関数は不要。

```
let x: i32 = 42
let y = x  // コピー
```

### 参照型

参照のコピー。オブジェクト自体は共有。

```
let s1: String = "hello"
let s2 = s1  // s1 と s2 は同じオブジェクトを参照
```

深いコピーが必要な場合は明示的に `clone(s)` を呼ぶ（v1 で設計）。

---

## 関連

- ADR-002: この文書の根拠
- `docs/stdlib/core.md`: String / Vec の API 詳細
- `docs/platform/abi.md`: FFI 境界でのメモリ表現
