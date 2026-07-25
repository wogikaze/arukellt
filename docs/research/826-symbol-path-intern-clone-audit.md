# #826 — Symbol / path interning + hot-path clone audit

ステータス: Phase 1 inventory + bounded NameIndex win  
日付: 2026-07-26  
関連: [#826](../../issues/open/826-symbol-path-intern-clone-audit.md)、[計画](../plans/826-symbol-path-intern-clone-audit.md)、[#823](../../issues/done/823-selfhost-compile-latency-quadratic-mir.md)、[#829](../../issues/done/829-selfhost-latency-phase-reprofile-hotspot.md)、[#827](../../issues/done/827-phase-arena-after-heap-model.md)、[`selfhost-phase-arena-ownership.md`](selfhost-phase-arena-ownership.md)

## スコープ

- CoreHIR → MIR → Wasm 経路の identifier / callee / type_name / path の重複と `clone(` サイト
- arena 実装はしない（#827）
- 公開 `InternedString` API は当面導入しない

## 1. Hot clone inventory（静的）

計画対象ファイルの `clone(` 出現数（2026-07-26 worktree）:

| ファイル | `clone(` | 役割 / 呼び出し経路 |
|---|---:|---|
| `mir/post_pass_callee_lookup.ark` | 155 | `lower.propagate` fixpoint。CALL ごとに callee 名を多数の `eq(clone(name), …)` / `starts_with` / `ends_with` で分類 |
| `wasm/sections_imports.ark` | 48 | emit import 節。`wasi_module` / `io_module` などを import 行ごとに `clone` |
| `mir/module_host_calls.ark` | 43 | host/WASI callee 判定。alias 列を `eq(clone(callee), …)` で線形比較 |
| `wasm/code_ref_locals_typename.ark` | 36 | `emit.code.locals`。`type_name` を prefix/eq で GC 型推論（#829 後も残る） |
| `mir/post_pass_type_propagate.ark` | 13 | LOCAL_SET / temporary への type_name sync |
| `mir/post_pass_callee_cache.ark` | 5 | `CalleePropCache` 構築・lookup。NameIndex キー挿入と return type 返却 |
| `corehir/core_op_registry.ark` | 2 | canonical_id → handler lookup（低頻度） |

追加の構造ホットスポット（計画外だが計測上重要）:

| ファイル | メモ |
|---|---|
| `collections/name_index.ark` | 旧実装は lookup probe ごとに `clone(name)` + `eq`。CalleePropCache / emit NameIndex / type_name→ref cache の共通基盤 |
| `std/collections/string.ark` | `eq` / `starts_with` / `ends_with` が `String` を値で受け、呼び出し側が再利用のために `clone` を強いられる |

### 呼び出し経路（要約）

```text
lower.propagate
  → mir_module_propagate_local_types
  → CalleePropCache build (NameIndex insert × fn_count)
  → per-function fixpoint ≤8
      → MIR_CALL: post_pass_callee_lookup
          → name_index_get(by_name/by_bare)   // NameIndex probes
          → is_*_intrinsic(clone(callee)) × many

emit.code.locals / insts
  → SelfEmitCtx NameIndex (function / struct / type_name_ref)
  → code_ref_locals_typename(type_name)       // prefix clones

emit imports
  → sections_imports: clone(module_name) per import row
```

支配的なパターンは **(A) NameIndex probe clone** と **(B) predicate 列の `eq(clone(s), lit)`**。  
(B) はサイト数が多いが、`String` 借用比較 API か intern id 比較がない限り局所修正では消えにくい。

## 2. Intern table 所有権・ライフタイム（提案）

`selfhost-phase-arena-ownership.md` に合わせる。

| 項目 | 提案 |
|---|---|
| Owner | Compile session（driver / `SelfEmitCtx` 相当の session record） |
| Storage | **session-durable bump**（phase arena reset 対象外） |
| Key | UTF-8 identifier / type_name / module path / canonical_id |
| Handle | `i32` symbol（`InternedString` は段階導入。当面は table + index） |
| 挿入 phase | 最初に文字列が安定した時点（resolve / lower 入口が候補） |
| 読み出し | MIR propagate、emit NameIndex、core_op registry |
| Reset | compile 終了時のみ。parse/typecheck/lower/emit 境界では reset **しない** |
| #827 との関係 | phase arena 導入後も intern table は durable 側に残す |

既存の `std::collections::compiler::interner_*` は線形 `Vec` で O(n) lookup のため、selfhost 規模の正本にはしない。  
`NameIndex`（String→i32）を **値インターナー**（i32→String + String→i32）へ拡張するか、専用 `SymbolTable` を session に置くのが次段。

推奨移行順:

1. NameIndex probe の clone 除去（本監査で実施）
2. callee / type_name の比較を symbol id 化（propagate / typename）
3. `MirInst.str_val` / `MirLocal.type_name` を段階的に id 化（#824 と構造体競合に注意）

## 3. Bounded before/after（NameIndex probe）

### 変更

`src/compiler/collections/name_index.ark` の `name_index_find_slot`:

- 旧: `name_index_hash(clone(name))` + probe ごと `eq(..., clone(name))`
- 新: FNV をインラインし、probe は `char_at` バイト比較（lookup 中の deep-clone ゼロ）

insert/set がキー所有のために 1 回 `clone(name)` する経路は残す。

### 計測プロトコル

Workload: flat-src `compile src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2 --time`  
Host: KEEP_CLOCK `arukellt-s2-clock.wasm`（before）→ `build-compiler` 後の artifact（after）。

結果表は計測完了後に追記する（下記 §3.1）。

### `clone_calls` KEEP_CLOCK 拡張について

計画 Phase 1 の phase 別 `clone_calls` / `clone_bytes` カウンタは **defer**:

- `raw_string_clone` / `__core_string_clone_impl` へのグローバル計装が必要
- overlay KEEP_CLOCK と別軸の ABI/validate リスクがある
- 本レーンは計測済み NameIndex 経路の削減を優先（推測 wrapper を増やさない）

## 3.1 Measurement results

Fair A/B（同一 worktree、`ARUKELLT_OVERLAY_KEEP_CLOCK=1` の `build-compiler` →
`arukellt-s2-runtime.wasm`、flat-src `main.ark` / `wasm32-gc` / `wasi-p2`）。  
旧 `find_slot` で 1 回 rebuild+計測 → 新実装で 1 回 rebuild+計測。

| | wall | peak RSS | propagate | emit | total (guest) |
|---|---:|---:|---:|---:|---:|
| before (probe clone) | 97.07 s | 1199 MiB | 3023 ms | 47285 ms | 96465 ms |
| after (inline compare) | 76.20 s | 1199 MiB | 2095 ms | 40889 ms | 75867 ms |
| Δ | **−21.5%** | ≈0 | **−30.7%** | −13.5% | −21.4% |

成功基準（計画）の wall −5% を満たす。RSS ピークはほぼ不変（deep-clone 削減が
bump 総量に効いても、この workload では他割当が支配的）。

注: `lower.reachability` も 34.3 s → 20.2 s と動いたが、NameIndex 主経路ではない。
ホスト負荷ノイズの可能性が高い。判定の主信号は **propagate** と **host wall**。

`clone_calls` KEEP_CLOCK 拡張は defer（§3）。lookup 経路の deep-clone は
「hash 用 1 + probe ごと 1」→「0」に静的削減。

## 4. 残作業

- [ ] callee_lookup の predicate 列を intern id または単一比較ヘルパへ（借用 `eq` が先決かも）
- [ ] session `SymbolTable` 設計を ADR/RFC なしの実装 issue に分割
- [ ] KEEP_CLOCK `clone_calls` 計装（別 PR、validate ゲート付き）
- [ ] #824 early body lowering 後に Mir 構造体フィールドの id 化を再評価

## 5. Non-goals（確認）

- phase arena コードなし
- AST cache (#825) 非対象
- 公開 InternedString API 非対象
