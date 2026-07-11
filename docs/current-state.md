# Arukellt — Current State

> This document reflects the actual, verified state of the project.
> Current-first source of truth for user-visible behavior and verification gates.
<!-- BEGIN GENERATED:CURRENT_STATE_UPDATED -->
> Updated: 2026-07-11.
> Generated-At: 2026-07-11T16:05:21+09:00
> Source-Commit: `a56192d1`
> Verification-Command: `python3 scripts/manager.py verify quick`
> Release-Readiness: **NOT READY**
> Blocking: 4 fixture failure(s), 6 verification check failure(s)
<!-- END GENERATED:CURRENT_STATE_UPDATED -->

## Pipeline

The **corehir** path is the only pipeline for all CLI commands (`compile`, `build`, `run`, `check`).

- **corehir** (only path): `Lexer → Parser → Resolver → TypeChecker → CoreHIR → MIR → Wasm`
- Component path (v2):
  - **ADR-008 契約**: `--emit component` は in-tree（`wasm-tools component new` への恒久依存なし）
  - **現行実装ギャップ**: 一部経路はまだ `WIT generation → wasm-tools component embed/new` や
    Python wrap helper（例: `p2_component_wrap.py`）を使う。理想と現状の差であり、
    公開契約は ADR-008。詳細は下記「ADR contract gaps」
- Shared orchestration entry point: selfhost driver (`src/compiler/driver/mod.ark` via `driver.ark` facade).
- Developer dump support: `ARUKELLT_DUMP_PHASES=parse,resolve,corehir,mir,optimized-mir,backend-plan`

### ADR contract gaps（2026-07-11）

ADR / research と食い違う現行コード・記述は、ここで差分として明示する（公開契約ではない）:

| 項目 | ADR / research | 現行 |
|------|----------------|------|
| `wasm32-freestanding` | ADR-007: 廃止。公開名はハードエラー | driver/emitter に旧 compile-only 実装が残存 → **削除対象** |
| Component emit | ADR-008: in-tree | 一部で `wasm-tools` / Python wrap が残る → **移行中** |
| jco browser | research: Node E2E 済み（パッチ要）。Chrome jco component E2E は **未検証** | 旧「#037 blocked」記述は誤り。#037 transpile ブロッカーは解消済み |
| CLI default | primary = `wasm32-gc`（ADR-013）。`project-state.toml` `cli_default` も `wasm32-gc` | 実装はまだ旧名 `wasm32-wasi-p2` を default 文字列として保持 → **alias 移行中** |
| canonical alias policy | ADR-007 で決定済み（`wasm32-wasi-p1`→`wasm32` 等） | target parser は旧名を直接使用する箇所あり → **移行中** |
| SIMD API shape | ADR-037: nominal `I32x4`/`F32x4`/`Mask4` + `std::wasm::V128`；portable bitwise/bit_select | #698 experimental: lane モジュール関数 + 無印 `v128` + `std::simd::v128` / `std::wasm::v128_*` に portable 操作が混在 → **移行前** |
| SIMD capabilities | ADR-037: `portable_simd_lowering` / `wasm_raw_v128` / `wasm_relaxed_simd` 分離 | `is_simd_target()` が全 target で `true`。Scalar 経路が選ばれず raw/portable 判定未分離 → **未実装** |
| VS Code `arukellt.target` | ADR-007 canonical: `wasm32` / `wasm32-gc` | extension enum が `wasm32-wasi-p1` / `wasm32-wasi-p2` のみ → **alias 移行中** |

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
|--------|--------------|----------------|---------------|-----|-------|
| `wasm32` | supported | complete | stable | Yes | AtCoder / linear-memory competition path (canonical; was wasm32-wasi-p1) |
| `wasm32-gc` | primary | partial | stable | Yes | Primary (ADR-013): Wasm GC + WASI P2 default host profile; GC lowering still partial |
| `native-cpp` | scaffold | scaffold | experimental | No | Scaffold C99 emit path |
| `native-llvm` | scaffold | scaffold | experimental | No | Scaffold LLVM IR emit; semantics/ABI per ADR-045 undecided |
| `wasm32-gc` + `--wasi p3` | not-started | unimplemented | unimplemented | No | Host profile on same language target; not a separate primary |
<!-- END GENERATED:CURRENT_STATE_TARGETS -->

### `wasm32-freestanding`（実装ギャップ・公開契約ではない）

**ADR-007 では廃止済み**（公開ターゲット名はハードエラー。alias にもしない）。

実装ギャップ: 旧 T2 相当の compile-only コードが `src/compiler/driver.ark` /
emitter にまだ残っている場合がある。これは公開契約ではなく **削除対象のレガシー**である。
検証・ドキュメント・CLI 案内では現行ターゲットとして扱わない。
正本のターゲット表は上記生成ブロックおよび [ADR-007](adr/ADR-007-targets.md)。

