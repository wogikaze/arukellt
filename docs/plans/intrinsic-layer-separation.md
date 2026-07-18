# Intrinsic 層分離 移行計画

ステータス: 実装計画（決定記録ではない）
関連 ADR: ADR-042

---

## ゴール

callee 文字列 dispatch を廃止し、semantic stdlib / runtime ABI / target intrinsic の
責務分離を完了する。emitter から stdlib 操作の実装本体を除去する。

## 移行段階

#798 は第 0〜5 段階の dispatch spine を担当する。第 6 段階の production
lowering、Ark fallback、differential proof は #819〜#822 が担当し、第 7 段階の
production readiness と `status = "production"` gate は #818 が担当する。
#816 と #817 は #729 の別 child であり、#798 の完了条件には含めない。

### 第 0 段階: 新しい callee 文字列 dispatch の凍結

- 移行開始後、新しい callee 文字列 dispatch を追加しない。
- 新規 operation が必要な場合は、`data/core-ops.toml` への entry と、
  既存文字列 dispatch への legacy compatibility mapping を同時に追加する。
- 既存の文字列 dispatch は削除しない。dispatch 削除は第 5 段階で行う。

### 第 1 段階: CoreOpRegistry schema と SignatureEntry 拡張

- `data/core-ops.toml` を schema_version = 4 に上げる。
- `SignatureEntry` は `core_op_id`（optional）と function signature のみを持つ。
  `CoreOpId` の metadata（effect、lowering、fallback、inline policy、semantics）
  は `data/core-ops.toml`（CoreOpRegistry）から引く。
- `CoreOpId` metadata は `SignatureEntry` へ複製しない。compiler cache は derived
  ビューであり、authoritative データではない。
- `exposure` を三軸に分離する:
  - `visibility` (`public` / `internal`)
  - `classification` (`layer` = `primitive` / `runtime` / `semantic_stdlib` / `target_raw`)
    `normal_stdlib` は CoreOpRegistry には登録せず、移行後に `core_op_id` を削除する。
  - `binding` (`policy` = `required` / `optional` / `forbidden`)
- `signature` は receiver 非依存の `inputs` 列に統一し、`receiver_index` で receiver を示す。
- 型式は `TypeExpr` として `kind` discriminator で表す。`String` → `type_id = "string"`、
  `()` → `unit` primitive、`std::wasm` の `v128` → `type_id = "wasm.v128"` に正規化する。
- lowering variant ごとに必要な payload を定義する:
  - `normal_call` — `[fallback]` の `implementation_symbol`
  - `mir_op` — `[lowering.mir]` の `opcode` / `operation`
  - `runtime_call` — `[lowering.runtime]` の discriminated union:
    - `kind = "internal"` — `symbol` + `abi_version`
    - `kind = "wit"` — `package` + `interface` + `function` + `version`
    - `kind = "native"` — `backend` + `symbol` + `abi_version`
  - `target_intrinsic` — `[lowering.target]` の `target_family` / `target_id` / `required_capabilities` / `required_target_features`。`target_id` は backend-owned handler key
- portable `std::simd` 操作は `target_intrinsic` ではなく `normal_call` + `specializations` とする。
- specialization は `priority`、`when` 条件、完全な `lowering` variant を含む。
- この段階では既存の callee 文字列 dispatch をそのままにしておく。
  既存の挙動を変えない。

### 第 2 段階: 全 builtin/intrinsic への CoreOpId と LoweringKind 割り当て

- 現在の全 builtin / intrinsic / runtime ABI operation に `CoreOpId` と
  `LoweringKind` を割り当てる。
- `std/manifest.toml` の `[[types]]` には `type_id`、`[[functions]]` には `core_op_id`
  を追加し、`data/core-ops.toml` を参照する。
- 同じ `CoreOpId` を持つ複数 public binding（prelude alias と `std::*` 等）が
  一貫していることを確認する。
- 割り当てた mapping を手作業で検証する。
- 同時に、旧文字列 dispatch と新 registry dispatch の結果を比較する shadow mode を
  実装し、最終的には一致率 100% を目指す。一致しない場合は mapping を修正する。
- 旧 dispatch と新 dispatch の比較は、`lowering.kind` ではなく capability 解決後の
  `EffectiveLoweringDecision` 同士を比較する。

### 第 3 段階: CoreOpRegistry と既存文字列 dispatch の shadow 検証

- generator / checker が `data/core-ops.toml` と `std/manifest.toml` の参照整合性、
  signature 互換、effect/lowering 整合、binding policy 違反、
  public binding の衝突、specialization ambiguity、fallback 条件を検査する。
- 検査は `visibility` / `classification` / `binding` に応じて条件を変える。
  `visibility = "public"` かつ `binding.policy = "required"` の operation のみ
  manifest 参照を要求する。
- shadow mode を CI または `verify quick` の一部として実行し、
  旧文字列 dispatch と新 registry dispatch の結果が一致率 100% になることを gate にする。
- 検証を通過するまで第 4 段階に進めない。

### 第 4 段階: emitter を FunctionId 経由の CoreOpRegistry lookup へ切り替える

- `inst_dispatch.ark` を、MIR に保存された `FunctionId`
  から `SignatureRegistry` を参照する形に書き換える。
- `CoreOpId` / `LoweringKind` によって dispatch する。
- `func_id_raw` は `FunctionId` の物理表現にすぎず、raw 値そのものを
  意味判定に使用しない。
- 切り替え gate は shadow mode の一致率 100% とする。

### 第 5 段階: 文字列 dispatch と callee 別名処理を削除する

