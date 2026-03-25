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

**v0 の `clone`**: deep clone を実装。ネストした参照型も含めて完全に複製する。

```
let s2 = clone(s1)  // s1 の deep copy（完全に別のオブジェクト）
// s1 を変更しても s2 は影響を受けない
```

## wasm32 ターゲット: linear memory lowering モデル

ADR-002 補足決定により `--target wasm32` プロファイルを追加。
GC セマンティクスを linear memory 上に lowering して出力する。

---

### 基本方針

「arena ベース + escape/共有のみ RC」の hybrid:

| 値の種類 | メモリ管理 |
|---------|----------|
| 短命な値・一時オブジェクト | arena |
| 関数外へ escape する値 | RC に昇格 |
| 複数箇所から共有される値 | RC に昇格 |
| クロージャ環境 | RC に昇格 |

---

### arena アロケータ

```
[arena header]
┌──────────────────────┐  <- arena_base
│ next_offset: i32     │
│ capacity: i32        │
├──────────────────────┤
│ object 0             │
│ object 1             │
│ ...                  │
└──────────────────────┘  <- arena_base + capacity
```

- `arena_alloc(size) -> i32`: bump pointer を進めてアドレスを返す
- 実行単位ごとに region を確保（典型: main 全体で 1 arena）
- オプション: 「関数内一時領域 + 昇格領域」の二層構成
- プログラム終了時に一括解放（解放コード不要）

---

### RC オブジェクトレイアウト

```
┌──────────────────────┐  <- rc_ptr
│ ref_count: i32       │  +0
│ data ...             │  +4
└──────────────────────┘
```

- `rc_inc(ptr)` / `rc_dec(ptr)`: 参照カウント操作
- `rc_dec` がカウント 0 になったとき `free()` を呼ぶ
- `free()` の実装: `wasm_allocator` モジュール提供（bump alloc は解放なし、RC には linked free list）

---

### 型ごとの lowering

#### struct

```wasm
;; wasm-gc
(struct.new $Point (f64.const 1.0) (f64.const 2.0))

;; wasm32 (arena)
;; arena_alloc(16) -> ptr
;; f64.store ptr+0, 1.0
;; f64.store ptr+8, 2.0
```

#### String（wasm32）

```
┌──────────┬──────────┐
│ ptr: i32 │ len: i32 │  <- 8 bytes のヘッダ
└──────────┴──────────┘
  ↓
┌────────────────┐
│ UTF-8 bytes... │  <- ptr が指す先
└────────────────┘
```

#### Vec\<T\>（wasm32）

```
┌──────────┬──────────┬──────────┐
│ ptr: i32 │ len: i32 │ cap: i32 │  <- 12 bytes
└──────────┴──────────┴──────────┘
```

grow 時は RC alloc で新バッファを確保し旧バッファを解放。

#### クロージャ（wasm32）

```
┌─────────────┬─────────────┐
│ fn_ptr: i32 │ env_ptr: i32│  <- 8 bytes、env は RC に昇格
└─────────────┴─────────────┘
```

#### Option / Result（wasm32）

| 型 | lowering |
|----|---------|
| `Option<i32>` | `(i32 tag, i32 value)` の 2 ワード |
| `Option<ref T>` | `i32` ポインタ（0 = None） |
| `Result<T, E>` | `(i32 tag, T ok, E err)` |

---

### 言語設計制約（lowering 不可機能）

| 除外機能 | 理由 |
|---------|------|
| finalizer の実行タイミング保証 | arena では解放タイミングが不定 |
| `Weak<T>` | GC の到達可能性に依存（当面禁止） |
| 循環参照グラフのユーザー作成 | RC ではリーク |

---

## 関連

- ADR-002: この文書の根拠
- `docs/stdlib/core.md`: String / Vec の API 詳細
- `docs/platform/abi.md`: FFI 境界でのメモリ表現
