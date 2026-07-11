# Arukellt — Current State

> This document reflects the actual, verified state of the project.
> Current-first source of truth for user-visible behavior and verification gates.
<!-- BEGIN GENERATED:CURRENT_STATE_UPDATED -->
> Updated: 2026-07-11.
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

| Target | Tier | ADR-013 Tier | Status | Run | Notes |
|--------|------|--------------|--------|-----|-------|
| `wasm32` | — | supported | stable | Yes | AtCoder / linear-memory competition path (canonical; was wasm32-wasi-p1) |
| `wasm32-gc` | — | primary | stable | Yes | Primary (ADR-013): Wasm GC + WASI P2 default host profile |
| `native-cpp` | — | scaffold | scaffold | No | Scaffold C99 emit path |
| `native-llvm` | — | scaffold | scaffold | No | Scaffold LLVM IR emit; semantics/ABI per ADR-045 undecided |
| `wasm32-gc` + `--wasi p3` | — | not-started | not-started | No | Host profile on same language target; not a separate primary |
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

### Benchmark Suite (bench_<suite>_<name>.ark)

| Benchmark | Suite | Compile ms | Run ms | Binary bytes | Correctness |
|-----------|-------|------------|--------|--------------|-------------|
| fib | cpu | 29.088 | 22.592 | 767 | pass |
| binary_tree | cpu | 27.950 | 29.617 | 747 | pass |
| vec_ops | cpu | 28.690 | 23.429 | 1,694 | pass |
| string_concat | cpu | 27.631 | 24.030 | 784 | pass |
| enum_dispatch | cpu | n/a | n/a | n/a | skipped |
| closure_map | cpu | n/a | n/a | n/a | skipped |
| struct_graph | memory | 31.499 | 29.425 | 1,346 | pass |
| error_chain | compute | n/a | n/a | n/a | skipped |
| parse_tree_distance | parse | 25.884 | 55.124 | 4,030 | fail |
| http_parser | application | n/a | n/a | n/a | skipped |
| log_processor | application | 34.965 | 27.468 | 1,598 | pass |
| config_loader | application | n/a | n/a | n/a | skipped |
| data_pipeline | application | n/a | n/a | n/a | skipped |
| template_engine | application | 29.678 | n/a | 2,407 | failed |
| file_io | io | 27.699 | 37.690 | 2,899 | pass |

Source: `tests/baselines/perf/baselines.json` (generated 2026-05-14, wasm32, selfhost compiler).
Some expanded benchmark fixtures currently record compile/runtime skips or correctness failures in this environment; the benchmark runner preserves those statuses instead of hiding them.

Legacy fixtures (`fib`, `binary_tree`, `vec_ops`, `string_concat`) live under `benchmarks/legacy/`
and are retained for cross-language C/Rust comparison. New benchmarks follow the
`bench_<suite>_<name>.ark` naming convention.

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

## Recent Milestones

> 過去の到達記録。**現行の verified state は本文上部と Data Model / ADR gaps を優先**。
> 「fully GC-native」等の過去主張は後から不完全と判明したものがある。

