# Component Model（現行詳細）

ステータス: **現行詳細メモ**（要約の正本は [`../current-state.md`](../current-state.md)）  
契約: [ADR-008](../adr/ADR-008-component-wrapping.md)。ギャップは current-state ADR gaps。

---

## Component Model Status

1. **Component emit**: `--emit component` produces `.component.wasm` on `wasm32-gc`
   (ADR-008: in-tree が契約。現行の一部経路は wrap helper / `wasm-tools` を併用 — ADR gaps 参照)。Library exports are routed specialized-first before the scalar generic fallback, so supported string/record/list/option/result adapters are not flattened to `s32` by the selfhost bootstrap path (#667).
2. **WIT generation**: `--emit wit` generates WIT from source-level export type annotations for the supported export surface, including bool, char, string, list, option, result, tuple, record, enum, and variant shapes used by the component fixture surface.
3. **CLI integration**: `--wit <path>`, `--emit wit`, `--emit component`, and `--emit all` are wired into the selfhost CLI.
  `--wit` paths are accepted, validated, and threaded through CLI → `DriverConfig` → resolver/typecheck/MIR → Wasm import section (Phase 1 slices [#652](../../issues/done/652-wit-import-parser-grammar.md)–[#654](../../issues/done/654-wit-import-component-emit.md)).
  WIT callable imports bind via `import "package/id" as alias` source syntax. The explicit callable matrix covers bool/i64/f32/f64/string/list/option/result/tuple/record/variant; duplicate aliases/packages and namespace collisions are span diagnostics (#670/#671).
  `ark.toml` `[dependencies]` resolves vendor WIT packages without `--wit` ([#663](../../issues/done/663-ark-toml-wit-package-resolution.md)) and resolves component `.wasm` artifacts through the manifest composition path. Resolved artifacts are SHA-256 locked in `ark.lock`, cached under `.build/components/`, and validated against package/world WIT metadata before `wac plug` composition (#674).
  `std::wit` owns the shared WIT 1.0 parser, naming helpers, and AST/type lowering primitives. Compiler WIT files are facades/adapters over that shared surface rather than an independent parsed-type model (#706).
  `stream<T>` / `future<T>` syntax is understood by `std::wit`, but callable compiler imports using those async shapes are rejected with `E0402` until async lowering is a supported backend contract (#671/#474).
  WIT `resource` declarations and `own<T>` / `borrow<T>` handles remain supported for their fixture-backed import/export round-trip ([#473](../../issues/done/473-wit-resource-handles.md)); WIT `flags` types remain supported for their fixture-backed round-trip ([#651](../../issues/done/651-wit-flags-type-support.md)).
4. **Generated bindings**: parsed WIT record/enum/variant declarations have stable Ark names and generated surface bindings; nested list/option/result/tuple fields preserve the WIT package/interface metadata. Recursive value bindings and unsupported resource-handle fields are rejected with `E0402` (#672).
5. **Developer visibility**: `--dump-phases backend-plan` includes WIT import package/interface and callable/type counts, so resolver lowering is inspectable rather than fixture-only (#670).
6. **Core Wasm paths**: `wasm32` / `wasm32-gc` core Wasm flows remain available alongside component emit.

### Known Component Model limitations

- The current selfhost `--emit component` path emits a Component Model wrapper around the core Wasm module. With Preview 1 host profiles it may inject a minimal WASI Preview 1 stub instance so the core module's `wasi_snapshot_preview1` imports can instantiate.
- On `wasm32-gc` with WASI P2, the emitter imports `wasi:cli/*` and related Preview 2 interface names directly ([issue 510](../../issues/done/510-t3-p2-import-table-switch.md)). Guest-native in-tree emit ([issue 714](../../issues/done/714-wasi-p2-emitter-native-component-output.md), [issue 668](../../issues/done/668-p2-native-component-polish.md)) produces a `wasi:cli/command` component without a P1 adapter: guest imports `get-stdout` / `get-stderr` + `blocking-write-and-flush`, and `gate_074` / `gate-668-p2-native-polish` prove `wasm-tools validate` + wasmtime (`hello p2`, stderr/args/env). Environment still uses a P1-shaped bridge (`gate-668-p2-args-env`). Canonical scratch limits: [`docs/plans/component-canonical-memory.md`](../plans/component-canonical-memory.md).
- Component output is `wasm32-gc`-oriented: use `--target wasm32-gc` for `--emit component`, `--emit wit`, and `--emit all` (legacy alias `wasm32-wasi-p2` may still appear in older fixtures).
- Component interop の現行結果は release check [`check_component_interop_wasmtime`](../data/release-guarantees.md#check-catalogue) を参照する。本メモでは pass/fail/count を所有しない。個別 fixture 名の正本は `tests/fixtures/` の component / WIT manifest である。
- Nested or otherwise unsupported component export shapes remain compile-time `E0401` rather than silently lowering through the scalar emitter. The authoritative per-shape Tier-2 status is [`docs/data/component-export-tier2.toml`](../data/component-export-tier2.toml): every #673 row is explicitly `supported`, `deferred`, or `rejected` with a reason. In particular `Option<String>`, `Option<Vec<i32>>`, generalized string/list result payloads, `Vec<String>`, `Vec<u8>`, `Vec<i64>`, `Vec<Option<i32>>`, string tuples, 3-element tuples, and general name-independent record/enum/variant layout planning remain deferred.
- General string/list/option/result/enum/record/complex canonical ABI lift-lower coverage is not complete for every nested shape. This is an explicit type-tier boundary, not a scalar-emitter routing ambiguity.
- Async Component Model export lowering remains deferred: `std::wit` can represent `future<T>` / `stream<T>`, while callable compiler imports and exports that require async backend semantics are rejected.
- **jco**: transpile of GC components works on jco 1.25.2 (old #037 blocker cleared). Node.js E2E verified with a local `arguments` reserved-word patch. **Chrome jco component E2E (HTTP → ESM import → WASI shim → run) is not yet verified** — see [`docs/research/target-runtime-verification.md`](../research/target-runtime-verification.md). Do not describe jco as “blocked upstream” for GC transpile.

### Component export type tiers

The compiler enforces type-tier restrictions on component exports at compile time:

| Tier | Types | Status | Error |
|------|-------|--------|-------|
| Tier 1 | i32, i64, f64, bool, char, u8, u16, u32, u64, i8, i16 | Supported | — |
| Tier 1 carry-over | f32 | Partially blocked (single-export `(f32) -> f32`, `(f32, f32) -> f32`, `f32 -> i32`, and `i32 -> f32` functions use name-independent bit-reinterpret adapters; multi-export worlds where every export is one of those four scalar f32 shapes use the general f32 adapter from [#659](../../issues/done/659-component-export-f32-canonical-abi.md); mixed-type f32 multi-export remains `E0401`) | E0401/#660 |
| Tier 1 carry-over | enum, record, variant | Partially blocked (the existing fixture-backed canonical ABI adapters remain supported and are now reachable from library emit after #667; general descriptor/layout adapters are explicitly deferred in the #673 matrix) | E0401 |
| Tier 2 | string, list, option, result, tuple | Partially supported. Existing name-independent/fixture-backed adapters are specialized-first; unsupported nested/generalized rows are enumerated in `docs/data/component-export-tier2.toml` and stay `E0401` rather than falling through to scalar lowering. | E0401/#673 |
| Tier 3 | resource | Partially blocked (single-export `pass_handle` i32 roundtrip and `take`/`peek` import use name-independent adapters; full handle-table lifecycle remains #054) | #054 |
| Tier 3 | stream, future | Syntax/AST supported in `std::wit`; callable compiler lowering rejected until async backend support | E0402/#474 |
| Tier 3 carry-over | flags | Partially blocked (single-export `flip_read` i32 roundtrip and `set_perms` import use name-independent adapters; general flags descriptors remain #121) | #121 |

Unsupported Tier 2/3 export shapes produce compile errors. Functions with non-exportable types are excluded from component exports with W0005 warning. Core Wasm binary validation catches GC reference types that bypass WIT-level checks (W0004).
