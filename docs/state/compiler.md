# コンパイラ MIR / bootstrap（現行詳細）

ステータス: **現行詳細メモ**（要約の正本は [`../current-state.md`](../current-state.md)）  
参照: [`../compiler/optimization.md`](../compiler/optimization.md)、[`../compiler/bootstrap.md`](../compiler/bootstrap.md)。  
PR 単位の実装履歴は [`../history/milestones.md`](../history/milestones.md) / 関連 plan。

---

## MIR Optimization Status

MIR 最適化パイプラインは実装済みで有効。詳細は
[`../compiler/optimization.md`](../compiler/optimization.md)。

- **20 MIR passes**（`src/compiler/passes/`）、最大 3 fixed-point rounds
- **`--opt-level` 0/1/2** — 既定 O1（9 safe passes）
- **Dead function elimination** — O1+。`wasm32-gc` は MIR reachability
  （wasm / component / wit emit）。`wasm32` は backend reachability
  （[`t3-reachability.md`](../compiler/t3-reachability.md)）
- **`wasm32-gc` peephole** — `local.set`/`local.get` → `local.tee`（O1+）
- **Struct field layout reorder** — hot-field-first（O2）
- **Backend reachability** — 到達可能な関数と WASI import のみ emit
- **MIR validation** — 各 pass 前後で検証
- Dump: `ARUKELLT_DUMP_PHASES=optimized-mir`

### `wasm32-gc` pass gating（現行）

`mir_opt/orchestrate.ark` の現行ゲート:

| Pass / 機能 | `wasm32-gc` | 備考 |
|-------------|-------------|------|
| dead function elimination | enabled | `--emit wasm` および component/wit |
| O2 `gc_hint` | unlocked | GC-safety note 付き |
| O2 `loop_unroll` / `licm` | gated | 独立 GC audit まで無効 |

回帰 fixture: `tests/fixtures/t3/wasm_dead_fn_elim.ark`、
`tests/fixtures/component/export_dead_fn_elim.ark`。

ADR-040 spine（`TypeTable` / `SignatureRegistry` / `MonoInstanceTable`）は
MIR に永続化済み。残作業は #724。

---

## Self-Hosting Bootstrap Status

完了条件: `scripts/run/verify-bootstrap.sh` が 0（SKIP なし）**かつ**
`python scripts/manager.py selfhost parity` が 0。
詳細は [`../compiler/bootstrap.md`](../compiler/bootstrap.md)。

| Stage | Description | Status |
|-------|-------------|--------|
| **Stage 0** | Pinned wasm `bootstrap/arukellt-selfhost.wasm`（ADR-029） | ✅ Committed |
| **Stage 2** | pinned が現行 `src/compiler/main.ark` → `s2.wasm` | ✅ `sha256(s2) == sha256(pinned)` |
| **Stage 3** | `sha256(s2) == sha256(s3)` fixpoint | ✅ Reached |
| **Fixture / CLI / diag parity** | ADR-029 gates | ✅ Reached |

信頼ベースは pinned selfhost wasm。Rust CLI フォールバックは廃止（#583）。

CI gates（いずれも selfhost-native）:

- `selfhost fixpoint`
- `selfhost fixture-parity`
- `selfhost diag-parity`
- `selfhost parity --cli`

### Selfhost 実行パス

Wrapper: [`scripts/run/arukellt-selfhost.sh`](../../scripts/run/arukellt-selfhost.sh)。

解決順:

1. `$ARUKELLT_SELFHOST_WASM`
2. `.build/selfhost/arukellt-s2.wasm`
3. `.bootstrap-build/arukellt-s2.wasm`
4. `bootstrap/arukellt-selfhost.wasm`

`wasmtime` 不在または wasm 未発見時は hard-fail。`ARUKELLT_USE_RUST=1` は非ゼロ終了。
