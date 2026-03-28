# コンパイルパイプライン

> **Current reality first**: 実装の現在地は [../current-state.md](../current-state.md) を基準にしてください。
> このページは current path と refactor target path を並べて示します。

## 現在の主要経路

```text
source (.ark)
  → ark-lexer
  → ark-parser
  → ark-resolve
  → ark-typecheck
  → ark-mir
  → ark-wasm
  → wasm32-wasi-p1 artifact / run via wasmtime
```

- `ark-driver::Session` が shared analysis / orchestration の入口
- `arukellt` が `check` / `compile` / `run` を提供
- `check` は frontend diagnostics まで、`compile` / `run` は Wasm emit と backend validation まで進む

## 採用済みの refactor target path

この refactor で採用する 1 本化パイプラインは次です。

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

### 分割方針

- 新規共有 crate は `ark-hir` のみ
- `Bind / Load / Analyze / Resolve` は `ark-resolve` 内 module 分割
- `MIRValidate / MIROptimize` は `ark-mir` 内 module 分割
- `BackendPlan` は `ark-target` 側
- crate 数を不必要に増やさず、責務境界だけ明確化する

## 現在の crate map

- `crates/ark-lexer`
- `crates/ark-parser`
- `crates/ark-resolve`
- `crates/ark-typecheck`
- `crates/ark-mir`
- `crates/ark-wasm`
- `crates/ark-driver`
- `crates/ark-target`
- `crates/ark-diagnostics`
- `crates/ark-lsp`
- `crates/ark-llvm`
- `crates/ark-stdlib`
- `crates/arukellt`
- planned shared IR crate for this refactor: `crates/ark-hir`

## Session / Artifact Graph 方針

`ark-driver::Session` は段階的に artifact graph へ寄せる。
最小単位の query / artifact は次を想定する。

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

互換のため旧 API は shim として残し、内部で新 query を呼ぶ。

## Diagnostics / Validation 境界

- frontend diagnostics は parse / resolve / typecheck origin を持つ
- backend validation は `W0004` を hard error として扱う
- hidden snapshot/dump support は `ARUKELLT_DUMP_PHASES` で有効化できる
- dump phase 名は `parse`, `resolve`, `corehir`, `mir`, `optimized-mir`, `backend-plan`

## Test / Verification Hook

- fixture harness は manifest-driven で 362 entries
- baseline collector は `tests/baselines/` を更新する
- `scripts/check-docs-consistency.py` が docs の drift を最低限検出する
- `scripts/verify-harness.sh` が correctness gate を担い、heavy perf は別 job に分ける

## 読み方

- 現挙動の説明は [../current-state.md](../current-state.md) を優先
- このページの refactor target は「採用した設計境界」を示す
- 旧 ADR / design 文書は制約や履歴としては有効でも、現挙動の source of truth にはしない

## 関連

- [../current-state.md](../current-state.md)
- [diagnostics.md](diagnostics.md)
- [../platform/wasm-features.md](../platform/wasm-features.md)
- [../migration/t1-to-t3.md](../migration/t1-to-t3.md)
- [../contributing.md](../contributing.md)
