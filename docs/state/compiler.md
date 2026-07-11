# コンパイラ MIR / bootstrap（現行詳細）

ステータス: **現行詳細メモ**（要約の正本は [`../current-state.md`](../current-state.md)）  
参照: [`../compiler/optimization.md`](../compiler/optimization.md)、[`../compiler/bootstrap.md`](../compiler/bootstrap.md)。

---

## MIR Optimization Status

The MIR optimization pipeline is fully implemented and active. See [docs/compiler/optimization.md](../compiler/optimization.md) for the complete reference.

- **20 MIR passes** implemented in selfhost `src/compiler/passes/`, running up to 3 fixed-point rounds
- **`--opt-level` 0/1/2** controls which passes run; default is O1 (9 safe passes)
- **Dead function elimination** removes unreachable stdlib functions at O1+ via MIR
  reachability pruning on `wasm32-gc` (`wasm` and component/wit emit); `wasm32` relies on backend
  reachability (see [t3-reachability.md](../compiler/t3-reachability.md))
- **`wasm32-gc` backend peephole**: `local.set`/`local.get` → `local.tee` conversion at O1+
- **Struct field layout reorder**: hot-field-first layout at O2
- **Backend reachability**: only reachable functions and WASI imports are emitted
- **MIR validation** brackets every pass for early bug detection
- **ADR-040 PR-1/2 (2026-07)**: `TypeTable` / `MirValueType` / `SignatureRegistry` 骨格を `fn_index` からミラー構築（conservative approximation、emit 未使用）。`mir::verify_mir_warn_only` を compile パイプラインに warning-only で挿入（W001–W005 集計ログ）
- **ADR-040 PR-3 (2026-07)**: `MirModule` へ spine 永続化（`type_table` / `signature_registry` / `mono_instance_table`）、`MonoInstanceTable` + subst、`MIR_CALL.func_id_raw` を lowering で設定。W003 除外強化。代表 fixture の mir-verify baseline を `.build/mir-verify-baseline/` に記録。`mir::verify_mir`（W005 fail）を追加（パイプラインは warn-only 維持）
- **ADR-040 PR-4-wide-audit / PR-4-switch (2026-07)**: `legacy_vt_compat` で Vec 戻り値を legacy MirFunction ABI（i32 ハンドル）に整合。`fn_index_mono_sync` で MonoInstanceTable から fn_index mono 戻り値 VT を同期。`mono_return_vt` は spine テーブル優先。`wasm32-gc` `reg-vt-audit-t3.py` (historical script name) mismatched=0（417/419 compiled）。`registry_switch_call_has_void_return` を `call_fallback.ark` に配線（一致 registry パスのみ void 判定）。`wasm32-gc` **381 pass**（ベースライン維持）
- **ADR-040 Phase 3c (2026-07-07)**: post-lowering `value_type` sync で W006=0。spine `type_id` 有効時 W005 スキップ + `val_type` 同期（`mir_local_set_val_type_only`）。`verify_mir_pipeline` を full hard-fail（W005/W006/W007）に切替。420 deduped primary-compile fixture (historical `t3-compile`) で W005=W006=W007=0。`wasm32-gc` **387 pass** / 32 validate-fail / 1 compile-fail。Phase 5–7 残作業は #724 継続追跡（`code_locals` spine 切替は compile trap のため revert 済み）
- Dump support: `ARUKELLT_DUMP_PHASES=optimized-mir` shows before/after state

### `wasm32-gc` MIR optimization re-enabled (issue #486, 2026-04-15; #650 wasm emit unlock 2026-06)

Prior to issue #486, primary (`wasm32-gc`, then documented as `wasm32-wasi-p2`) was forced to `O0` MIR optimization to
stabilize fixture tests. Issue #486 replaced the blanket override with per-pass gating.

Issue #650 extended `wasm32-gc` reachability pruning to `--emit wasm` (not only component/wit)
and documented `wasm32-gc` O2 pass gating in `mir_opt/orchestrate.ark`:

