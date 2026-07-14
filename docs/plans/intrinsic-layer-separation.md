# Intrinsic 層分離 移行計画

ステータス: 実装計画（決定記録ではない）
関連 ADR: ADR-042

---

## ゴール

callee 文字列 dispatch を廃止し、semantic stdlib / runtime ABI / target intrinsic の
責務分離を完了する。emitter から stdlib 操作の実装本体を除去する。

## 移行段階

### 第 1 段階: registry schema と SignatureEntry 拡張

- `core-ops.toml` を `data/core-ops.toml` として正規の production path に配置し、
  schema_version を上げる。
- `SignatureEntry` に `semantic_id`、effect（直交属性セット）、`const semantics`、
  `inline_policy`、`lowering_kind`、target、fallback を追加する。
- effect モデルは `pure` / `read` / `write` / `allocate` / `IO` / `noreturn` 等の
  単一 enum にせず、memory / allocates / may_trap / noreturn / external_io /
  nondeterminism / atomic / volatile の直交属性セットとする。
- この段階では既存の callee 文字列 dispatch をそのままにしておく。
  既存の挙動を変えない。

### 第 2 段階: 全 builtin/intrinsic への SemanticId と LoweringKind 割り当て

- 現在の全 builtin / intrinsic / runtime ABI operation に `SemanticId` と
  `LoweringKind` を割り当てる。
- `std/manifest.toml` の `[[types]]` には `type_id`、`[[functions]]` には `semantic_id`
  を追加し、`core-ops.toml` を参照する。
- 割り当てた mapping を手作業で検証し、既存文字列 dispatch と同じ dispatch 結果に
  なることを確認する。

### 第 3 段階: registry と既存文字列 dispatch の対応検証

- generator / checker が `core-ops.toml` と `std/manifest.toml` の参照整合性、
  signature 互換、effect/lowering 整合、未参照 semantic op、重複 public binding を
  検査するよう整備する。
- 検証を通過するまで次の段階に進めない。

### 第 4 段階: emitter を FunctionId 経由の registry lookup へ切り替える

- `call_dispatch_table.ark` / `inst_dispatch.ark` を、MIR に保存された `FunctionId`
  から `SignatureRegistry` を参照する形に書き換える。
- `LoweringKind` / `SemanticOpId` によって dispatch する。
- `func_id_raw` は `FunctionId` の物理表現にすぎず、raw 値そのものを
  意味判定に使用しない。

### 第 5 段階: 文字列 dispatch と callee 別名処理を削除する

- `eq(clone(callee), "...")` 比較を `call_*.ark` から完全に削除する。
- `normalize_callee_name`、callee 別名処理、`__intrinsic_` prefix stripping を削除する。
- この時点で intrinsic 追加は完全に凍結する（新規操作は `core-ops.toml` へ追加する）。

### 第 6 段階: runtime ABI 分離、inliner、stdlib 移行

- host intrinsic（HTTP、fs、sockets、clock、random、process、stdio、env）を
  emitter から外し、runtime ABI / WIT import lowering へ統合する。
- 小さな stdlib 専用 inliner を導入する（compiler-shipped core/std だけ、
  再帰なし、MIR 命令数・code size 制限付き）。
- pure operation（`gcd`、range 操作、`starts_with`/`ends_with`/`contains`/`index_of`、
  `trim`、`reverse`、any/find/fold、sort 等）を Ark stdlib へ移す。
- allocation-dependent 操作（`split`、`join`、`replace`、`repeat`、`pad_left/right`、
  `lines`、HashMap/HashSet、数値 parse/format 等）を Ark へ移す。
- 各移行には differential test（Ark fallback vs 旧 intrinsic）を伴う。

### 第 7 段階: 検証と cleanup

- GC/LM 二重 intrinsic ファイルを削除する（representation 違いは sealed raw API に吸収）。
- `intrinsic_*_gc.ark` / `intrinsic_*_lm.ark`  dual files を整理する。
- `core-ops.toml` / `std/manifest.toml` / `docs/current-state.md` の整合を再確認する。
- `python3 scripts/manager.py verify quick` が通ることを確認する。

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

## 本計画のスコープ外

以下は ADR-042 で「別 ADR / RFC が必要」とされているため、
本計画の各段階に含めない。

- **prelude のコンパイル対象復帰**: `combine_loaded_and_main_decls_skip_prelude` の
  廃止と prelude 本体の backend 通過。専用 RFC 依存の子 issue として追跡する。
- **sealed raw API のモジュール名と公開面**: `core::raw` / `core::intrinsics` / `core::rt`
  等の最終決定。専用 RFC 依存の子 issue として追跡する。

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
