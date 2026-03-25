# 公開 ABI 方針

ADR-006 の内容を展開する。

---

## 3層構造（確定）

### Layer 1: 内部 ABI（非公開）

- arukellt コンパイラ独自
- バージョン間の互換性保証なし
- コンパイラ実装の都合で変えてよい
- 関数呼び出し規約、スタックフレーム構造、レジスタ割り当て方針

### Layer 2: WASM 公開 ABI

v0 での基本（2つの公開面を併用）:
- **Layer 2A: raw Wasm ABI**
  - import: `(import "module" "name" (func ...))`
  - export: `(export "name" (func ...))`
  - 数値型は Wasm value type そのまま（i32, i64, f32, f64）
  - 複合型（struct, enum）は linear memory / GC heap 上のポインタで渡す
- **Layer 2B: Component Model / WIT ABI（WASI p2）**
  - WIT で world/interface を公開
  - canonical ABI に従って値を lower/lift する
  - jco 等の component toolchain で配布可能な surface を提供する

運用ルール:
- ADR-006 に従い、2A/2B はどちらも Layer 2 の拡張として扱う
- 独立した第4層は作らない
- 言語の意味論は 2A/2B で一致させる

### Layer 3: native 公開 ABI

- C ABI 相当のみ
- POSIX / Windows の差異は platform 抽象層が吸収
- arukellt 独自の拡張は入れない
- LLVM IR バックエンドからのみ使用（ADR-005）

---

## 型の ABI 表現（wasm32, v0）

ADR-002 により **Wasm GC 採用**。

### Layer 2（Wasm 公開 ABI）

Layer 2A（raw Wasm）では GC 参照を直接使用:

| arukellt の型 | Wasm GC 表現 |
|--------------|-------------|
| `i32` | `i32` |
| `i64` | `i64` |
| `f32` | `f32` |
| `f64` | `f64` |
| `bool` | `i32`（0 = false, 1 = true） |
| `char` | `i32`（Unicode scalar value） |
| `struct` | `(ref $struct_type)` |
| `enum` | `(ref $enum_type)`（tag + payload） |
| `String` | `(ref $string)` |
| `Vec[T]` | `(ref $vec_T)` |
| `Option[T]` | `(ref null $T)` または tagged struct |
| `Result[T, E]` | `(ref $result_T_E)` |

Layer 2B（WIT/canonical ABI）は、上記と同じ型意味論を canonical ABI の lower/lift で表現する。

### Layer 3（native 公開 ABI）

C ABI では GC 参照を直接渡せない。linear memory 経由でコピー:

| arukellt の型 | C ABI 表現 |
|--------------|-----------|
| `i32` | `int32_t` |
| `i64` | `int64_t` |
| `f32` | `float` |
| `f64` | `double` |
| `bool` | `int32_t` |
| `String` | `(ptr, len)` — linear memory にコピー |
| `struct` | `ptr` — linear memory にコピー |
| `Vec[T]` | `(ptr, len)` — linear memory にコピー |

詳細は `docs/design/gc-c-abi-bridge.md` を参照。

---

## multi-value の利用

Wasm multi-value を使うと、複数の戻り値を直接返せる。

```wasm
;; (i32, i32) を返す関数
(func $split (result i32 i32)
  i32.const 10
  i32.const 20)
```

arukellt では tuple / 2要素以下の Result をこの形で lowering する方向で検討する。詳細は `toolchain/compiler-phases.md` で設計する。

---

## FFI の呼び出し規約

### arukellt → WASI

- wasm32 ターゲット（AtCoder 向け）は WASI p1 import を直接呼ぶ
- wasm-gc ターゲットは Component Model/WIT（WASI p2）経由を正規面として持つ
- どちらも `platform/wasi-resource-model.md` の capability 設計に従う

### arukellt → 外部 C ライブラリ（native バックエンドのみ）

C ABI（System V AMD64 / Windows x64）に従う。arukellt 側で `extern "C"` 相当の宣言が必要（構文は未設計）。

### 外部 → arukellt の export

export した関数は C ABI 相当で呼び出せる。複合型はポインタ渡し。