- `wasm32-gc` dead function elimination is **enabled** for `--emit wasm` and component/wit emit
  using the export-surface root contract ([t3-reachability.md](../compiler/t3-reachability.md))
- O2 `gc_hint` is **unlocked** for `wasm32-gc` with GC-safety note + `t3-run:scalar/gc_hint_short_lived.ark`
- O2 `loop_unroll` and `licm` remain **gated** for `wasm32-gc` until independently GC-audited
- Regression fixtures: `tests/fixtures/t3/wasm_dead_fn_elim.ark` (wasm emit),
  `tests/fixtures/component/export_dead_fn_elim.ark` (component emit)

Historical note: older docs referenced `T3_GATED_PASSES` (internal name) in Rust `session.rs` and
`src/compiler/passes/`; the selfhost compiler now gates in `mir_opt/orchestrate.ark`.


## Self-Hosting Bootstrap Status

> **Completion criterion:** `scripts/run/verify-bootstrap.sh` exits 0 (no
> SKIP) **and** `python scripts/manager.py selfhost parity` exits 0.
> See [docs/compiler/bootstrap.md](../compiler/bootstrap.md) for full details.

Verification status of each bootstrap stage (source: `src/compiler/*.ark`):

The selfhost compiler records generic call specializations in the typechecker (`mono_instances`) and emits the current selfhost compiler at a byte-stable bootstrap fixpoint (see issue #312).

| Stage | Description | Status |
|-------|-------------|--------|
| **Stage 0** | Pinned-reference selfhost wasm (`bootstrap/arukellt-selfhost.wasm`, ADR-029) | ✅ **Committed** — 854 KiB, sha256 `68c515…57d`, built from the modular `src/compiler/**` tree |
| **Stage 2** | Pinned wasm compiles current `src/compiler/main.ark` → `s2.wasm` | ✅ **Verified** — modular trees use an isolated bootstrap workspace (flat overlay); `sha256(s2) == sha256(pinned)` |
| **Stage 3** | `sha256(s2) == sha256(s3)` fixpoint (selfhost reproduces itself) | ✅ **Reached** — pinned, s2, and s3 are byte-identical |
| **Fixture parity** | Selfhost compiler passes pinned-vs-current behavioural parity | ✅ **Reached** — 321 PASS, 0 FAIL, 41 SKIP (ADR-029) |
| **CLI parity** | Selfhost `--version` / `--help` match committed snapshot goldens | ✅ **Reached** — 6 PASS, 0 FAIL (ADR-029) |
| **Diagnostic parity** | Selfhost `check` output matches committed `.selfhost.diag` / `.diag` goldens | ✅ **Reached** — 12 PASS, 22 SKIP, 0 FAIL (ADR-029) |

### Fixpoint status

All bootstrap stages pass. The trusted base for verification is the
committed pinned-reference selfhost wasm at
`bootstrap/arukellt-selfhost.wasm` (ADR-029, #585) — the legacy Rust
binary `target/debug/arukellt` is **no longer required** by any selfhost
gate.

The fixpoint criterion is `sha256(s2) == sha256(s3)` — the standard
bootstrap fixpoint where the selfhost compiler reproduces itself from
its own output. Stage 0 is the pinned wasm; Stage 2 is its output on
the current `src/compiler/main.ark`; Stage 3 is Stage 2's output on the
same source.

```
pinned: bootstrap/arukellt-selfhost.wasm
  sha256 = 68c515d983b3149dedb2fd8ae7af9291152aa8c565187129f45b19e8ad09857d
  size   = 874 305 bytes
s2 (pinned compiles modular overlay; postprocessed):
  sha256 = 68c515d983b3149dedb2fd8ae7af9291152aa8c565187129f45b19e8ad09857d
s3 (s2 self-compile; postprocessed):
  sha256 = 68c515d983b3149dedb2fd8ae7af9291152aa8c565187129f45b19e8ad09857d
```

The fixpoint is byte-stable: pinned == s2 == s3. The pinned wasm is the first
artifact produced from the **modular** `src/compiler/**` tree (CoreHIR-based
pipeline) and reproduces itself via the flat bootstrap overlay. It includes the
IDE surface (`ide-analyze`, `lsp`, `debug-adapter`) — the bootstrap overlay now
flattens the `analysis`, `lsp`, and `dap` namespaces instead of stubbing them.
The legacy monolithic-emitter source patches in `scripts/selfhost/checks.py`
were removed; only `component_emitter.ark` is still pulled from the `7911a527`
monolithic snapshot.

Bootstrap overlay generation in `scripts/selfhost/checks.py` is still required
for stage-0→s2 when `src/compiler/**/mod.ark` trees are present (a direct
modular compile of the full tree exceeds the default linear-memory budget).
Pinned bypasses in runtime gates, IDE checks, CLI wrapper, and `DIAG_PARITY_SKIP`
overlay exceptions were removed in the modular full-compile Phase 3 pass.

Peak heap / phase timings: `python3 scripts/selfhost/measure_compile_heap.py [--overlay]`.

CI checks (`python3 scripts/manager.py selfhost <gate>`) — all four are
selfhost-native per ADR-029:

- `selfhost fixpoint` — pinned-bootstrap + Stage-3 sha256 fixpoint
- `selfhost fixture-parity` — pinned-vs-current execution-output parity across `run:` fixtures
- `selfhost diag-parity` — current selfhost `check` vs committed `.selfhost.diag` / `.diag` goldens
- `selfhost parity --cli` — current selfhost `--version` / `--help` vs `tests/snapshots/selfhost/cli-{version,help}.txt`

All four are wired into `verify-harness.sh --full` (and individually via
`--fixpoint`, `--selfhost-fixture-parity`, `--selfhost-diag-parity`).
They exit 0 (SKIP) when `bootstrap/arukellt-selfhost.wasm` is absent so
CI does not hard-fail on a partial checkout. Refresh policy for the
pinned wasm is documented in `bootstrap/PROVENANCE.md`.

### Dual-period policy

The historical Rust/selfhost dual-period has ended. The current verification
contract is selfhost-native and pinned by ADR-029.

### Selfhost-only execution path (#559, #583, ADR-029)

The user-facing `arukellt` CLI is served by a shell wrapper that runs the
**selfhost wasm exclusively** under `wasmtime`. Per #583 the legacy
`ARUKELLT_USE_RUST=1` opt-in has been **retired** and the former Rust CLI
entrypoint has been removed. There is no
longer any Rust CLI crate; all compiler behaviour comes from the selfhost
compiler (`src/compiler/main.ark`).

Wrapper artifact: [`scripts/run/arukellt-selfhost.sh`](../scripts/run/arukellt-selfhost.sh).

Resolution order (selfhost wasm only):

1. `$ARUKELLT_SELFHOST_WASM` (explicit override).
2. `.build/selfhost/arukellt-s2.wasm` (fresh build).
3. `.bootstrap-build/arukellt-s2.wasm` (bootstrap intermediate).
4. `bootstrap/arukellt-selfhost.wasm` (committed pinned reference; see
   `bootstrap/PROVENANCE.md`).

If `wasmtime` is unavailable, or no selfhost wasm can be located, the wrapper
hard-fails with a clear diagnostic — there is no longer a Rust fallback.
Setting `ARUKELLT_USE_RUST=1` now exits non-zero with a pointer to this notice.

Examples:

```bash
# Selfhost wasm via wasmtime (the only execution path)
scripts/run/arukellt-selfhost.sh --help
scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark -o hello.wasm
```

Selfhost gates (`scripts/manager.py selfhost {fixpoint,fixture-parity,parity,diag-parity}`)
are **selfhost-native** per ADR-029 (#585): they bootstrap from the committed
pinned-reference wasm at `bootstrap/arukellt-selfhost.wasm` and never call any
Rust binary. The former package workspace has been retired; compiler behaviour
comes from selfhost sources plus the pinned reference wasm.