- **Modular full-compile fixpoint reached (2026-06)** — the pinned bootstrap wasm is now built from the modular `src/compiler/**` tree and reproduces itself byte-for-byte (`sha256(pinned) == sha256(s2) == sha256(s3)`). Collision-aware export naming, CoreHIR i64 widening, shaped generic type annotations, binop operand type peeking, and a conditional `local.tee` peephole landed in the modular pipeline; the legacy monolithic emitter patches in `scripts/selfhost/checks.py` were removed. The bootstrap overlay now includes the `analysis`/`lsp`/`dap` namespaces, so the selfhost wasm serves the IDE gates (`ide-analyze`, `lsp`, `debug-adapter`); the LSP advertises completion, `signatureHelp`, and `codeAction` providers, stdlib definition/hover via manifest index (#334 baseline), AST-inferred hover with doc comments (#336), manifest-driven signature help (#337), and auto-import completion/code actions (#340). The lexer diagnostic position bug from the monolithic era is fixed (goldens updated).
- **Selfhost Phase 1 fixpoint achieved** — `sha256(s2) == sha256(s3)` passes (`attainment: reached`). The selfhost compiler (`src/compiler/main.ark`) reproducibly compiles itself. Multi-file module loading, qualified call resolution, and cross-module type handling are all working. See [Self-Hosting Bootstrap Status](#self-hosting-bootstrap-status).
- **`arukellt doc` subcommand added (issue 456)** — stdlib manifest lookup via `arukellt doc <symbol>`. Supports `--json`, `--target`, and fuzzy-match "did you mean?" for unknown symbols.
- **Host capability honesty (#633)** — `std::host::http`, `std::host::sockets`, and `std::host::udp` are not user-reachable on the current selfhost execution path (`call_host_io.ark` dispatches env/fs/process/stdio only). Manifest and [ADR-007: Targets](adr/ADR-007-targets.md#capability-surface) now cross-link #446/#447/#077/#139. HTTPS is not supported for HTTP.
- **GC lowering on primary (`wasm32-gc`)** — Historical milestone claimed “fully GC-native”; **later found incomplete**. Current reality: partial GC struct/array lowering; `String`/`Vec`/enum still mixed (see Data Model). Do not treat this bullet as current verified state.
- **Component / WIT** — `--emit component`, `--emit wit`, and `--emit all` are available on `wasm32-gc` (legacy alias `wasm32-wasi-p2` may still appear in CLI/impl)
- **Stdlib roadmap issues 039–059** — closed; see `issues/done/`
- **Primary runtime correctness sweep (2026-04)** — wasmtime 29.x DRC GC bug mitigated (null collector workaround); peephole local.tee suppressed for GC-ref locals; nested concat scratch-local clobbering fixed; `eq`/`ne`/`split` builtins implemented. Fixture harness then **575/575 pass** with 31 new primary-run entries (historical `t3-run:`). With the wasmtime 46 upgrade (2026-06), the default GC collector switched from DRC to the copying collector, making the null-collector workaround unnecessary for new code paths.
- **Current open queue** — active work focuses on WASI / `std::host::*` rollout, Component Model depth, and trait-first stdlib (see `issues/open/`)
- **`std::host::process::exit` and `abort` available (issue 445)** — `__intrinsic_process_exit(i32)` and `__intrinsic_process_abort()` are wired into the `wasm32` and `wasm32-gc` WASI emitters via `wasi_snapshot_preview1/proc_exit`. Both are noreturn; the emitter emits `unreachable` after every call site. `abort()` uses `proc_exit(134)` (SIGABRT convention). `std::host::process` is no longer a stub.
- **Associated function syntax for builtin types (issue 701)** — `Vec::new<i32>()`, `String::from("hello")`, and `i32::from("42")` now compile and execute correctly. The parser desugars these to the corresponding intrinsic names (`Vec_new_i32`, `String_from`, `parse_i32`) at AST construction time, so the resolver, typechecker, and MIR lowering require no changes. Existing monomorphic constructors (`Vec_new_i32()`, `String_from()`) continue to work unchanged.
- **In-file test syntax (ADR-041)** — `test` is a contextual keyword (lexer keeps `TK_IDENT`) introducing three declaration forms: `test "name" { ... }` (standalone), `test <fn> "name" { ... }` (function-bound white-box), and `test mod "name" { test ... ; ... }` (1-level nested test module). Test bodies are resolved and type-checked in the enclosing file scope. `arukellt test <file>` discovers and lists tests, then type-checks the file. Test names are not registered as module symbols (not callable from production code). See `docs/adr/ADR-041-in-file-test-syntax.md`.
- **In-file test coverage targets (#715)** — Phase 1 adoption adds ≥180 `test` declarations under `std/` (core, collections, text, bytes pure helpers) and Phase 2 adds ≥60 under `src/compiler/` (lexer/parser/resolver/typechecker/mir/diagnostics pure helpers). In-file tests are white-box unit tests co-located with implementation; integration and side-effectful behavior (host, component, wasm emitter body) remain fixture-only. `python3 scripts/check/check-infile-test-adoption.py` reports progress (advisory, non-blocking). `arukellt test` allows lint warnings `W0005`–`W0007` during check-only discovery so assertion helpers do not fail the gate.

## Component Model Status

1. **Component emit**: `--emit component` produces `.component.wasm` on `wasm32-gc`
   (ADR-008: in-tree が契約。現行の一部経路は wrap helper / `wasm-tools` を併用 — ADR gaps 参照)。
2. **WIT generation**: `--emit wit` generates WIT from source-level export type annotations for the supported export surface, including bool, char, string, list, option, result, tuple, record, enum, and variant shapes used by the component fixture surface.
3. **CLI integration**: `--wit <path>`, `--emit wit`, `--emit component`, and `--emit all` are wired into the selfhost CLI.
  `--wit` paths are accepted, validated, and threaded through CLI → `DriverConfig` → resolver/typecheck/MIR → Wasm import section (Phase 1 slices [#652](../issues/done/652-wit-import-parser-grammar.md)–[#654](../issues/done/654-wit-import-component-emit.md)).
  Scalar WIT function imports bind via `import "package/id" as alias` source syntax and resolve to `host::add(...)`-style calls in `tests/fixtures/wit_import/`.
  `ark.toml` `[dependencies]` vendor WIT packages resolve without `--wit` ([#663](../issues/done/663-ark-toml-wit-package-resolution.md)); compose round-trip fixture `tests/fixtures/wit_import/compose_roundtrip/` validates provider/socket WIT surfaces via `arukellt compose --validate` ([#665](../issues/done/665-wit-import-compose-roundtrip-e2e.md)).
  `stream<T>` / `future<T>` async WIT types are now accepted for import parsing
  (see #474 Phase 4). WIT `resource` declarations and `own<T>` / `borrow<T>` handles are
  supported for fixture-backed import and export round-trip ([#473](../issues/done/473-wit-resource-handles.md)).
  WIT `flags` types are supported for import and export round-trip
  fixtures ([#651](../issues/done/651-wit-flags-type-support.md)).
4. **Current export behavior**: non-exportable functions surface `W0005` warnings.
5. **Core Wasm paths**: `wasm32` / `wasm32-gc` core Wasm flows remain available alongside component emit.

### Known Component Model limitations

- The current selfhost `--emit component` path emits a Component Model wrapper around the core Wasm module. With Preview 1 host profiles it may inject a minimal WASI Preview 1 stub instance so the core module's `wasi_snapshot_preview1` imports can instantiate.
- On `wasm32-gc` with WASI P2, the emitter imports `wasi:cli/*` and related Preview 2 interface names directly ([issue 510](../issues/done/510-t3-p2-import-table-switch.md)). Living wrap helpers (e.g. `p2_component_wrap.py`) may still build a `wasi:cli/command` component via `wasm-tools component embed/new` (~5.3KB, no P1 adapter) while in-tree emit (ADR-008) is completed: stdout is adapted through host `get-stdout` + `blocking-write-and-flush`, guest core wasm is patched for canonical ABI `write(ret, ptr, len, 0)`, and `gate_074` proves `wasm-tools validate` + wasmtime prints `hello p2` ([issue 074](../issues/done/074-wasi-p2-native-component.md)). Canonical scratch limits: [`docs/plans/component-canonical-memory.md`](plans/component-canonical-memory.md).
- Component output is `wasm32-gc`-oriented: use `--target wasm32-gc` for `--emit component`, `--emit wit`, and `--emit all` (legacy alias `wasm32-wasi-p2` may still appear in older fixtures).
- The selfhost component interop gate currently passes 103/103 fixtures (`bool-logic`, `bool-renamed`, `calculator`, `char-renamed`, `enum-color-code`, `enum-color-code-renamed`, `enum-colors`, `enum-colors-renamed`, `enum-roundtrip`, `enum-roundtrip-renamed`, `f32-binary`, `f32-multi`, `f32-param-i32`, `f32-renamed`, `f32-result-i32`, `f32-square`, `f64-renamed`, `i16-renamed`, `i32-renamed`, `i64-renamed`, `i8-renamed`, `int-widths`, `list-first`, `list-renamed`, `list-return`, `list-return-renamed`, `list-roundtrip`, `list-roundtrip-renamed`, `metadata-names`, `metadata-scalars`, `multi-type-exports`, `option-bool`, `option-i64`, `option-i64-param`, `option-maybe`, `option-param`, `option-param-renamed`, `option-renamed`, `option-roundtrip`, `option-roundtrip-renamed`, `primitives-float`, `record-add`, `record-add-renamed`, `record-distance`, `record-distance-renamed`, `record-point`, `record-point-renamed`, `record-roundtrip`, `record-roundtrip-renamed`, `result-bool`, `result-param`, `result-param-renamed`, `result-renamed`, `result-roundtrip`, `result-roundtrip-renamed`, `result-safe-div`, `result-string-param`, `string-byte`, `string-byte-renamed`, `string-char`, `string-char-renamed`, `string-count16`, `string-count16-renamed`, `string-count32`, `string-count32-renamed`, `string-count64`, `string-count64-renamed`, `string-countu64`, `string-countu64-renamed`, `string-empty`, `string-empty-renamed`, `string-greet`, `string-len`, `string-len-renamed`, `string-multi`, `string-renamed`, `string-return`, `string-return-renamed`, `string-score`, `string-score-renamed`, `string-score32`, `string-score32-renamed`, `string-signed16`, `string-signed16-renamed`, `string-signed8`, `string-signed8-renamed`, `tuple-bool-param`, `tuple-i64-result`, `tuple-mixed-param`, `tuple-param`, `tuple-param-renamed`, `tuple-renamed`, `tuple-roundtrip`, `tuple-roundtrip-renamed`, `tuple-swap`, `u16-renamed`, `u32-renamed`, `u64-renamed`, `u8-renamed`, `variant-roundtrip`, `variant-roundtrip-renamed`, `variant-shape-area`, `variant-shape-area-renamed`).
- Callable scalar WIT function imports (`import "test:host/math" as host` + `--wit host_math.wit`) typecheck and lower to `MIR_WIT_CALL` with core Wasm import entries ([#034](../issues/done/034-wit-cli-integration.md)). WIT `resource` / `own<T>` / `borrow<T>` fixture shapes compile via name-independent adapters ([#473](../issues/done/473-wit-resource-handles.md)); `stream<T>` / `future<T>` async resource shapes are still rejected with `E0402`.
- Nested or otherwise unsupported component export shapes such as mixed-type multi-export f32 (f32 exports alongside non-f32-scalar exports), mixed-type multi-export string (`String -> String` alongside `String -> i32` or other non-unary string shapes), extra exports next to single-export string/list/option/result adapter shapes, non-`Color` enums (see `export_unsupported_enum_status.ark`), non-`Point` records (see `export_unsupported_record_rect.ark`), non-`Shape` payload variants (see `export_unsupported_variant_payload_i32.ark`), `Option<String>`, `Option<Vec<i32>>`, `Result<i32, bool>`, `Result<i64, i64>`, `Result<String, i32>`, `Result<String, String>`, `Result<Vec<i32>, String>` parameters, `Vec<bool>`, `Vec<u8>`, `Vec<i64>`, `Vec<Option<i32>>`, `Vec<String>`, `tuple<String, String>`, and 3-element tuples are rejected with `E0401` before backend emission.
- general string/general list/general option/result/general enum/general record/complex canonical ABI lift-lower coverage is not complete for every case
- async Component Model features: WIT `future<T>` / `stream<T>` type mapping and import parsing are supported (#474 Phase 4); async function declaration (`async fn`), `await` expression, and full async component export lowering are deferred to #474 later phases
- **jco**: transpile of GC components works on jco 1.25.2 (old #037 blocker cleared). Node.js E2E verified with a local `arguments` reserved-word patch. **Chrome jco component E2E (HTTP → ESM import → WASI shim → run) is not yet verified** — see [`docs/research/target-runtime-verification.md`](research/target-runtime-verification.md). Do not describe jco as “blocked upstream” for GC transpile.
### Component export type tiers

The compiler enforces type-tier restrictions on component exports at compile time:

| Tier | Types | Status | Error |
|------|-------|--------|-------|
| Tier 1 | i32, i64, f64, bool, char, u8, u16, u32, u64, i8, i16 | Supported | — |
| Tier 1 carry-over | f32 | Partially blocked (single-export `(f32) -> f32`, `(f32, f32) -> f32`, `f32 -> i32`, and `i32 -> f32` functions use name-independent bit-reinterpret adapters; multi-export worlds where every export is one of those four scalar f32 shapes use the general f32 adapter from [#659](../issues/done/659-component-export-f32-canonical-abi.md); mixed-type f32 multi-export remains `E0401`) | E0401/#660 |
| Tier 1 carry-over | enum, record, variant | Partially blocked (single-export `Color -> i32`, single-export `Color -> Color`, paired `Color -> Color` / `Color -> i32`, single-export `Point -> i32`, single-export `Point -> Point`, single-export `(Point, Point) -> Point`, paired `Point -> i32` / `(Point, Point) -> Point`, `Shape -> f64`, and `Shape -> Shape` functions use name-independent adapters; non-`Color` enums, non-`Point` records, and non-`Shape` payload variants remain `E0401` — see `export_unsupported_enum_status.ark`, `export_unsupported_record_rect.ark`, and `export_unsupported_variant_payload_i32.ark`; general descriptor adapters beyond the #121 fixture matrix remain future work ([#648](../issues/done/648-component-export-general-canonical-abi.md) closed with explicit boundary)) | E0401 |
| Tier 2 | string, list, option, result, tuple | Partially blocked (single-export `String -> String`, `String -> u8`, `String -> u16`, `String -> u32`, `String -> i8`, `String -> i16`, `String -> i32`, `String -> bool`, `String -> char`, `String -> f32`, `String -> f64`, `String -> i64`, `String -> u64`, `i32 -> String`, `Vec<i32> -> i32`, `i32 -> Vec<i32>`, `Vec<i32> -> Vec<i32>`, `i32 -> Option<i32>`, `bool -> Option<bool>`, `bool -> Option<i64>`, `Option<i32> -> i32`, `Option<i64> -> i64`, `Option<i32> -> Option<i32>`, `bool -> Result<bool, bool>`, `(i32, i32) -> Result<i32, String>`, `Result<i32, i32> -> i32`, `Result<i32, String> -> i32`, `Result<i32, i32> -> Result<i32, i32>`, `(i32, i32) -> tuple<i32, i32>`, `(i64, i64) -> tuple<i64, i64>`, structural `tuple<i32, i32> -> i32`, structural `tuple<i32, bool> -> i32`, structural `tuple<bool, bool> -> i32`, and structural `tuple<i32, i32> -> tuple<i32, i32>` functions have name-independent adapters; multi-export worlds where every export is `String -> String` use the general string adapter from [#660](../issues/done/660-component-export-tier2-general-adapters.md); mixed-type string multi-export and general list/option/result/tuple adapters remain `E0401`) | E0401/#660 |
| Tier 3 | resource | Partially blocked (single-export `pass_handle` i32 roundtrip and `take`/`peek` import use name-independent adapters; full handle-table lifecycle remains #054) | #054 |
| Tier 3 | stream, future | Scaffold — WIT parse/type mapping (#474 Phase 4); export emission deferred | E0402/#474 |
| Tier 3 carry-over | flags | Partially blocked (single-export `flip_read` i32 roundtrip and `set_perms` import use name-independent adapters; general flags descriptors remain #121) | #121 |

Unsupported Tier 2/3 export shapes produce compile errors. Functions with non-exportable
types are excluded from component exports with W0005 warning. Core Wasm binary validation
catches GC reference types that bypass WIT-level checks (W0004).

## Known Limitations

- `--deny-clock` and `--deny-random` are enforced at **compile time** via MIR scan (`mir_uses_capability`). Detection is transitive. These flags apply to the `run` subcommand; the `compile` subcommand does not accept them (compile only emits Wasm bytes, no runtime policy is applied).
- No `--dir` flag means no filesystem access (module contract: [stdlib/modules/fs.md](stdlib/modules/fs.md))
- `native-*` is **scaffold-only** (#641): compile-only GNU assembler stub via `native::emit_native_scaffold`; `run_supported=false`. Full selfhost-native lowering remains #529 Phase 7 follow-up.
- some historical docs remain archived / historical and should not override current-state
- **Host module target-gating and reachability**: `std::host::http`, `std::host::sockets`, and `std::host::udp` are not user-reachable on the current selfhost compile path (see [ADR-007: Targets](adr/ADR-007-targets.md#capability-surface) and #633). Importing `std::host::sockets` or `std::host::udp` on `wasm32` still produces E0500 (issue 448). `std::host::http` is HTTP/1.1 only when implemented; HTTPS is not supported.
- **Bootstrap vs s2 library exports (#666)**: the pinned bootstrap selfhost wasm (`bootstrap/arukellt-selfhost.wasm`) uses a memory-bounded component overlay stub and returns empty WIT / non-invokable components for library-style `pub fn` exports. Build or point `ARUKELLT_SELFHOST_WASM` at `.build/selfhost/arukellt-s2.wasm` for library `--emit wit` and scalar library `--emit component` (`add`/`mul`, `wasm-tools component wit`, `wasmtime --invoke`). CI gates treat empty library WIT as a failure when the active selfhost wasm is s2.
- **Library vs command component worlds (#666)**: on `wasm32-gc` with default WASI P2 host profile, modules that export component-compatible `pub fn` surfaces compile through the **library export** path (generic or specialized canonical ABI adapters). Modules with no exportable `pub fn` and no explicit `--world` use the **P2 command** wrapper (`wasi:cli/run`). When both `pub fn` exports and `fn main` are present, exports take precedence: the artifact is a library component with callable exports; `main` remains in core wasm but is not exported as `wasi:cli/run`. For a command-only program, omit exportable `pub fn` declarations or pass `--world wasi:cli/command`.

## MIR Optimization Status

The MIR optimization pipeline is fully implemented and active. See [docs/compiler/optimization.md](docs/compiler/optimization.md) for the complete reference.

- **20 MIR passes** implemented in selfhost `src/compiler/passes/`, running up to 3 fixed-point rounds
- **`--opt-level` 0/1/2** controls which passes run; default is O1 (9 safe passes)
- **Dead function elimination** removes unreachable stdlib functions at O1+ via MIR
  reachability pruning on `wasm32-gc` (`wasm` and component/wit emit); `wasm32` relies on backend
  reachability (see [t3-reachability.md](compiler/t3-reachability.md))
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
  using the export-surface root contract ([t3-reachability.md](compiler/t3-reachability.md))
- O2 `gc_hint` is **unlocked** for `wasm32-gc` with GC-safety note + `t3-run:scalar/gc_hint_short_lived.ark`
- O2 `loop_unroll` and `licm` remain **gated** for `wasm32-gc` until independently GC-audited
- Regression fixtures: `tests/fixtures/t3/wasm_dead_fn_elim.ark` (wasm emit),
  `tests/fixtures/component/export_dead_fn_elim.ark` (component emit)

Historical note: older docs referenced `T3_GATED_PASSES` (internal name) in Rust `session.rs` and
`src/compiler/passes/`; the selfhost compiler now gates in `mir_opt/orchestrate.ark`.

## API Baseline Notes

- `parse_i64` baseline shape: `Result<i64, String>`
- `parse_f64` baseline shape: `Result<f64, String>`
- The observed outputs are frozen in `tests/baselines/api-baseline.json`

## Self-Hosting Bootstrap Status

> **Completion criterion:** `scripts/run/verify-bootstrap.sh` exits 0 (no
> SKIP) **and** `python scripts/manager.py selfhost parity` exits 0.
> See [docs/compiler/bootstrap.md](docs/compiler/bootstrap.md) for full details.

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
