# Arukellt — Current State

> This document reflects the current structured state of the project,
> including the last observed stale evidence where applicable.
> Current-first source of truth for user-visible behavior and verification gates.
<!-- BEGIN GENERATED:CURRENT_STATE_UPDATED -->
> Updated: 2026-07-11.
> Generated-At: 2026-07-11
> Implementation-Commit: `a80b4181`
> Documentation-Commit: `b68fc393`
> Fixture-Snapshot-Commit: `89eb5eb4`
> Verification-Command: `python3 scripts/manager.py verify quick`
> Release-Readiness: **NOT READY**
> Blocking: 367 fixture failure(s), 1 verification check failure(s), 4 additional full-verification blocker group(s)
> Distinct incidents: 6 (derived from incident_id in release-guarantees.toml; 6 failing checks)
<!-- END GENERATED:CURRENT_STATE_UPDATED -->

## Pipeline

The **corehir** path is the only pipeline for all CLI commands (`compile`, `build`, `run`, `check`).

- **corehir** (only path): `Lexer → Parser → Resolver → TypeChecker → CoreHIR → MIR → Wasm`
- Component path (v2):
  - **ADR-008 契約**: `--emit component` は in-tree（`wasm-tools component new` への恒久依存なし）
  - **現行実装ギャップ**: 一部経路はまだ `WIT generation → wasm-tools component embed/new` や
    Python wrap helper（例: `p2_component_wrap.py`）を使う。理想と現状の差であり、
    公開契約は ADR-008。詳細は下記「Accepted ADR contract gaps」
- Shared orchestration entry point: selfhost driver (`src/compiler/driver/mod.ark` via `driver.ark` facade).
- Developer dump support: `ARUKELLT_DUMP_PHASES=parse,resolve,corehir,mir,optimized-mir,backend-plan`

### Accepted ADR contract gaps（2026-07-13）

採択済み ADR / research と食い違う現行コード・記述（公開契約との差）:

| 項目 | ADR / research | 現行 |
|------|----------------|------|
| `wasm32-freestanding` | ADR-007: 廃止。公開名はハードエラー | driver/emitter に旧 compile-only 実装が残存 → **削除対象** |
| Component emit | ADR-008: in-tree | 一部で `wasm-tools` / Python wrap が残る → **移行中** |
| Default Wasm feature emit | ADR-007 §5.1: ターゲット別 allow/deny（iwasm / wasmtime∩Node∩Browser∩jco） | emitter が機能単位で完全強制していない → **段階的ゲート** |
| jco browser | research: Browser core Wasm プローブ済み。jco component Chrome HTTP E2E は別 | #037 transpile ブロッカーは解消（jco≥1.25.2）。component E2E は別途 |
| CLI default | primary = `wasm32-gc`（ADR-013）。`project-state.toml` `cli_default` も `wasm32-gc` | 実装はまだ旧名 `wasm32-wasi-p2` を default 文字列として保持 → **alias 移行中** |
| canonical alias policy | ADR-007 で決定済み（`wasm32-wasi-p1`→`wasm32` 等） | target parser は旧名を直接使用する箇所あり → **移行中** |
| VS Code `arukellt.target` | ADR-007 canonical: `wasm32` / `wasm32-gc` | extension enum が `wasm32-wasi-p1` / `wasm32-wasi-p2` のみ → **alias 移行中** |
| stdlib manifest / generated docs target labels | ADR-007 canonical 名をユーザー向けに使う | `availability` の内部キーは `t1`/`t3` のまま。生成ラベルは移行中 → **SSOT 移行中** |

### Proposed migration gaps（normative ではない）

未採択（PROPOSED）ADR との差。公開契約ではない。採択後に Accepted gaps へ移す:

| 項目 | 提案 ADR | 現行 |
|------|----------|------|
| Trait stdlib redesign | ADR-046 根絶 + ADR-036（D5 撤回）+ ADR-014 削除方針 | モノモルフィック / free 公開面が残存 → **移行前**（根絶方針は採択済み） |
| Intrinsic layer | ADR-042: `docs/data/core-ops.toml` SSOT | manifest / resolver 二重面 → **移行前** |
| SIMD API shape | ADR-037: nominal `I32x4`/`F32x4`/`Mask4` + `std::wasm::V128` | #698 experimental lane モジュール + 無印 `v128` → **移行前** |
| SIMD capabilities | ADR-037: portable/raw/relaxed 三軸 | `is_simd_target()` が全 target で `true` → **未実装** |
| Portable SIMD op semantics | RFC-003: 初期核の NativeSimd↔Scalar 同値 | 未固定 → **RFC DRAFT** |
| Trait expressiveness | RFC-004: Self + 型引数、associated type 先送り | 未実装 → **RFC DRAFT** |