- `eq(clone(callee), "...")` 比較を `call_*.ark` から完全に削除する。
- `normalize_callee_name`、callee 別名処理、`__intrinsic_` prefix stripping を削除する。
- この時点まで intrinsic 追加は凍結する（新規操作は `data/core-ops.toml` へ追加する）。
- resolver 境界で必要な互換 alias は `legacy_bindings` として固定し、backend
  dispatch から分離する。削除は production exit の #818 で行う。

### 第 6 段階: runtime ABI 分離、inliner、stdlib 移行

実装 owner:

- runtime ABI / WIT lowering: #819（HTTP/sockets の標準化は #727 の成果を利用）
- stdlib-only inliner: #820（#816 の compiled prelude を利用）
- pure operation の Ark stdlib 移行: #821
- allocation / Vec / String representation 依存 operation の Ark stdlib 移行: #822（#817 を利用）

- host intrinsic（HTTP、fs、sockets、clock、random、process、stdio、env）を
  emitter から外し、runtime ABI / WIT import lowering へ統合する。
- 小さな stdlib 専用 inliner を導入する（compiler-shipped core/std だけ、
  再帰なし、MIR 命令数・code size 制限付き）。`inline.policy` は
  `always` を強い hint として解釈する。
- pure operation（`gcd`、range 操作、`starts_with`/`ends_with`/`contains`/`index_of`、
  `trim`、`reverse`、any/find/fold、sort 等）を Ark stdlib へ移す。
- allocation-dependent 操作（`split`、`join`、`replace`、`repeat`、`pad_left/right`、
  `lines`、HashMap/HashSet、数値 parse/format 等）を Ark へ移す。
- 各移行には differential test（Ark fallback vs 旧 intrinsic）を伴う。

### 第 7 段階: 検証と cleanup

最終 gate owner: #818。#819〜#822 の実装を #818 へ再集約しない。

- `data/core-ops.toml` / `std/manifest.toml` / `docs/current-state.md` の整合を再確認する。
- 移行対象の targeted tests が pass し、T3 validation 失敗数が #808 baseline を
  超えないことを確認する。
- `python3 scripts/manager.py verify quick` は #808 解決後の最終 epic close で
  global green を目指す。
- GC/LM 二重 intrinsic ファイルの整理と sealed raw API 経由への移行は、
  本計画のスコープ外として [#817](../../issues/done/817-sealed-raw-api-module.md) で実施する。

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
FunctionId だけを知る。public `panic` 関数は `core_op_id = "panic"` で
`data/core-ops.toml` の runtime_call entry を参照する。

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

- raw Wasm SIMD 命令（`std::wasm`）
- relaxed SIMD
- atomics
- target feature detection
- Wasm-specific reference/table operations

portable `std::simd` の固定 SIMD 算術（`I32x4.add` 等）は `normal_call` + specializations
であり、target intrinsic ではない。

---

## #798 のスコープ外

以下は ADR-042 で「別 ADR / RFC が必要」とされているため、#729 の
別 child として進め、#798 の dispatch-spine 完了条件には含めない。

- **prelude のコンパイル対象復帰**: `combine_loaded_and_main_decls_skip_prelude` の
  廃止と prelude 本体の backend 通過。詳細は
  [RFC-005](../rfcs/005-prelude-compilation-restoration.md)（ACCEPTED）。
  実装は [#816](../../issues/done/816-prelude-compilation-restoration.md)。
- **sealed raw API のモジュール名と公開面**: [RFC-006](../rfcs/006-sealed-raw-api.md)
  で `core::raw` を採択。実装は
  [#817](../../issues/done/817-sealed-raw-api-module.md)。
- **GC/LM 二重 intrinsic ファイルの cleanup**: sealed raw API の実装後に
  [#817](../../issues/done/817-sealed-raw-api-module.md) で実施する（`*_lm`
  二重ファイルは #817 で parent へ吸収済み。残 GC helper は raw 層所有）。

---

## 目標規模（目安）

最終的な目標は個数ではなく責務で決める:

- 真の target-independent primitive: 20〜40 種類
- runtime ABI 分類: 5〜15 種類
- compiler-known CoreOpId: 20〜50 種類
- raw target SIMD: 多数でもよいが、表から自動生成
- portable `std::simd` 操作: 表から自動生成
- 通常 stdlib: 個数制限なし

## 等価性検証

semantic lowering には必ず Ark fallback body を残す。
最適化 ON/OFF、GC/LM、各 target について、Ark fallback 版と
optimized lowering 版を同じ入力で実行し、結果と副作用が一致する
differential test を置く。

CoreOp ごとに `semantics.equivalence` で比較方法を指定する。
整数や `bool` は `exact_bitwise` または `exact_bool`、
浮動小数 SIMD 等は `float_value_nan_payload_ignored`、
`noreturn` operation は `noreturn`、集合結果は `set_order_agnostic` 等。

旧文字列 dispatch から新 `SignatureRegistry` dispatch への移行には、
切り替え前に shadow mode で両者の結果を比較し、一致率 100% を gate とする。
比較対象は `EffectiveLoweringDecision`（capability 解決後の正規化 lowering）とする。

## 検証コマンド

```bash
python3 scripts/manager.py verify quick
```

移行中は targeted migration tests と #808 baseline ratchet も gate とする。

## 関連

- [ADR-042: Intrinsic Layer Separation](../adr/ADR-042-intrinsic-layer-separation.md)
- [ADR-040: Semantic Type Spine](../adr/ADR-040-typed-mir-signature-registry.md)
- [ADR-037: std::simd](../adr/ADR-037-std-simd.md)
- RFC-003: NaN semantics (planned)
- [issue #808: T3/Wasm validation failures](../../issues/open/808-t3-wasm-validation-failures.md)
