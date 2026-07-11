# Intrinsic 層分離 移行計画
ステータス: 実装計画（決定記録ではない）
関連 ADR: ADR-042

---

## ゴール

callee 文字列 dispatch を廃止し、semantic stdlib / runtime ABI / target intrinsic の
責務分離を完了する。emitter から stdlib 操作の実装本体を除去する。

## 移行段階

### 第 1 段階: intrinsic 追加を凍結

新規の文字列 dispatch を禁止する。例外は target-specific SIMD のみ。
同時に、すべての呼び出し判定を callee 名ではなく `func_id_raw` と
`SemanticId` へ移す。

### 第 2 段階: host intrinsic を runtime ABI へ分離

HTTP、fs、socket、clock、random、process、stdio を emitter から外す。
インライン化とは無関係なので先にできる。
WIT/import lowering の汎用機構に統合する。

### 第 3 段階: semantic registry を作る

SignatureEntry に semantic ID、effect、may trap、const evaluable、
inline policy、lowering policy、fallback body を追加する。
manifest / ops 表（候補: `std/manifest.toml` または `core-ops.toml`）を
単一の正本にして、各種 compiler データを生成する。

### 第 4 段階: 小さな stdlib 専用 inliner

最初は一般的な高度 inliner でなくてよい:

- compiler-shipped core/std だけ対象
- 再帰なし
- MIR 命令数が小さい
- 単一 basic block または単純 CFG
- `@inline(always)` または cost threshold 以下
- target ごとの code size 上限あり

semantic operation は早期にはインライン化せず高水準最適化に使い、
後段でインライン化する設計が理想。

### 第 5 段階: pure operation を Ark へ移す

まず `gcd`、range 操作、trim start/end、starts/ends with、contains/index_of、
reverse、any/find/fold、sort から移す。
split、replace、format、HashMap などは、allocation や representation の設計が
安定してから移す。

### 第 6 段階: prelude のコンパイル対象復帰

prelude 本体を本当にコンパイル対象に戻し、偽の関数本体を廃止する。
**要別 ADR / RFC** — 本段階の詳細は別文書で決定する。

---

## 各機能の移動先

### Compiler/MIR に残すもの (言語プリミティブ)

- GC object/array の生成
- array length
- unchecked array get/set
- GC cast/test
- 必要な write barrier
- raw linear-memory load/store
- `memory.copy`, `memory.fill` (MIR 命令として)
- trap/unreachable
- function reference, indirect call
- 本当に表現不可能な bit cast
- target-specific SIMD（`std::wasm` 経由の raw 命令）

`memory.copy` 等は「名前付き関数を emitter で検出」するのではなく、
MIR 命令にする。

### Runtime ABI へ移すもの

- allocation/reallocation
- process exit/abort
- panic handler
- stdin/stdout/stderr
- filesystem
- clocks
- random
- HTTP
- sockets
- streams
- environment variables

WASI 系は、コンパイラが `fs_read_file` や `http_get` を個別に知るのではなく、
汎用的な WIT/import lowering だけを知るべき。
`std::fs::read_to_string` は `Ark stdlib → WIT binding/import → host` 経路にする。

`panic` はコンパイラが bounds check 失敗などから呼ぶため、Rust の lang item に
近い扱いとする。実装は runtime に置き、コンパイラは `PanicHandler` という
FunctionId だけを知る。

### Ark stdlib へ移すもの

- `starts_with`, `ends_with`, `contains`, `index_of`
- `trim`, `replace`, `split`, `join`, `repeat`, `pad_left/right`, `lines`
- `to_upper/lower`
- `sort`, `reverse`
- `map/filter/fold/find/any`
- `range_new/contains/len`
- `gcd`
- HashMap/HashSet 本体
- 数値 parse/format

`map_i32_i32`, `map_i64_i64`, `map_f64_f64` のようなモノモーフィック intrinsic 群は
generic/trait 実装へ統合する。

### Target intrinsic として残すもの

- Wasm SIMD 命令（portable: `std::simd`、raw: `std::wasm`）
- relaxed SIMD
- atomics
- target feature detection
- Wasm-specific reference/table operations

---

## 目標規模（目安）

最終的な目標は個数ではなく責務で決める:

- 真の target-independent primitive: 20〜40 種類
- runtime ABI 分類: 5〜15 種類
- compiler-known semantic ID: 20〜50 種類
- target SIMD: 多数でもよいが、表から自動生成
- 通常 stdlib: 個数制限なし

## 等価性検証

semantic lowering には必ず Ark fallback body を残す。
最適化 ON/OFF、GC/LM、各 target について、Ark fallback 版と
optimized lowering 版を同じ入力で実行し、結果と副作用が一致する
differential test を置く。

## 検証コマンド

```bash
python3 scripts/manager.py verify quick
```

## 関連

- [ADR-042: Intrinsic Layer Separation](../adr/ADR-042-intrinsic-layer-separation.md)
- [ADR-040: Semantic Type Spine](../adr/ADR-040-typed-mir-signature-registry.md)
- [ADR-037: std::simd](../adr/ADR-037-std-simd.md)
