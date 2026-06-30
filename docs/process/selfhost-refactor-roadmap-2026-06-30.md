# Selfhost Refactoring Roadmap — 2026-06-30

This roadmap operationalizes the two-layer audit ([selfhost-code-smell-audit-2026-06-30.md](selfhost-code-smell-audit-2026-06-30.md) + [selfhost-feature-underuse-audit-2026-06-30.md](selfhost-feature-underuse-audit-2026-06-30.md)) into 9 execution phases (Phase 0–8).

**Status: planned (2026-06-30), not yet started.**

## Scope decision

- **Depth**: all layers (Easy + Medium + Hard, including trait/impl spike and enum-with-payload).
- **Granularity**: phase-level roadmap. Per-issue detail is generated when each phase starts.
- **verify failures**: repaired first (Phase 0) to establish a clean baseline before any refactor lands.

## Phase dependency graph

```
Phase 0 (verify修復) ──→ 全Phaseの前提
                          │
Phase 1 (文字列) ─────────┤
Phase 2 (制御構文) ───────┤  互いに独立、並行可能
Phase 3 (ボイラープレート) ┤
                          │
Phase 4 (データ構造) ─────┤  Phase 3 の Vec_extend を利用可能
Phase 5 (型安全) ─────────┤  Phase 2 の match を利用
                          │
Phase 6 (trait/impl) ─────┤  Phase 5 完了が望ましい
Phase 7 (enum) ───────────┤  Phase 6 完了が望ましい
                          │
Phase 8 (generic HOF) ────┘  ADR-036 に block
```

- **Parallelizable**: Phase 1 / 2 / 3 are mutually independent.
- **Sequential preferred**: Phase 6 → Phase 7 (trait + enum compose).
- **External block**: Phase 8 depends on ADR-036 language work.

## Per-phase completion criteria (all phases)

1. `python3 scripts/manager.py verify quick` exits 0 (no new breakage after Phase 0).
2. Relevant golden / fixture gates pass.
3. Changes committed (per AGENTS.md Commit Policy).
4. Audit documents (2 files) updated: completed items marked done.
5. Large phases (4, 6, 7) require an ADR recording design decisions.

---

## Phase 0 — verify cleanup (prerequisite)

**Goal**: `python3 scripts/manager.py verify quick` → 0 failures.

### Current failures (13, grouped into 6 clusters)