### CoreHIR boundary and driver responsibilities

| Layer | Owns | Does not own |
|-------|------|--------------|
| `corehir/` | `CoreHirRawProgram` DTO, frontend AST enclave, export surface, MIR view | Parser beyond `frontend_ast_*` / `frontend_*_kind*` / `frontend_kind_map` |
| `compiler/session*.ark` | `CompileSession` artifact, `session_corehir` lower queries | File loading, emit mode dispatch |
| `loader/` | Module graph, stdlib paths, `LoadState` | Typecheck, MIR lowering |
| `driver/` | Pipeline orchestration, config/result, timing | Component contract rules, Wasm bytes |
| `component/` | WIT/component validation (`contract_preflight`), emit | AST shape parsing |
| `mir/` | Lowering from `CoreHirMirView` | Direct `parser::` imports (adapters only) |

<!-- BEGIN GENERATED:CURRENT_STATE_TARGETS -->
## Targets

| Target | Support Tier | Implementation | Contract Stability | Run | Notes |
|--------|--------------|----------------|--------------------|-----|-------|
| `wasm32` | supported | complete | stable | Yes | Supported: AtCoder / linear-memory competition path (was wasm32-wasi-p1) |
| `wasm32-gc` | primary | partial | stable | Yes | Primary (ADR-013): Wasm GC + WASI P2 default host profile; GC lowering still partial |
| `native-cpp` | scaffold | scaffold | experimental | No | Scaffold C99 emit path |
| `native-llvm` | scaffold | scaffold | experimental | No | Scaffold LLVM IR emit; semantics/ABI per ADR-045 undecided |

### Host profiles

| Host profile | Targets | Planned | Rejected | Support Tier | Implementation | Contract Stability | Notes |
|--------------|---------|---------|----------|--------------|----------------|--------------------|-------|
| `wasi-p1` | `wasm32` | — | `wasm32-gc` | supported | partial | stable | WASI Preview 1 host profile (AtCoder / linear path on wasm32 only; wasm32-gc+P1 rejected per ADR-007) |
| `wasi-p2` | `wasm32-gc` | — | — | primary | partial | stable | Default host profile for primary target wasm32-gc (ADR-013) |
| `wasi-p3` | `wasm32-gc` | — | — | not-started | unimplemented | experimental | Future WASI Preview 3 host profile on wasm32-gc; not a separate language target |

> **Status axis glossary:**
> - **Support Tier**: primary / supported / scaffold / not-started — project priority
> - **Implementation**: complete / partial / scaffold / unimplemented — runtime readiness
> - **Contract Stability**: stable / experimental — public API commitment
> - A target can be `stable` (contract) + `partial` (implementation) simultaneously:
>   `stable` means the target name and CLI interface won't change; `partial` means
>   not all language features are fully lowered to Wasm yet.
<!-- END GENERATED:CURRENT_STATE_TARGETS -->