<!-- BEGIN GENERATED:CURRENT_STATE_TEST_HEALTH -->
## Test Health

- Unit tests: selfhost verification is tracked by `python3 scripts/manager.py verify`
- Fixture harness: 654 passed, 4 failed, 29 skipped (manifest-driven)
- Fixture manifest: 1199 entries
- Wasm validation is a hard error (W0004)
- Verification entry point: `python3 scripts/manager.py verify quick` — **162/168 checks pass**
<!-- END GENERATED:CURRENT_STATE_TEST_HEALTH -->

### Docs and CI hygiene gates

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

Current benchmark measurements (target: `wasm32` (legacy baseline id `wasm32-wasi-p1`), mode: `update-baseline`, 5 iterations).
Full results and history are tracked in [`docs/process/benchmark-results.md`](process/benchmark-results.md).

Run benchmarks locally with:

```bash
python3 scripts/util/benchmark_runner.py --mode full            # full measurement (release build)
python3 scripts/util/benchmark_runner.py --mode compare    # compare against stored baseline
```

詳細な計測表・履歴は [`docs/process/benchmark-results.md`](process/benchmark-results.md) および生成ブロックを参照。

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

Canonical hello.ark sizes at opt-level 2 from [`docs/process/wasm-size-reduction.md`](process/wasm-size-reduction.md): wasm32=534 B, wasm32-gc=918 B.

<!-- BEGIN GENERATED:CURRENT_STATE_DIAGNOSTICS -->
## Diagnostics and Validation

- Canonical diagnostics registry lives in `src/compiler/diagnostics.ark`
- Diagnostics are tracked by code, severity, and phase origin
- `W0001`: same-body heuristic warning for shared mutable aliasing (warning, `typecheck`)
- `W0002`: deprecated target alias warning (warning, `target`)
- `W0004`: generated Wasm failed backend validation (error, `backend-validate`)
- `W0005`: non-exportable function skipped from component exports (warning, `component`)
- `W0101`: deprecated `import <name>` syntax; use `use <name>` (warning, `parse`)
- `E0500`: module requires a different target (e.g. `std::host::sockets` on wasm32 emits E0500; use `--target wasm32-gc`) (error, `resolve`)
- `E0501`: symbol not found in module (e.g. `string::nonexistent_fn()` when the function is not exported by the imported module) (error, `typecheck`)
- Structured diagnostic snapshots are available for tests/docs via `ARUKELLT_DUMP_DIAGNOSTICS=1`
<!-- END GENERATED:CURRENT_STATE_DIAGNOSTICS -->

## CLI Command Surface

The `arukellt` binary exposes the following subcommands:

| Command | Description |
|---------|-------------|
| `arukellt compile <file>` | Compile an `.ark` file to Wasm (`wasm32` or `wasm32-gc`) |
| `arukellt run <file>` | Compile and run an `.ark` file |
| `arukellt check <file>` | Type-check without compiling |
| `arukellt build` | Build the project in the current directory (requires `ark.toml`) |
| `arukellt fmt [file]` | Format `.ark` source files |
| `arukellt test <file>` | Discover and type-check in-file `test` declarations (ADR-041) |
| `arukellt lint <file>` | Run static analysis lints |
| `arukellt targets` | List supported compilation targets |
| `arukellt analyze` | Wasm binary analysis utilities |
| `arukellt init [dir]` | Initialize a new Arukellt project (`--template minimal\|cli\|with-tests\|wasi-host`, `--list-templates`) |
| `arukellt script` | Run scripts defined in `ark.toml` |
| `arukellt doc <symbol>` | Look up stdlib documentation for a symbol or module |
| `arukellt doc --html -o <file>` | Generate rich static stdlib HTML documentation |
| `arukellt lsp` | Start the Language Server Protocol server |
| `arukellt debug-adapter` | Start the Debug Adapter Protocol server |
| `arukellt compose` | Compose Wasm component binaries (validates + `wac plug` delegation — #443 Phase 3) |

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

- `--deny-clock` and `--deny-random` are enforced at **compile time** via MIR scan (`mir_uses_capability`). Detection is transitive. These flags apply to the `run` subcommand; the `compile` subcommand does not accept them (compile only emits Wasm bytes, no runtime policy is applied).
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

要約: `--emit component` / `wit` / `all` は `wasm32-gc` で利用可能（ADR-008 in-tree 契約。実装ギャップは ADR gaps）。
詳細・制限・fixture 列挙は [`docs/state/component-model.md`](state/component-model.md)。

## MIR Optimization / Bootstrap

MIR 最適化と selfhost bootstrap の詳細履歴は
[`docs/state/compiler.md`](state/compiler.md)。
参照: [`compiler/optimization.md`](compiler/optimization.md)、[`compiler/bootstrap.md`](compiler/bootstrap.md)。
