# Component Model（現行詳細）

ステータス: **現行詳細メモ**（要約の正本は [`../current-state.md`](../current-state.md)）  
契約: [ADR-008](../adr/ADR-008-component-wrapping.md)。ギャップは current-state ADR gaps。

---

## Component Model Status

1. **Component emit**: `--emit component` produces `.component.wasm` on `wasm32-gc`
   (ADR-008: in-tree が契約。現行の一部経路は wrap helper / `wasm-tools` を併用 — ADR gaps 参照)。
2. **WIT generation**: `--emit wit` generates WIT from source-level export type annotations for the supported export surface, including bool, char, string, list, option, result, tuple, record, enum, and variant shapes used by the component fixture surface.
3. **CLI integration**: `--wit <path>`, `--emit wit`, `--emit component`, and `--emit all` are wired into the selfhost CLI.
  `--wit` paths are accepted, validated, and threaded through CLI → `DriverConfig` → resolver/typecheck/MIR → Wasm import section (Phase 1 slices [#652](../../issues/done/652-wit-import-parser-grammar.md)–[#654](../../issues/done/654-wit-import-component-emit.md)).
  Scalar WIT function imports bind via `import "package/id" as alias` source syntax and resolve to `host::add(...)`-style calls in `tests/fixtures/wit_import/`.
  `ark.toml` `[dependencies]` vendor WIT packages resolve without `--wit` ([#663](../../issues/done/663-ark-toml-wit-package-resolution.md)); compose round-trip fixture `tests/fixtures/wit_import/compose_roundtrip/` validates provider/socket WIT surfaces via `arukellt compose --validate` ([#665](../../issues/done/665-wit-import-compose-roundtrip-e2e.md)).
  `stream<T>` / `future<T>` async WIT types are now accepted for import parsing
  (see #474 Phase 4). WIT `resource` declarations and `own<T>` / `borrow<T>` handles are
  supported for fixture-backed import and export round-trip ([#473](../../issues/done/473-wit-resource-handles.md)).
  WIT `flags` types are supported for import and export round-trip
  fixtures ([#651](../../issues/done/651-wit-flags-type-support.md)).
4. **Current export behavior**: non-exportable functions surface `W0005` warnings.
5. **Core Wasm paths**: `wasm32` / `wasm32-gc` core Wasm flows remain available alongside component emit.

### Known Component Model limitations

- The current selfhost `--emit component` path emits a Component Model wrapper around the core Wasm module. With Preview 1 host profiles it may inject a minimal WASI Preview 1 stub instance so the core module's `wasi_snapshot_preview1` imports can instantiate.
- On `wasm32-gc` with WASI P2, the emitter imports `wasi:cli/*` and related Preview 2 interface names directly ([issue 510](../../issues/done/510-t3-p2-import-table-switch.md)). Living wrap helpers (e.g. `p2_component_wrap.py`) may still build a `wasi:cli/command` component via `wasm-tools component embed/new` (~5.3KB, no P1 adapter) while in-tree emit (ADR-008) is completed: stdout is adapted through host `get-stdout` + `blocking-write-and-flush`, guest core wasm is patched for canonical ABI `write(ret, ptr, len, 0)`, and `gate_074` proves `wasm-tools validate` + wasmtime prints `hello p2` ([issue 074](../../issues/done/074-wasi-p2-native-component.md)). Canonical scratch limits: [`docs/plans/component-canonical-memory.md`](../plans/component-canonical-memory.md).
- Component output is `wasm32-gc`-oriented: use `--target wasm32-gc` for `--emit component`, `--emit wit`, and `--emit all` (legacy alias `wasm32-wasi-p2` may still appear in older fixtures).
- The selfhost component interop gate currently passes **103/103** fixtures.
  個別名の正本は fixture manifest（`tests/fixtures/` の component / wit 系）であり、
  本メモへ手書き列挙しない。
- Callable scalar WIT function imports (`import "test:host/math" as host` + `--wit host_math.wit`) typecheck and lower to `MIR_WIT_CALL` with core Wasm import entries ([#034](../../issues/done/034-wit-cli-integration.md)). WIT `resource` / `own<T>` / `borrow<T>` fixture shapes compile via name-independent adapters ([#473](../../issues/done/473-wit-resource-handles.md)); `stream<T>` / `future<T>` async resource shapes are still rejected with `E0402`.
- Nested or otherwise unsupported component export shapes such as mixed-type multi-export f32 (f32 exports alongside non-f32-scalar exports), mixed-type multi-export string (`String -> String` alongside `String -> i32` or other non-unary string shapes), extra exports next to single-export string/list/option/result adapter shapes, non-`Color` enums (see `export_unsupported_enum_status.ark`), non-`Point` records (see `export_unsupported_record_rect.ark`), non-`Shape` payload variants (see `export_unsupported_variant_payload_i32.ark`), `Option<String>`, `Option<Vec<i32>>`, `Result<i32, bool>`, `Result<i64, i64>`, `Result<String, i32>`, `Result<String, String>`, `Result<Vec<i32>, String>` parameters, `Vec<bool>`, `Vec<u8>`, `Vec<i64>`, `Vec<Option<i32>>`, `Vec<String>`, `tuple<String, String>`, and 3-element tuples are rejected with `E0401` before backend emission.
- general string/general list/general option/result/general enum/general record/complex canonical ABI lift-lower coverage is not complete for every case
- async Component Model features: WIT `future<T>` / `stream<T>` type mapping and import parsing are supported (#474 Phase 4); async function declaration (`async fn`), `await` expression, and full async component export lowering are deferred to #474 later phases
- **jco**: transpile of GC components works on jco 1.25.2 (old #037 blocker cleared). Node.js E2E verified with a local `arguments` reserved-word patch. **Chrome jco component E2E (HTTP → ESM import → WASI shim → run) is not yet verified** — see [`docs/research/target-runtime-verification.md`](../research/target-runtime-verification.md). Do not describe jco as “blocked upstream” for GC transpile.
### Component export type tiers

The compiler enforces type-tier restrictions on component exports at compile time:

| Tier | Types | Status | Error |
|------|-------|--------|-------|
| Tier 1 | i32, i64, f64, bool, char, u8, u16, u32, u64, i8, i16 | Supported | — |
| Tier 1 carry-over | f32 | Partially blocked (single-export `(f32) -> f32`, `(f32, f32) -> f32`, `f32 -> i32`, and `i32 -> f32` functions use name-independent bit-reinterpret adapters; multi-export worlds where every export is one of those four scalar f32 shapes use the general f32 adapter from [#659](../../issues/done/659-component-export-f32-canonical-abi.md); mixed-type f32 multi-export remains `E0401`) | E0401/#660 |
| Tier 1 carry-over | enum, record, variant | Partially blocked (single-export `Color -> i32`, single-export `Color -> Color`, paired `Color -> Color` / `Color -> i32`, single-export `Point -> i32`, single-export `Point -> Point`, single-export `(Point, Point) -> Point`, paired `Point -> i32` / `(Point, Point) -> Point`, `Shape -> f64`, and `Shape -> Shape` functions use name-independent adapters; non-`Color` enums, non-`Point` records, and non-`Shape` payload variants remain `E0401` — see `export_unsupported_enum_status.ark`, `export_unsupported_record_rect.ark`, and `export_unsupported_variant_payload_i32.ark`; general descriptor adapters beyond the #121 fixture matrix remain future work ([#648](../../issues/done/648-component-export-general-canonical-abi.md) closed with explicit boundary)) | E0401 |
| Tier 2 | string, list, option, result, tuple | Partially blocked (single-export `String -> String`, `String -> u8`, `String -> u16`, `String -> u32`, `String -> i8`, `String -> i16`, `String -> i32`, `String -> bool`, `String -> char`, `String -> f32`, `String -> f64`, `String -> i64`, `String -> u64`, `i32 -> String`, `Vec<i32> -> i32`, `i32 -> Vec<i32>`, `Vec<i32> -> Vec<i32>`, `i32 -> Option<i32>`, `bool -> Option<bool>`, `bool -> Option<i64>`, `Option<i32> -> i32`, `Option<i64> -> i64`, `Option<i32> -> Option<i32>`, `bool -> Result<bool, bool>`, `(i32, i32) -> Result<i32, String>`, `Result<i32, i32> -> i32`, `Result<i32, String> -> i32`, `Result<i32, i32> -> Result<i32, i32>`, `(i32, i32) -> tuple<i32, i32>`, `(i64, i64) -> tuple<i64, i64>`, structural `tuple<i32, i32> -> i32`, structural `tuple<i32, bool> -> i32`, structural `tuple<bool, bool> -> i32`, and structural `tuple<i32, i32> -> tuple<i32, i32>` functions have name-independent adapters; multi-export worlds where every export is `String -> String` use the general string adapter from [#660](../../issues/done/660-component-export-tier2-general-adapters.md); mixed-type string multi-export and general list/option/result/tuple adapters remain `E0401`) | E0401/#660 |
| Tier 3 | resource | Partially blocked (single-export `pass_handle` i32 roundtrip and `take`/`peek` import use name-independent adapters; full handle-table lifecycle remains #054) | #054 |
| Tier 3 | stream, future | Scaffold — WIT parse/type mapping (#474 Phase 4); export emission deferred | E0402/#474 |
| Tier 3 carry-over | flags | Partially blocked (single-export `flip_read` i32 roundtrip and `set_perms` import use name-independent adapters; general flags descriptors remain #121) | #121 |

Unsupported Tier 2/3 export shapes produce compile errors. Functions with non-exportable
types are excluded from component exports with W0005 warning. Core Wasm binary validation
catches GC reference types that bypass WIT-level checks (W0004).

