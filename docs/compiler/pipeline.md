# コンパイルパイプライン

> **Current reality first**: 実装の現在地は [../current-state.md](../current-state.md) を基準にしてください。
> このページは、現在の主要経路と、repo が保持している設計境界を整理するための補助資料です。

## 現在の主要経路

```text
source (.ark)
  → ark-lexer
  → ark-parser
  → ark-resolve
  → ark-typecheck
  → ark-mir
  → ark-wasm
```

現在の user-visible な主経路は `Lexer → Parser → Resolver → TypeChecker → MIR → Wasm` です。

- `ark-driver::Session` が共有 orchestration の入口
- `arukellt` が `check` / `compile` / `run` を提供
- `check` は frontend diagnostics まで
- `compile` / `run` は backend validation まで進む
- `wasm32-wasi-p1` は互換パス、`wasm32-wasi-p2` は canonical GC-native パス
- `--emit component` / `--emit wit` は `wasm32-wasi-p2` 側の追加出力

## 設計上の境界

repo 内では、より細かい段階名も引き続き使います。

```text
Lex
  → Parse
  → Bind
  → Load
  → Analyze
  → Resolve
  → Check+BuildCoreHIR
  → LowerToMIR
  → MIRValidate
  → MIROptimize
  → BackendPlan
  → WasmEmit / LLVMEmit
  → BackendValidate
```

これは「今すぐ全部が独立した public surface」という意味ではなく、責務分割の目印です。

## MIR lowering: legacy fallback state (2026-04-15)

The `LowerToMIR` step is currently implemented entirely via the **legacy AST lowerer**
(`lower_to_mir` in `crates/ark-mir/src/lower/func.rs`). The CoreHIR lowerer
(`lower_hir_to_mir`) is a placeholder returning empty MIR; every compilation therefore
falls back to the legacy path.

- `MirSelection::Legacy` and `OptimizedLegacy` are **deprecated** (since 0.1.0)
- The `test` command compiles with `MirSelection::OptimizedCoreHir` unconditionally
  (both CoreHir and Legacy currently use legacy internally)
- Removal of `lower_to_mir` is blocked until `lower_hir_to_mir` is implemented:
  see [legacy-path-status.md](legacy-path-status.md) and
  `issues/open/508-legacy-path-removal-unblocked-by.md`
- Migration examples and warning behavior are documented in
  [legacy-path-migration.md](legacy-path-migration.md)

## 現在の crate map

- `crates/ark-lexer`
- `crates/ark-parser`
- `crates/ark-resolve`
- `crates/ark-typecheck`
- `crates/ark-hir`
- `crates/ark-mir`
- `crates/ark-wasm`
- `crates/ark-target`
- `crates/ark-diagnostics`
- `crates/ark-driver`
- `crates/ark-stdlib`
- `crates/ark-lsp`
- `crates/ark-llvm`
- `crates/arukellt`

## Session / Artifact Graph 方針

`ark-driver::Session` は artifact / query 指向の境界をまとめるハブです。
現在参照される主な段階名:

- `parse()`
- `bind()`
- `load_graph()`
- `analyze()`
- `resolve()`
- `check_core_hir()`
- `lower_mir()`
- `opt_mir()`
- `plan_backend()`
- `emit_wasm()`

旧 API や移行途中の境界が残っていても、現挙動の判定は `docs/current-state.md` と実装コードを優先してください。

## MIR / Backend Optimization

`--opt-level` controls the optimization pipeline (default: 1).

| Level | MIR | Backend | Dead Function Elimination |
|-------|-----|---------|--------------------------|
| `0` | None | None | Disabled |
| `1` | 9 safe passes (const_fold, branch_fold, cfg_simplify, copy_prop, const_prop, dead_local_elim, dead_block_elim, unreachable_cleanup, cse) | peephole (`local.tee`) | Enabled |
| `2` | All 20 passes × up to 3 rounds + inter-function inline | peephole + struct field layout reorder | Enabled |

MIR optimization runs between `opt_mir()` and `plan_backend()`.
Dead function elimination runs after MIR optimization, before backend emission.
Backend peephole and layout optimization run during `emit_wasm()`.

### T3 (`wasm32-wasi-p2`) — pass configuration (updated 2026-04-15, issue #486)

T3 uses a **separate pass invocation path** (`passes::run_all()` directly, bypassing
`desugar_exprs` which is not GC-safe). The blanket T3 `O0` MIR override that was
present before #486 has been removed.

| Level | T3 MIR passes active | Dead Function Elimination |
|-------|----------------------|--------------------------|
| `0` | None | Disabled |
| `1` | All 9 O1 passes (same as T1) | **Disabled** (WASI export reachability concern) |
| `2` | All O1 + 3 safe O2 arithmetic passes (algebraic_simplify, strength_reduction, string_concat_opt) | **Disabled** |

Several O2/O3 passes remain gated for T3 (`T3_GATED_PASSES`) until each is
independently verified GC-safe: `escape_analysis`, `type_narrowing`, `loop_unroll`,
`licm`, `bounds_check_elim`, `inline_small_leaf`, `aggregate_simplify`, `gc_hint`,
`branch_hint_infer`. See `crates/ark-mir/src/passes/README.md` for unlock conditions.

Full documentation: [optimization.md](optimization.md)

## Diagnostics / Validation 境界

- frontend diagnostics は parse / resolve / typecheck origin を持つ
- backend validation failure (`W0004`) は hard error
- component export surface では `W0005` が使われる
- hidden snapshot / dump support は `ARUKELLT_DUMP_PHASES` と `ARUKELLT_DUMP_DIAGNOSTICS=1`

## Test / Verification Hook

- fixture harness は `tests/fixtures/manifest.txt` 駆動
- fixture 総数や verification gate の現在値は `docs/current-state.md` を参照
- `scripts/check/check-docs-consistency.py` が generated docs drift を検出
- `scripts/manager.py` が correctness gate を担う

## 読み方

- 現挙動の説明は [../current-state.md](../current-state.md) を優先
- このページは pipeline の責務境界を読むための補助資料
- 古い refactor 計画や historical docs は、current behavior の source of truth ではない

## 関連

- [../current-state.md](../current-state.md)
- [diagnostics.md](diagnostics.md)
- [ir-spec.md](ir-spec.md) — CoreHIR / MIR の正規仕様
- [../language/spec.md](../language/spec.md) — 言語仕様 (凍結対象)
- [../platform/wasm-features.md](../platform/wasm-features.md)
- [../migration/t1-to-t3.md](../migration/t1-to-t3.md)
- [optimization.md](optimization.md) — MIR + backend optimization passes
- [../contributing.md](../contributing.md)
