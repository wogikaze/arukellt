---
Status: done
Created: 2026-07-15
Updated: 2026-07-25
Closed: 2026-07-25
ID: 722
Track: wasm-feature
Depends on: none
Orchestration class: design-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 3
Source: ADR-008 改訂（2026-07）+ ADR-033 Phase A — Typed Function References は Wasm 3.0 で shipped 済み
---

## Closed — 2026-07-25

Measurement/evaluation slice complete (Phase A/B/C). Production emitter migration is
[#831](../open/831-call-ref-emitter-migration.md). Phase B *implementation* (`br_on_null`)
is deferred until after #831; see Phase B evaluation below.

# Typed Function References (`call_ref`) ベンチマーク計測

## Summary

Typed Function References は Wasm 3.0 で Phase 5 shipped 済み（ADR-008 改訂）。
wasmtime 46、V8 14.6 でデフォルト有効。

現在 Arukellt は `funcref`（untyped）+ `call_indirect` + function table で
クロージャ/HOF を実装している（ADR-033 Baseline）。typed function references
`(ref $func_type)` と `call_ref` 命令は未使用。

本 issue は ADR-033 の段階的移行計画（Phase A/B/C）をトラッキングする。
ADR-033 は「段階移行するという決定」のみを記録し、Phase の詳細は本 issue に委譲する。

## Current state

### クロージャ/HOF の実装

- `src/compiler/wasm/sections_table.ark:12-65` — funcref table + element section
- `src/compiler/wasm/call_indirect.ark:14-22` — `call_indirect` 命令の emit
- `src/compiler/mir/lower/call_indirect_emit.ark:17-34` — 間接呼び出しの MIR lowering

### 移行フェーズ（ADR-033 から委譲）

- **Baseline (now)**: `call_indirect` for all closure/HOF dispatch
- **Phase A (emitter audit)**: HOF call site のうち callee が既知の `ref.func`
  （direct function values, monomorphic callbacks）を特定し、型インデックスが
  静的に分かって table slot が不要な場合に `call_ref` を emit
- **Phase B (nullable refs)**: `Option<fn ...>` / nullable function-reference の
  null チェックを手動比較から `br_on_null` / `br_on_non_null` に切替
  （GC type system が許す場合）
- **Phase C (benchmark gate)**: 代表的な fixture で `call_indirect` vs `call_ref`
  の性能比較を実施し、≥5% improvement で `call_ref` を audited patterns の
  default に採用（issue #069 acceptance benchmark）

現在は Baseline で止まっている。

## What to measure

### 1. call site 分類

HOF / クロージャの call site を以下の3つに分類:

| 分類 | 説定 | 現在の emit | `call_ref` 適用可否 |
|------|------|------------|-------------------|
| **A: 静的直接** | 呼び出し先の関数が静的に分かる（monomorphic callback） | `call_indirect` | ✅ `call_ref` に切替可能 |
| **B: 静的型付き** | 呼び出し先の型は分かるが関数は動的（trait object 相当） | `call_indirect` | ✅ `call_ref` に切替可能（table 不要） |
| **C: 完全動的** | 呼び出し先も型も動的 | `call_indirect` | ❌ `call_indirect` のまま |

### 2. ベンチマーク項目

以下のベンチマークで `call_indirect` (現状) vs `call_ref` (Phase A) を比較:

- `benchmarks/` 配下の HOF を使うベンチマーク（`map`/`filter`/`fold` 等）
- クロージャキャプチャを含むベンチマーク
- 関数ポインタ渡しのベンチマーク

計測環境:
- wasmtime 46（`--invoke` method または CLI）
- V8 14.6 (Node.js 26) — ブラウザ向けの参考値

### 3. 計測指標

- 実行時間（中央値、p99）
- バイナリサイズ（table section 削減効果）
- 型セクションサイズ（typed funcref 型定義の追加）

## Phase A audit results

Call-site classification was performed by compiling representative fixtures to
`wasm32-gc`, disassembling with `wasm-tools print`, and inspecting `call_indirect`
usage.

### Class A: static direct / known `ref.func`

- Top-level function callbacks passed to builtin HOFs (`map_i32_i32`,
  `filter_i32`, `fold_i32_i32`, etc.) are already emitted as **direct calls**.
- Known function values passed to user-defined `fn` parameters are emitted as an
  `i32` table index; with typed function references these would become
  `ref.func` at the call site.
- Count in the current fixture set: **1** (`higher_order.ark` passes `double` to
  `apply`).

### Class B: type known, function dynamic

- User-defined functions that take a `fn(...)` parameter and call it emit
  `call_indirect` with a statically known type index.
- Count in the current fixture set: **1** (`f(x)` inside `apply` in
  `higher_order.ark`).
- Selfhost compiler wasm (`arukellt-s2-runtime.wasm`): **0** `call_indirect`
  instructions (compiler code only emits MIR, it does not execute indirect
  calls at runtime).

### Class C: fully dynamic

- No fixtures or known code paths exercise a function reference whose type is
  also dynamic.
- Count: **0**.

### Prototype

WAT prototypes demonstrating the `call_indirect` baseline and the equivalent
`call_ref` / typed-function-reference version are in
`docs/research/wat-probes/typed-funcref/`:

- `higher_order_call_indirect.wat` — untyped funcref table + `call_indirect`
- `higher_order_call_ref.wat` — `(ref $callback)` parameter + `call_ref`
- `bench_call_indirect.wat` — 10M iteration benchmark, baseline
- `bench_call_ref.wat` — 10M iteration benchmark, prototype

### Phase C microbenchmark (wasmtime 46)

10M iterations of `apply(callback, acc)` on an `i32` callback:

| Variant | median real | median user |
|---|---|---|
| `call_indirect` (table index) | ~0.054 s | ~0.045 s |
| `call_ref` (typed funcref)    | ~0.043 s | ~0.031 s |

`call_ref` is approximately **15–25 % faster** in this microbenchmark, exceeding
the ≥5 % improvement gate.

Binary size of the `higher_order` equivalent:

| Variant | wasm bytes |
|---|---|
| `call_indirect` (table + elem segment) | 140 |
| `call_ref` (no table, declare elem)    | 132 |

The prototype removes the `table` section but adds a `declare` element segment
and a typed function reference in the type section, resulting in a small net
saving for this single-call example.  Programs with many `fn` parameters would
see larger table-section savings, but each typed reference needs a distinct
type index, so type-section growth must also be measured in a real emitter.

Real-world impact will depend on how often the compiler emits `call_indirect`
today; Phase A found only one user fixture that produces it, so total program
speedup from this change alone is expected to be small unless user code uses
`fn` parameters heavily.

## Acceptance criteria

### Phase A (emitter audit)

- [x] HOF / クロージャ call site の分類（A/B/C）が完了する
- [x] 分類 A（静的直接）の call site 数が把握できる
- [x] `call_ref` を emit するプロトタイプ（実験ブランチ）が作成される

### Phase B (nullable refs)

- [x] `Option<fn ...>` / nullable function-reference の null チェック箇所を特定
- [x] `br_on_null` / `br_on_non_null` への切替可否を評価

#### Phase B evaluation (2026-07-25)

**箇所特定**

- `Option<fn ...>` は通常の GC enum（tag + payload）として lower される。
  `None = null` ではない（ADR-035）。
- `std/` / `src/compiler/` に `Option<fn>` の実使用はほぼ無い。
- 試作では `Some(g) => g(5)` の呼び出しが `unreachable` / `drop` に展開され、
  現状は呼び出し自体が正しく emit されていない。

**切替可否**

- `br_on_null` / `br_on_non_null` は nullable typed funcref が前提。
- ADR-035 は Option 全体の `None = null` を禁止しているため、
  `Option<T>` を nullable GC ref に特殊化する方針は採らない。
- 将来の応急候補: bare `fn` を常に `(ref null $sig)` とし、呼び出し前に
  `ref.as_non_null` を emit する（use-site nullability。ADR-035 §5 に寄せられるが、
  言語上の非 null `fn` とはズレる）。
- **推奨:** 本格 `call_ref` emitter（#831）が安定してから Phase B 実装に着手する。
  本 issue の Phase B は評価完了とし、実装は defer。

### Phase C (benchmark gate)

- [x] ベンチマークで `call_indirect` vs `call_ref` の性能比較が完了する
- [x] バイナリサイズの変化が計測される
- [x] ≥5% improvement の判断基準に対する評価が記録される
- [x] 計測結果に基づき、Phase A 移行を進めるか/見送るかの推奨が記載される

#### Phase C recommendation (2026-07-25)

- Microbench（wasmtime 46、10M iter）: `call_ref` は約 **15–25% 高速**（≥5% 超え）。
- Binary size: 140 → 132 bytes（単一 call site の WAT プロトタイプ）。
- 実プログラム効果は小さい見込み（fixture の `call_indirect` は実質
  `higher_order.ark` のみ；s2 compiler wasm は 0）。
- **GO:** Class A/B の audited patterns を本番 emitter で `call_ref` に移行する
  → [#831](../open/831-call-ref-emitter-migration.md)。

## Note

- 本 issue は ADR-033 から委譲された Phase A/B/C の**計測・評価**が目的。
  本格的な emitter 変更は [#831](../open/831-call-ref-emitter-migration.md)。
- `call_ref` に移行しても `call_indirect` は完全には削除できない（分類 C のため）
- table section は分類 C が残る限り必要だが、サイズは削減される可能性がある

## Related

- ADR-008: WasmGC Post-MVP 拡張機能（#5 Typed Function References）
- ADR-033: クロージャ呼び出しを call_ref に移行
- ADR-002: Memory Model (Wasm GC 採用)
- ADR-007: コンパイルターゲット整理（wasm32-gc）
- #831: call_ref emitter 移行（本計測の follow-up）