| Cluster | Failures | Nature | Fix difficulty |
|---------|---------:|-------|----------------|
| A. env/tool incompat | 1 | T3 fixture WASM validation (#686) — `wasm-tools validate --features gc` incompatible with wasm-tools 1.252.0 | Easy (script fix) |
| B. repo structure | 1 | 18 `test-*.ark` in root, `scripts/s3_diff.py` / `s3_to_s4.py` in scripts/ root | Easy (file move) |
| C. golden drift | 3 | WIT round-trip (#618), component standard-world (#118), LSP lifecycle (#569) | Medium (regen or impl fix) |
| D. compiler boundary limits | 4 | file >249 lines (4 files), fn >60 lines (5 fns), import fan-out (7 files), import cycle (resolver/bindings) | Medium-Hard (split refactor) |
| E. module load / doc | 2 | false-done close-gate (`test:host/math`, `test:doc/parse`), doc example check (superpowers/plans) | Medium |
| F. other | 2 | runtime Wasm debug smoke (#638, walrus parse), harness 12 checks | Investigate |

### Work

1. **Cluster A**: fix `scripts/check/check-t3-wasm-validate.py` `wasm-tools validate` invocation for 1.252.0 (`--features` → `-d` or `--json-out`).
2. **Cluster B**: move root `test-*.ark` → `tests/fixtures/`; move `scripts/s3_diff.py` / `s3_to_s4.py` → `scripts/util/`.
3. **Cluster C**: investigate root cause of 3 golden drifts; regenerate goldens or fix impl regression.
4. **Cluster D**: minimum split to satisfy boundary limits (full split deferred to Phase 3/4).
5. **Cluster E**: fix `test:host/math` / `test:doc/parse` module load; add `skip-doc-check` to `docs/superpowers/plans/*` ark blocks or fix them.
6. **Cluster F**: investigate walrus parse + harness 12 failures; repair.

### Verification

`verify quick` → 0 failures.

### Risk

Clusters C/D may be impl bugs; if deeper than expected, split into separate issues and proceed with Phase 1+ on a "no new breakage" basis.

---

## Phase 1 — string building modernization (Easy, high value)

**Audit items**: S2/R7 (string builder), R1 (escape consolidation), S1 (JSON DOM).

**Goal**: Replace 665 hand-rolled `concat` JSON chains and 87 manual string accumulations with `std::text::builder` + `std::json` DOM.

### Target files

- `lsp/responses_*.ark` (navigation, lifecycle, symbol) — JSON DOM
- `lsp/diag_object.ark`, `lsp/diag_json.ark` — JSON DOM
- `diagnostics/json.ark` — JSON DOM
- `component/wit_text.ark`, `component/wit_type_defs.ark` — builder
- `lint/registry.ark`, `main/init_templates.ark` — builder
- 12 `quote_string`/`json_escape` wrappers → `std::json::json_escape`

### Work order

1. **R1**: consolidate 12 escape wrappers → `use std::json::json_escape` (pure deletion).
2. **S2**: 87 `mut s = String_from` + `concat` → `builder_new` + `builder_append` + `builder_build`.
3. **S1**: 665 JSON `concat` chains → `JsonValue` construction + `stringify` (library gap: add LSP type helpers).

### Verification

JSON output byte-identical via golden comparison. LSP lifecycle gate (#569) passes.

### Risk

S1 needs `JsonValue` helpers for LSP types (Position, Range). Helper addition is a small std extension.

---

## Phase 2 — control flow modernization (Easy, mechanical)

**Audit items**: L1 (match), L2 (for-in).

**Goal**: if-else kind dispatch chains → `match`; `while i < len` → `for i in 0..n` / `for item in values(v)`.

### Target files

- `parser/decl_dispatch.ark` (9-chain → match)
- `lexer/punct_compound.ark` (9-chain → match)
- `parser/pratt_bp_infix_right.ark` (10-chain → match)
- `parser/pratt_binops_arith.ark` (5-chain → match)
- 110 `while i < len(...)` → for-in (repo-wide, gradual)

### Work order

1. **L1**: token/node kind dispatch if-else chains → `match` (parser/lexer focus).
2. **L2**: simple index loops → `for i in 0..n` (low-risk first).
3. **L2 cont.**: `for item in values(v)` form (`Vec<String>`/`Vec<i32>`; verify struct Vecs).

### Verification

`verify quick` + all parser/lexer fixtures pass.

### Risk

Low. `match` on i32 with literal patterns verified on selfhost target (`tests/fixtures/control/match_int.ark`).

---

## Phase 3 — boilerplate reduction (Pure refactor)

**Audit items**: R5 (raw opcode → helper), R6 (getter removal), R8 (struct literal), R2 (Vec extend helper).

**Goal**: Replace 5,407 raw `emit_byte`/`emit_leb128` with helpers; remove 100+ trivial getters; replace 19 giant constructors with struct literals.

### Target files

- `wasm/intrinsic_vec_push_gc.ark` (20 raw OP_LOCAL_GET), `intrinsic_vec_pop_gc.ark` (15), `intrinsic_vec_access.ark` (11) → `helpers_core_locals::emit_local_get`
- `component/contract_allows_*.ark` (30+ getters) → direct access
- `main/args_record.ark` (27-param ctor), `driver/config_record.ark` (18-param) → struct literal
- 64 `push(dst, get_unchecked(src, i))` loops → `Vec_extend` helper

### Work order

1. **R5**: raw opcode → helper (wasm/ focus, mechanical).
2. **R6**: getter → direct field access (component/ focus).
3. **R8**: giant constructors → struct literals.
4. **R2**: add `Vec_extend<T>` to prelude (i32/String/i64 monomorphized variants); replace 64 manual extend loops.

### Verification

`verify quick` + all wasm fixtures pass. Emit output byte-identical.

### Risk

R5 requires byte-identical emit output. R2 needs prelude extension (monomorphized variants sufficient).

---

## Phase 4 — data structure modernization (Medium)

**Audit items**: S3/R3 (HashMap adoption), R4 (string-set table).

**Goal**: 20+ linear name→index lookups → `HashMap<String, i32>`; 411 `eq(clone(...))` OR chains → table lookup / HashSet.

### Target files

- `mir/lower/ctx_mono_type_params.ark`, `ctx_gc_struct.ark`, `ctx_gc_enum.ark` (5 linear searches) → HashMap
- `resolver/wit_type_map.ark` (2 body-identical `wit_record_index`/`wit_enum_index`) → shared helper or HashMap
- `lsp/init_workspace.ark`, `lsp/symbol_index_project.ark`, `lsp/import_graph.ark` (3 body-identical `*_contains`) → HashSet
- `typechecker/builtins_simd.ark` (92 chains) → table lookup
- `mir/lower/call_types.ark` (33 chains) → table lookup
- `mir/lower/ctx_fn_return_vt.ark` (40+ chains) → table lookup

### Work order

1. **R3**: 20+ linear lookups → HashMap (`std::collections::hash_map` or intrinsic layer).
2. **R3 cont.**: 3 `*_contains` → `HashSet<String>`.
3. **R4**: 411 `eq(clone(...))` chains → declarative table `[(name, kind, handler_id), ...]` + table dispatch.

### Verification

`verify quick` + typecheck fixtures pass.

### Risk

R4 changes dispatch structure; needs intrinsic-registration coverage test. Choose between codegen or hand-written table.

---

## Phase 5 — type safety improvement (Medium)

**Audit items**: L5/R9 (sentinel → Option/Result), B (magic sentinel).

**Goal**: 67 `return 0 - 1` / `return 0 - 2` → `Option<i32>` / `Result<T,E>` or named constants.

### Work order

1. **Intermediate step**: introduce named constants `NOT_FOUND: i32 = 0 - 1` / `NOT_HANDLED: i32 = 0 - 2` per module (immediate readability win).
2. **Gradual migration**: lookup function returns → `Option<i32>` (callers use `match` / `is_some`).
3. Remaining sentinels → `Result<T, ErrorKind>`.

### Target files

`resolver/scope_lookup.ark`, `mir/lower/core_*.ark` (20+), `wasm/code_body.ark`, `fmt/prune_imports.ark`, `mir_opt/loop_unroll.ark`.

### Verification

`verify quick` + boundary-check gates pass.

### Risk

Caller-side fixes may ripple widely. Proceed gradually.

---

## Phase 6 — polymorphism introduction (Hard, needs spike)

**Audit items**: L6/R10 (trait/impl → GC dispatch), D (GC/LM dual impl).

**Goal**: 106 `is_gc_target()` runtime branches → trait/impl static dispatch.

### Work order

1. **Spike**: verify `trait`/`impl` dispatch works on selfhost target (define a small trait, compile + run).
2. **MemoryAccess trait design**: define abstract ops `load_byte`/`store_byte`/`array_get`/`array_set` as a trait.
3. **GC/LM 2 implementations**: `impl MemoryAccess for GcTarget` / `impl MemoryAccess for LinearTarget`.
4. **intrinsic body unification**: merge GC + LM versions of each intrinsic via `MemoryAccess` trait.
5. Remove `is_gc_target()` branches.

### Target files

`wasm/intrinsic_*.ark` (106 sites), `wasm/sections_types_sigs.ark` (20 sites).

### Verification

`verify quick` + GC array smoke gate + wasm fixtures pass. Emit byte-identical.

### Risk

Highest. If spike fails, defer Phase 6 entirely and proceed to Phase 7. Fallback: codegen per-target file generation.

### ADR required

Yes — record trait dispatch design + spike result.

---

## Phase 7 — IR representation modernization (Hard, large surface)

**Audit items**: L7 (enum-with-payload vs tagged struct).

**Goal**: `HirNode { tag: i32, ... }` / `MirInst { op: i32, ... }` tagged structs → enum with payload variants.

### Work order

1. Convert `HirNode` to `enum HirNode` with variants (update dump/format code simultaneously).
2. Convert `MirInst` to `enum MirInst` with variants.
3. Update all pass dispatch (parser → resolver → typechecker → mir/lower → wasm emitter) to `match`.
4. Update debug dump code for enum variants.

### Target files

`hir/node_record.ark`, `mir/inst_record.ark`, `mir/block_record.ark` + all pass dispatch code.

### Verification

`verify quick` + boundary-check gates + dump golden comparison.

### Risk

Large surface — every pass affected. Debug dump infra depends on struct layout; full rewrite of dump code needed. Phase 6 completion preferred (trait + enum compose).

### ADR required

Yes — record enum-with-payload migration design.

---

## Phase 8 — library gap resolution (Library gap, blocked on ADR-036)

**Audit items**: S4/R2 (generic HOFs), L4 (closures).

**Goal**: Implement generic `map`/`filter`/`fold`/`any`/`find` via trait-based `Iterator`/`Fn`; replace 109 hand-written loops.

**Prerequisite**: ADR-036 (trait-based stdlib redesign) implementation complete.

### Work order

1. Implement `Iterator` trait and `Fn` trait in language/compiler (ADR-036 work).
2. Implement generic `map`/`filter`/`fold`/`any`/`find` via `Iterator` trait.
3. Migrate `Vec_extend<T>` to generic version.
4. Replace 109 hand-written loops with closure + HOF.

### Verification

`verify quick` + typecheck fixtures.

### Risk

Language feature implementation itself is large-scale. Depends on ADR-036 progress. Outside selfhost's responsibility.

---

## Progress tracking

| Phase | Status | Audit items resolved |
|-------|--------|---------------------|
| 0 — verify cleanup | done (13→8 failures) | repo structure, doc examples, WIT golden, LSP golden, --emit fix, Cargo workspace, import cycle |
| 1 — string building | in-progress (R1 done, S2 done, S1 pending) | R1 (12 escape wrappers consolidated), S2 (75/75 files, 579 calls — all `VAR = concat(VAR, X)` patterns converted; bootstrap limit refuted) |
| 2 — control flow | planned | L1, L2 |
| 3 — boilerplate | done | R5 (raw opcode→helper in vec intrinsics), R6 (50 trivial getters→direct field access), R8 (giant constructors→struct literals), R2 (Vec_extend helpers, 25 manual extend loops replaced) |
| 4 — data structures | planned | S3, R3, R4 |
| 5 — type safety | planned | L5, R9, B |
| 6 — polymorphism | planned | L6, R10, D |
| 7 — IR representation | planned | L7 |
| 8 — library gap | planned (blocked) | S4, R2, L4 |

Status values: planned → in-progress → done → deferred.