Default Wasm feature emit（[ADR-007 §5.1](adr/ADR-007-targets.md#default-wasm-feature-emit)）:
`wasm32` ⊆ iwasm（Wasm 2.0 Core まで）;
`wasm32-gc` ⊆ wasmtime ∩ Node ∩ Browser ∩ jco≥1.25.2（**multiple memories は default 禁止**）。
運用表: [platform/target-runtime-and-surfaces.md](platform/target-runtime-and-surfaces.md#default-wasm-feature-emit)。

### `wasm32-freestanding`（実装ギャップ・公開契約ではない）

**ADR-007 では廃止済み**（公開ターゲット名はハードエラー。alias にもしない）。

実装ギャップ: 旧 T2 相当の compile-only コードが `src/compiler/driver.ark` /
emitter にまだ残っている場合がある。これは公開契約ではなく **削除対象のレガシー**である。
検証・ドキュメント・CLI 案内では現行ターゲットとして扱わない。
正本のターゲット表は上記生成ブロックおよび [ADR-007](adr/ADR-007-targets.md)。

<!-- BEGIN GENERATED:CURRENT_STATE_TEST_HEALTH -->
## Test Health

- Unit tests: selfhost verification is tracked by `python3 scripts/manager.py verify`
- Fixture harness (observed snapshot): 796 passed, 367 failed, 417 skipped (observed harness: 1580)
- Fixture registry: 2693 manifest entries (distinct unit from harness outcomes)
- Not in last harness snapshot: 1113 registry entries (not proof they fail)
- Accounting note: 796+367+417=1580 outcomes from the 2026-07-11 selfhost fixture-parity run; 2693 is tests/fixtures/manifest.txt registry size. The 1113 remainder were not part of that run (not proof they fail).
- Wasm validation is a hard error (W0004)
- Verification entry point: `python3 scripts/manager.py verify quick` — **175/176 checks pass**

### Active blockers

Generated from `data/release-guarantees.toml` (checks with `release_blocking = true, result = "fail"`).

| ID | Scope | Category | Affected | Incident | Failure summary | Command | Owner | Issue | First seen | Last verified | Freshness |
|----|-------|----------|---------:|----------|-----------------|---------|-------|-------|------------|---------------|-----------|
| `check_fixture_harness` | `full` | `fixture` | 367 | `incident_fixture_parity_367` | Failures in observed harness snapshot. Same incident as selfhost fixture-parity — not double-counted. See project-state.toml for current registry count. | `python3 scripts/manager.py verify fixtures` | compiler/runtime | #287 | `89eb5eb4` | `89eb5eb4` | `fresh` |
| `check_close_gate_076` | `quick` | `verification` | 1 | `incident_close_gate_076` | P2 filesystem close gate expected 'hello p2 fs' but produced NUL bytes | `python3 scripts/check/check-false-done-close-gates.py` | WASI P2 filesystem | #076 | `89eb5eb4` | `89eb5eb4` | `fresh` |
| `check_selfhost_fixpoint` | `full` | `bootstrap` | 1 | `incident_selfhost_fixpoint` | Stage 2 and Stage 3 compiler hashes differ; fixpoint not reached | `python3 scripts/manager.py selfhost fixpoint --build` | selfhost compiler | #459 | `a80b4181` | `a80b4181` | `fresh` |
| `check_selfhost_cli_parity` | `full` | `bootstrap` | 3 | `incident_selfhost_cli_parity` | CLI parity drifts for --help, lint, and compose --validate | `python3 scripts/manager.py selfhost parity --mode --cli` | selfhost CLI | #530 | `a80b4181` | `a80b4181` | `fresh` |
| `check_wat_roundtrip` | `full` | `target-contract` | 1 | `incident_wat_roundtrip` | The wasm2wat/wat2wasm roundtrip gate fails | `bash scripts/run/wat-roundtrip.sh` | Wasm backend | unfiled | `a80b4181` | `a80b4181` | `fresh` |
| `check_component_interop_wasmtime` | `full` | `component-interop` | 103 | `incident_component_interop_103` | All wasmtime component-interop cases fail. Dedicated command (not aggregate verify full). | `python3 scripts/manager.py verify component-interop` | component model | #074 | `a80b4181` | `a80b4181` | `fresh` |
<!-- END GENERATED:CURRENT_STATE_TEST_HEALTH -->

### Docs and CI hygiene gates

- **Code-quality entrypoints:** `python3 scripts/manager.py fmt`, `fmt --check`,
  `lint`, and `quality changed|quick|structure|full|report` are the canonical
  local/CI commands (ADR-047). `quality structure` emits the hard repository
  contracts as text or schema-versioned JSON. `quality report` emits advisory
  distributions and deterministic hotspots without treating metric values as
  quality scores. `quality quick` applies the touched-file ratchet and hard
  structure checks; repository-wide format/lint and churn analysis belong to
  `quality full`.
- **Comment/API policy:** `python3 scripts/check/check-comment-policy.py` derives
  manifest API (A), root compiler boundary (B), and internal visibility (C), and
  enforces A/B documentation plus structured temporary comments. `--json` uses
  the same finding model as text output.
- **Canonical parser migration boundary:** 23 compiler/stdlib files are
  content-addressed in `docs/data/ark-formatter-baseline.toml`. Only their exact
  current hashes may skip a formatter parse failure;
  any edit fails closed. Issue #791,
  owner `compiler-tooling`, removal condition, and 2026-08-31 re-evaluation are
  mandatory. This is an implementation gap, not canonical formatting.
- **Opt-equivalence (O0 == O1):** `bash scripts/run/test-opt-equivalence.sh --quick` runs in `verify quick` background checks; release checklist item is no longer deferred.
- **Stdlib scoreboard:** `docs/stdlib/scoreboard.md` is generated by `python3 scripts/gen/generate-docs.py` from `std/manifest.toml` and fixture coverage.
- **Anchor fragments:** `python3 scripts/check/check-anchor-fragments.py` validates `path.md#anchor` links (ADR-019 §2) in the `verify quick` static pass.
- **Playground format UI:** `docs/playground/index.html` exposes a Format toolbar action wired to `pg.format()`; tokenize drives editor syntax highlighting (#635).

## Data Model (all Wasm targets)

**Important:** The `wasm32` compatibility path still uses the
linear-memory data model. `wasm32-gc` emits Wasm GC reference locals/types and
`struct.*` / `array.*` instructions for the current aggregate lowering
shape. MIR/CoreHIR include a distinct `VT_GC_REF` tag for aggregate reference
locals, params, and struct/enum returns. This is an implementation slice, not the
complete GC-native data model: strings, Vec, enums, options/results, and generic
payloads may still use linear-memory or fixed-shape representation in places.
Layout policy (compiler-private) is proposed in ADR-035; phases live in
`docs/plans/wasm-gc-implementation.md`.

The data model across all targets:

| Type | Representation |
|------|---------------|
| `i32`, `bool` | `i32` |
| `i64` | `i64` |
| `f64` | `f64` |
| `String` | `i32` pointer to heap-allocated length-prefixed bytes |
| `Vec<T>` | `i32` pointer to heap-allocated buffer with length/capacity |
| Structs | `wasm32`: `i32` pointer to heap-allocated struct; `wasm32-gc`: GC struct references for supported field shapes |
| Enums / Option / Result | Discriminated union in linear memory (GC layout migration in progress; ADR-035) |
| Closures | Parameter-passing captures; `call_indirect` for HOF dispatch |

`wasm32-gc` differs from `wasm32` both in WASI/host profile / component emit and in
this GC aggregate emission path. `wasm32` remains the linear-memory compatibility backend.

**Future:** Completing the Wasm GC backend remains tracked by issue #686 and
ADR-035 / `docs/plans/wasm-gc-implementation.md`, including GC strings, Vec/enum
representations, cast-based dispatch,
full fixture coverage, and `wasm32` / `wasm32-gc` parity gates.

## Performance Snapshot

> **INVALID / NO CURRENT MEASUREMENTS.**
> [`docs/history/benchmarks/benchmark-results.md`](history/benchmarks/benchmark-results.md) の Current Run
> （2026-06-09, target `wasm32-wasi-p1`）は全件 skipped であり、現行 performance evidence ではない。
> 有効な計測が再取得されるまで、この節をリリース判断や最適化根拠に使わないこと。

Run benchmarks locally with:

```bash
python3 scripts/util/benchmark_runner.py --mode full            # full measurement (release build)
python3 scripts/util/benchmark_runner.py --mode compare    # compare against stored baseline
```

詳細な計測表・履歴は [`docs/history/benchmarks/benchmark-results.md`](history/benchmarks/benchmark-results.md) を参照（現状は invalid artifact）。有効測定がない場合は current process側には再実行commandのみを置く。

<!-- BEGIN GENERATED:CURRENT_STATE_PERF -->
## Baseline and Perf Gates

- Baselines are materialized under `tests/baselines/`
- Compile-time baseline cases:
  - `docs/examples/hello.ark`
  - `docs/examples/vec.ark`
  - `docs/examples/closure.ark`
  - `docs/sample/parser.ark`
- Current thresholds:
  - `arukellt check`: median compile time regression must stay within 10%
  - `arukellt compile`: median compile time regression must stay within 20%
- Heavy perf comparisons are intentionally separated from the normal correctness gate
<!-- END GENERATED:CURRENT_STATE_PERF -->

### Binary Size (`wasm32` vs `wasm32-gc`)

| Source | wasm32 size | wasm32-gc size | Notes |
|--------|---------|---------|-------|
| hello.ark | 494 B | 494 B | Both targets use same linear-memory emitter at default opt |
| vec.ark | 2,382 B | 2,382 B | Vec ops, same target path |
| closure.ark | n/a | n/a | Compile fails (ICE) — pre-existing, tracked in issue backlog |

Historical size-attribution measurements live in
[`history/process/wasm-size-reduction.md`](history/process/wasm-size-reduction.md).
They are not a current benchmark baseline; do not copy their retired target-era
numbers into current contracts.

<!-- BEGIN GENERATED:CURRENT_STATE_DIAGNOSTICS -->
## Diagnostics and Validation

- Canonical code declarations live in `src/compiler/diagnostics/codes.ark`; lifecycle metadata is recorded in `data/warnings.toml`
- Diagnostics are tracked by code, severity, phase origin, and implementation maturity
- `W0001`: possible unintended sharing of reference type (warning, `typecheck`, implemented, emitted)
- `W0002`: deprecated target alias (warning, `lint-post-resolve`, implemented, emitted)
- `W0003`: unused symbol (warning, `lint-post-resolve`, implemented, emitted)
- `W0004`: generated Wasm failed validation (error, `backend-validate`, implemented, emitted)
- `W0005`: non-exportable function skipped from component exports (warning, `component`, implemented, emitted)
- `W0006`: unused import (warning, `lint-post-resolve`, implemented, emitted)
- `W0007`: unused binding (warning, `lint-post-resolve`, implemented, emitted)
- `W0008`: documentation drift (warning, `lint-post-resolve`, declared, not currently emitted)
- `W0009`: deprecated API usage with the manifest-recorded replacement (warning, `lint-post-resolve`, implemented, emitted)
- `W0010`: prefer use import for frequently qualified paths (warning, `lint-post-resolve`, implemented, emitted)
- `W0011`: prefer else-if over nested else { if } (warning, `lint-post-resolve`, implemented, emitted)
- `W0101`: deprecated import syntax; use `use` (warning, `parse`, declared, not currently emitted)
- `W0102`: component lowering note (warning, `lint-post-resolve`, declared, not currently emitted)
- Structured diagnostic snapshots are available for tests/docs via `ARUKELLT_DUMP_DIAGNOSTICS=1`
<!-- END GENERATED:CURRENT_STATE_DIAGNOSTICS -->

<!-- BEGIN GENERATED:CURRENT_STATE_CLI_SURFACE -->
## CLI Command Surface

The `arukellt` binary exposes the following subcommands.
Generated from [`data/cli-surface.toml`](data/cli-surface.toml).

| Command | Description | Presence | Contract | Implementation |
|---------|-------------|:--------:|:--------:|:--------------:|
| `arukellt compile` | Compile an .ark file to Wasm | present | stable | ✅ functional |
| `arukellt run` | Compile and run an .ark file | present | stable | ✅ functional |
| `arukellt check` | Type-check without compiling | present | stable | ✅ functional |
| `arukellt build` | Build the project in the current directory (requires ark.toml) | present | stable | ⚠️ limited |
| `arukellt fmt` | Format .ark source files | present | provisional | ✅ functional |
| `arukellt test` | Discover and type-check in-file test declarations (ADR-041) | present | provisional | ✅ functional |
| `arukellt lint` | Run static analysis lints | present | provisional | ✅ functional |
| `arukellt targets` | List supported compilation targets | present | stable | 🚫 unavailable |
| `arukellt analyze` | Analyze a compiled Wasm binary | present | provisional | ✅ functional |
| `arukellt init` | Initialize a new Arukellt project | present | stable | ✅ functional |
| `arukellt script` | Manage and run project scripts from ark.toml | present | provisional | ✅ functional |
| `arukellt doc` | Look up stdlib documentation or generate HTML reference | present | stable | ✅ functional |
| `arukellt component` | Build/inspect/validate components (build aliases compile --emit component) | present | provisional | ⚠️ limited |
| `arukellt compose` | Validate and plan component linking; binary compose via wac plug (ADR-034) | present | provisional | ⚠️ limited |
| `arukellt lsp` | Start the LSP server (stdio) | present | provisional | ✅ functional |
| `arukellt debug-adapter` | Start the DAP debug adapter (scaffold) | present | experimental | 🔧 scaffold |
| `arukellt help` | Print help | present | stable | ✅ functional |

> **Axis legend:** Presence = subcommand exists in binary. Contract = CLI contract maturity (stable/provisional/experimental). Implementation = runtime behavior (functional/limited/scaffold/unavailable).
> A `stable` contract with `unavailable` implementation means the command name is stable but the feature is not yet working.
<!-- END GENERATED:CURRENT_STATE_CLI_SURFACE -->

### `arukellt doc`

Looks up stdlib manifest metadata and displays:
- Function signature (`fn name(params) -> return`)
- Module path (e.g. `std::host::stdio`)
- Stability (`stable`, `provisional`, etc.)
- Target availability (`wasm32` / `wasm32-gc` flags from `availability` block)
- Doc description, examples, errors, and `see_also` when present

Flags: `--json` (machine-readable output), `--target <TARGET>` (show availability warning for specific target), `--html -o <output.html>` (generate a static stdlib reference page).

Unknown symbols produce a "Did you mean?" list of fuzzy candidates. Module paths (e.g. `std::host::http`) list all functions in the module.

`arukellt doc --html -o docs/docs/std/index.html` generates the GitHub Pages
stdlib reference served at `/arukellt/docs/std/`. The Pages workflow builds
this file through the selfhost CLI entrypoint instead of a Python doc generator.

## Known Limitations

- `--deny-clock` / `--deny-random`: **intended** compile-time MIR scan on `run` (transitive; not on `compile`), but **not implemented** on the current selfhost CLI. Fixtures remain in `DIAG_PARITY_SKIP` (#459). See [`process/policy.md`](process/policy.md) and [`data/capabilities.toml`](data/capabilities.toml).
- No `--dir` flag means no filesystem access (module contract: [stdlib/modules/fs.md](stdlib/modules/fs.md))
- `native-*` is **scaffold-only** (#641): compile-only GNU assembler stub via `native::emit_native_scaffold`; `run_supported=false`. Full selfhost-native lowering remains #529 Phase 7 follow-up.
- some historical docs remain archived / historical and should not override current-state
- **Host module target-gating and reachability**: `std::host::http`, `std::host::sockets`, and `std::host::udp` are not user-reachable on the current selfhost compile path (see [Capability surface](platform/target-runtime-and-surfaces.md#capability-surface) and #633). Importing `std::host::sockets` or `std::host::udp` on `wasm32` still produces E0500 (issue 448). `std::host::http` is HTTP/1.1 only when implemented; HTTPS is not supported.
- **Bootstrap vs s2 library exports (#666)**: the pinned bootstrap selfhost wasm (`bootstrap/arukellt-selfhost.wasm`) uses a memory-bounded component overlay stub and returns empty WIT / non-invokable components for library-style `pub fn` exports. Build or point `ARUKELLT_SELFHOST_WASM` at `.build/selfhost/arukellt-s2.wasm` for library `--emit wit` and scalar library `--emit component` (`add`/`mul`, `wasm-tools component wit`, `wasmtime --invoke`). CI gates treat empty library WIT as a failure when the active selfhost wasm is s2.
- **Library vs command component worlds (#666)**: on `wasm32-gc` with default WASI P2 host profile, modules that export component-compatible `pub fn` surfaces compile through the **library export** path (generic or specialized canonical ABI adapters). Modules with no exportable `pub fn` and no explicit `--world` use the **P2 command** wrapper (`wasi:cli/run`). When both `pub fn` exports and `fn main` are present, exports take precedence: the artifact is a library component with callable exports; `main` remains in core wasm but is not exported as `wasi:cli/run`. For a command-only program, omit exportable `pub fn` declarations or pass `--world wasi:cli/command`.

## API Baseline Notes

- `parse_i64` baseline shape: `Result<i64, String>`
- `parse_f64` baseline shape: `Result<f64, String>`
- The observed outputs are frozen in `tests/baselines/api-baseline.json`

## Recent Milestones

過去の到達記録は [`history/milestones.md`](history/milestones.md) へ分離した。
現行の verified state は本ファイル上部と ADR gaps / Data Model を優先する。

## Component Model Status

要約: command component は pinned compiler で利用可能、library component は s2 compiler が必要、WIT emit は partial。正確な軸別状態は [`data/component-availability.md`](data/component-availability.md) を参照。
詳細・制限・fixture 列挙は [`docs/state/component-model.md`](state/component-model.md)。

Export boundary (summary; full tiers in `state/component-model.md`): unsupported shapes
such as non-`Color` enums, non-`Shape` payload variants, and non-`Point` records
(see `export_unsupported_record_rect`) are rejected with `E0401`. Broader f32 /
Tier-2 general adapters are tracked via #659 / #660 (closed with explicit
boundary under #648).

## MIR Optimization / Bootstrap

MIR 最適化と selfhost bootstrap の詳細履歴は
[`docs/state/compiler.md`](state/compiler.md)。
参照: [`compiler/optimization.md`](compiler/optimization.md)、[`compiler/bootstrap.md`](compiler/bootstrap.md)。
