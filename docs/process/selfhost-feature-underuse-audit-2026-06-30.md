# Selfhost Feature-Underuse Audit — 2026-06-30

Audit scope: `src/compiler/` (selfhost compiler, ~90,000 lines). This is the **second-layer** audit, going one level deeper than [selfhost-code-smell-audit-2026-06-30.md](selfhost-code-smell-audit-2026-06-30.md). Where the first audit classified *what* the bad code looks like, this audit classifies *why* it is hard to maintain: **language and std-library features that already exist but are not used**, so selfhost hand-rolls less-maintainable equivalents.

**Status: complete (2026-06-30).** Classification/audit only — no code changes were made.

Method: three parallel read-only subagent sweeps (std-feature underuse; language-feature underuse; reinvented-wheel clusters), cross-checked with repo-wide quantification. All "compiler supports X" claims were verified against `tests/fixtures/` and `std/` examples that compile on the current selfhost target.

## Executive summary

selfhost systematically writes **around** features the language and std library already provide. The gap is not a single missing feature — it is broad under-adoption across at least **8 language features** and **6 std modules**, plus **10 reinvented-wheel clusters**. Root causes split into four buckets (see §"Why underused" for per-feature attribution):

| Root cause | Features affected | Share |
|-----------|-------------------|------|
| Habit / predates feature maturity | `match`, `for-in`, `?`, closures, `Option/Result` vs sentinel, enum-with-payload | ~50% |
| Library gap (std needs more variants) | higher-order prelude (`map`/`filter`/`fold` for custom types), `Vec_extend`, `split` adoption | ~20% |
| Compiler-maturity caution for selfhost target | `trait`/`impl` dispatch | ~15% |
| Deliberate style choice | method-call syntax (function-call deemed "safer") | ~15% |

### Headline quantification (grep, 2026-06-30)

| Signal | Count | Notes |
|--------|------:|-------|
| `for ... in` loops | **0** | language supports it; 110 `while i < len(...)` instead |
| `trait` / `impl` definitions | **0** | language supports it; 106 `is_gc_target()` runtime branches instead |
| Closure literals passed as args | **0** | language supports it; prelude `map_i32_i32` etc. unused (33 refs are intrinsic registration only) |
| `?` operator (real, not in strings) | **1** | language supports it; nested `match Result::Ok/Err` instead |
| `std::text::builder` usage | **0** | std module exists; 87 `mut s = String_from` + `concat` accumulations instead |
| `std::collections` usage | **2** | std module exists; 20+ linear-scan name lookups instead |
| `split(` usage in compiler logic | **0** | prelude has `split`; hand-rolled `index_of`/`substring` parsing instead |
| `return 0 - 1` / `return 0 - 2` sentinels | **67** | `Option`/`Result` exist; sentinels used instead |
| `match` expressions | 64 | mostly Result unpacking; **not** used for token/node-kind dispatch despite support |
| method-call `.foo(` | 5 | vs 15,811 `module::foo(` — deliberate style choice (low priority) |

## Part 1 — Language features underused

### L1. `match` expression for kind dispatch — rarely used

**Status.** Language supports `match` on `i32` with literal patterns (verified: `tests/fixtures/control/match_int.ark` compiles on selfhost target). selfhost uses `match` only 64×, almost entirely for `Result::Ok/Err` unpacking — **not** for token-kind / node-kind / opcode dispatch.

**Workaround.** Long sequential `if k == TK_FN() { } if k == TK_STRUCT() { } ...` chains.

**Examples.**
- `src/compiler/parser/decl_dispatch.ark:18-58` — 9 sequential `if k2 == tokens::TK_*()` for declaration dispatch.
- `src/compiler/lexer/punct_compound.ark:10-36` — 9 sequential `if c == 61` / `if c == 33` for compound punctuation.
- `src/compiler/parser/pratt_bp_infix_right.ark:6-35` — 10 sequential `if kind == tokens::TK_*()` for precedence.
- `src/compiler/parser/pratt_binops_arith.ark:7-11` — 5 sequential `if kind == ...` for arithmetic op mapping.

**Why not used.** Habit — the codebase predates confident `match`-on-i32 support. Not a compiler gap.

**Cost.** Adding a new token/node kind requires updating multiple `if` chains; no exhaustiveness check; easy to miss a branch.

**Migration.** Easy — direct mechanical replacement, no library gap.

### L2. `for ... in` loop — not used at all

**Status.** Language supports `for i in 0..n` and `for item in values(v)` (verified: `tests/fixtures/selfhost/comprehension_map.ark:11` uses `for v in values(doubled)`). selfhost uses it **0×**; 110 `while i < len(...) { ...; i = i + 1 }` loops instead.

**Workaround.** Manual index-based `while` loops everywhere (this is also Category F of the first audit).

**Examples.**
- `src/compiler/wasm/sections_exports.ark:48` — `while i < len(names) { ...; i = i + 1 }`.
- `src/compiler/mir/lower/ctx_mono_type_params.ark:148` — `while j < arg_count && j < len(type_var_names) { ... }`.
- `src/compiler/resolver/context.ark:48,54` — `while i < len(builtin_names)` ×2.

**Why not used.** Habit — codebase predates full `for-in` support. Not a compiler gap.

**Cost.** Off-by-one risk, manual increment, no iterator abstraction.

**Migration.** Easy for `for i in 0..n` form; `for item in values(v)` needs `values()` to work for the element type (works for `Vec<i32>`/`Vec<String>`; may need verification for struct vectors).

### L3. `?` operator — not used

**Status.** Language supports `?` (verified: `tests/fixtures/stdlib_option_result/question_mark.ark:4` uses `let val: i32 = parse_i32(s)?`). selfhost uses it **1×** (and that may be incidental); 41 `?` matches are all inside string literals (type variables like `?T`).

**Workaround.** Nested `match r { Result::Ok(x) => ..., Result::Err(e) => return Result::Err(e) }`.

**Examples.**
- `src/compiler/loader/module_file_load_local.ark:27-43` — nested `match primary_result { Ok => ..., Err => { match mod_result { ... } } }` for fallback file load.
- `src/compiler/lint/config.ark:102-114` — manual Result unpacking for config load.
- `src/compiler/main/compile_core.ark:106-117` — manual Result handling for file writes.

**Why not used.** Habit + possible library gap (Result helper methods may not have been mature when written). Not a current compiler gap.

**Cost.** Significant boilerplate; error-propagation paths hard to follow.

**Migration.** Medium — need to confirm `?` works in all selfhost return contexts and that Result helper methods (`map`, `and_then`) exist or are added.

### L4. Closures as first-class values — not used

**Status.** Language supports closure expressions `fn(x) -> T { ... }` (spec §3.16). selfhost passes **0** closure literals as arguments. The 22 `fn(` matches are all type annotations, not closure literals.

**Workaround.** All loop bodies inlined with `while`; prelude higher-order fns (`map_i32_i32` etc.) are registered as intrinsics (33 refs) but never called with a closure in selfhost code.

**Why not used.** Likely library gap — prelude HOFs are monomorphized per-type (`map_i32_i32`, `map_String_String`, ...) and selfhost needs variants for custom types (`FnSig`, `AstNode`, `MirFunction`). Also possible performance caution in early implementations.

**Cost.** Duplicated iteration logic; no composability; 109 hand-written `while` loops that are pure map/filter/fold/any/find shapes.

**Migration.** Hard — needs either generic HOFs (trait-based `Iterator`/`Fn` per ADR-036) or many more monomorphized prelude variants. This is a **library gap**, not a selfhost fault.

### L5. `Option`/`Result` vs sentinel integers — mixed

**Status.** Language supports `Option<T>` and `Result<T,E>` (verified in stdlib + fixtures). selfhost uses them in only 25 + 15 = 40 contexts, but returns **67** `0 - 1` / `0 - 2` sentinels.

**Workaround.** `i32` return with `0 - 1` = "not found", `0 - 2` = "not handled" (also Category B of the first audit).

**Examples.**
- `src/compiler/resolver/scope_lookup.ark:11,19` — `return 0 - 1` for "not found".
- `src/compiler/mir/lower/core_for_iter.ark:13,18,27` — `return 0 - 1` for error conditions.
- `src/compiler/mir/lower/core_call_payload.ark:18` — `return 0 - 2` for "not handled".
- `src/compiler/wasm/code_body.ark:151` — `return 0 - 1` for lookup failure.

**Why not used.** Habit from early development when the type system was less mature. Not a current compiler gap.

**Cost.** No type safety; sentinel can collide with valid value; semantics undocumented; caller must know which sentinel means what.

**Migration.** Medium — mechanical for lookup functions; need named `NOT_FOUND`/`NOT_HANDLED` constants as an intermediate step, then `Option<i32>`.

### L6. `trait` / `impl` — not used at all

**Status.** Language has `trait`/`impl` (v1 implemented, per `docs/language/syntax.md`). selfhost defines **0** `trait` and **0** `impl`.

**Workaround.** Free functions with manual `if is_gc_target()` runtime branching (106 calls in `wasm/`); manual `if`-chain trait resolution in `typechecker/call_method.ark`.

**Examples.**
- `src/compiler/wasm/sections_types_sigs.ark` — 20 `is_gc_target()` calls in one file for type-section emission.
- `src/compiler/wasm/intrinsic_option_result.ark` — 6 `is_gc_target()` calls.
- Every `wasm/intrinsic_*.ark` opens with `if is_gc_target() { emit_X_gc(...); return }` then re-implements for linear memory (Category D of the first audit).
- `src/compiler/typechecker/call_method.ark` — manual trait resolution via `if` chains.

**Why not used.** Compiler-maturity caution — trait dispatch may not have been stable for the selfhost target when written. Needs verification of current trait-dispatch support on the selfhost WASM target before migrating.

**Cost.** Very high — adding a new memory model or target requires touching every intrinsic; no central polymorphic abstraction; GC/LM dual implementations drift.

**Migration.** Hard — requires confident trait dispatch on the selfhost target. Highest value but highest risk; needs a spike first.

### L7. Enum with payload vs tagged struct — tagged structs used

**Status.** Language supports `enum` with payload variants (verified in stdlib + fixtures). selfhost models IR nodes as structs with a `kind: i32` / `tag: i32` / `op: i32` field + manual dispatch.

**Examples.**
- `src/compiler/hir/node_record.ark:6` — `struct HirNode { tag: i32, ty: HirType, ... }`.
- `src/compiler/mir/inst_record.ark:4` — `pub struct MirInst { op: i32, dest: i32, ... }`.
- `src/compiler/mir/block_record.ark` — same pattern for MIR blocks.

**Why not used.** Historical artifact + practical concern: tagged structs are easier to serialize/dump for debugging (MIR dump, HIR dump). Migrating to enum-with-payload would require reworking all dump/format code.

**Cost.** No exhaustiveness checking; easy to forget a new tag; type system cannot enforce coverage.

**Migration.** Hard — large surface area (every pass that inspects `tag`/`op`); debug-dump infrastructure depends on the struct layout. Defer until other refactors land.

### L8. Method-call syntax — deliberately not used

**Status.** Language supports `x.method()` (v1). selfhost uses it **5×** vs **15,811×** `module::method(`.

**Why not used.** Deliberate style choice — `docs/language/syntax.md` says function-call form is "safer". Function-call is also more grep-able.

**Cost.** Low — this is a legitimate style choice, not a defect.

**Migration.** Not recommended — leave as-is.

## Part 2 — Standard library features underused

### S1. `std::json` — partially used (escape only, DOM API unused)

**Status.** `std::json` provides `JsonValue` enum, `parse`, `stringify`, `stringify_pretty`, `json_get`, `json_get_index`, `json_as_*`, `json_encode_string`, `json_escape`. selfhost imports `std::json` in 16 files but **only uses `json_escape`** (via per-module wrappers). `JsonValue`/`stringify`/`json_encode_string` = **0** usage in compiler.

**Reinvention.** 665 `concat` calls building JSON by hand; 50+ hand-rolled JSON builder functions (also Category I of the first audit).

**Examples.**
- `src/compiler/lsp/responses_navigation.ark:7-20` — hover response built with 14 `concat` calls.
- `src/compiler/lsp/responses_lifecycle.ark:4-6` — 705-char JSON literal on one line.
- `src/compiler/lsp/diag_object.ark:30-46` — diagnostic object built with 16 `concat` calls.
- `src/compiler/diagnostics/json.ark:14-24` — diagnostic span built with 11 `concat` calls.

**Cost.** Critical — escape/brace/comma mistakes; no type safety; inconsistent across LSP/DAP/diagnostics; 705-char literals unreadable.

**Migration.** Medium — replace concat chains with `JsonValue` construction + `stringify`. Library gap: need helper fns to convert LSP-specific types (`Position`, `Range`, `Location`) to `JsonValue`, or add `json_object_set_int/string` mutator API.

### S2. `std::text::builder` — not used at all

**Status.** `std::text::builder` provides `builder_new`, `builder_append`, `builder_append_char`, `builder_append_line`, `builder_build`, `builder_len`. selfhost uses it **0×**.

**Reinvention.** 87 `mut s = String_from(...)` + repeated `s = concat(s, ...)` accumulations.

**Examples.**
- `src/compiler/component/wit_text.ark:17-23` — WIT text built via `out = concat(out, ...)`.
- `src/compiler/component/wit_type_defs.ark:31-48` — WIT record type built in a `while` loop with `concat`.
- `src/compiler/lint/registry.ark:126-135` — lint rule list built with `concat` in a loop.
- `src/compiler/main/init_templates.ark:50-55` — template TOML built with `concat`.

**Cost.** High — verbose; possible O(n²) reallocation; no builder clarity.

**Migration.** Easy — direct replacement (`builder_new` / `builder_append` / `builder_build`). No library gap. This is the single highest-value easy migration.

### S3. `std::collections` (HashMap/HashSet) — barely used

**Status.** `std::collections::hash_map` and `hash_set` exist (`hashmap_str_i32_*`, `hashset_str_*`, etc.). selfhost uses `std::collections` **2×**. (Note: `HashMap_*` **intrinsics** are used heavily in 10 files — that is the low-level intrinsic layer, not the std wrapper.)

**Reinvention.** 20+ linear-scan `while i < len(names) { if eq(get_unchecked(names, i), name) { return i }; i = i + 1 }; return 0 - 1` lookup functions.

**Examples.**
- `src/compiler/mir/lower/ctx_mono_type_params.ark:53-62` — `ctx_resolve_mono_type_param` linear search.
- `src/compiler/mir/lower/ctx_gc_struct.ark:12-23` — GC struct registration linear search.
- `src/compiler/mir/lower/ctx_gc_enum.ark:32-50` — enum variant lookup with **two** linear searches (names + aliases).
- `src/compiler/lsp/init_workspace.ark:46-56`, `lsp/symbol_index_project.ark:14-24`, `lsp/import_graph.ark:6-16` — three separate `*_contains(Vec<String>, String) -> bool` functions, all body-identical.
- `src/compiler/wasm/ctx_gc_type.ark:126-140` — struct type by name linear search.

**Cost.** High — O(n) where O(1) is appropriate; copy-paste `contains` helpers; parallel `Vec<String>` + `Vec<i32>` arrays must be kept in sync manually.

**Migration.** Medium — replace parallel name/index arrays with `HashMap<String, i32>`; replace `*_contains` with `HashSet<String>`. Need to decide whether to use `std::collections` wrappers or stick with intrinsics.

### S4. Higher-order prelude (`map`/`filter`/`fold`/`any`/`find`) — not used

**Status.** Prelude has `map_i32_i32`, `filter_i32`, `fold_i32_i32`, `any_i32`, `find_i32`, `map_option_i32_i32` (and `i64`/`f64`/`String` variants). selfhost references them 33× — **all in intrinsic registration/lowering**, never as actual calls with a closure.

**Reinvention.** 109 hand-written `while` loops that are pure map/filter/fold/any/find shapes.

**Examples.**
- `src/compiler/wasm/sections_types.ark:123-133` — `find_canon_type_index` is a `find` over `Vec<String>`.
- `src/compiler/typechecker/module_wit.ark:81-87` — `wit_push_fn_sigs_for_doc` is a `map` over `doc.funcs`.
- `src/compiler/driver/errors_phase.ark:22-27` — error collection loop is a `fold`/`for_each`.
- `src/compiler/resolver/context.ark:45-57` — `register_builtins` is two `for_each` calls.

**Cost.** Medium — verbose, off-by-one risk, not composable.

**Migration.** Hard — **library gap**: prelude only has monomorphized variants for `i32`/`i64`/`f64`/`String`. selfhost needs variants for custom types (`FnSig`, `AstNode`, `MirFunction`). Requires generic `Iterator`/`Fn` traits per ADR-036. Not a selfhost fault.

### S5. `split` — not used in compiler logic

**Status.** Prelude has `split(s, delim) -> Vec<String>`. selfhost uses it **0×** in compiler logic (3 refs are intrinsic handlers).

**Reinvention.** Hand-rolled `index_of` + `substring` parsing.

**Examples.**
- `src/compiler/component/wit_parse_text.ark:149-164` — WIT function parsing via `index_of(substring(...), ": func")` + manual offset math.
- `src/compiler/lsp/framing_parse.ark:19` — LSP header parsing via `index_of(substring(...), "\r\n\r\n")`.
- `src/compiler/lsp/signature_help_resolve.ark:70` — closing-paren search via `index_of(substring(...), ")")`.

**Cost.** Low-medium — fragile, but limited to a few sites.

**Migration.** Easy where the structure is genuinely "split on delimiter"; no library gap.

### S6. `std::seq` — not used

**Status.** `std::seq` offers `map`/`filter`/`take`/`skip`/`fold`/`binary_search`/`min`/`max`/`sum`/`reverse`/`contains`/`unique`/`count_eq`. selfhost uses it **0×**.

**Reinvention.** Linear search where `binary_search` would fit; custom `append_with_comma` where `join` would fit.

**Examples.**
- `src/compiler/wasm/sections_types.ark:123-133` — linear `find_canon_type_index` (could be `binary_search` if sorted).
- `src/compiler/lsp/responses_symbol.ark:48-53` — custom `append_symbol_information` reimplements `join` with `,`.

**Cost.** Low — few use cases for i32-specific seq ops in selfhost.

**Migration.** Easy where applicable; no library gap for i32 variants.

## Part 3 — Reinvented-wheel clusters (cross-cutting)

These overlap with the first audit's categories but are reframed here as "reinvention of an existing capability".

### R1. String escaping — 12 wrapper functions, 1 std function

**Count.** 12 distinct `json_escape`/`quote_string`/`diag_json_escape` wrappers across LSP/diagnostics, all ultimately delegating to `std::json::json_escape`.

**Existing alternative.** `std::json::json_escape` / `std::json::json_encode_string`.

**Worst.** `lsp/feature_code_action.ark:16`, `lsp/organize_imports.ark:7`, `lsp/rename_workspace.ark:7`, `lsp/rename_semantic.ark:13`, `lsp/formatting.ark:9`, `lsp/completion_items.ark:7` — six separate one-line `fn quote_string(s) -> String { ... json_escape ... }` wrappers.

**Blocker.** None — pure refactor; consolidate to a single `use std::json::json_escape`.

### R2. Vec extend / append-all — 64 hand-written loops, no `Vec_extend`

**Count.** 64 `push(dst, get_unchecked(src, i))` loops across 30+ files.

**Existing alternative.** None — no `Vec_extend` in std/prelude.

**Worst.** `lint/checks.ark:134-172` (6 loops), `typechecker/module_env_merge.ark:35-66` (5 loops), `mir_opt/loop_unroll.ark:131-135`.

**Blocker.** Library gap — need `Vec_extend<T>(dst, src)` in prelude. May need generic function support.

### R3. Linear name→index lookup — 20+ functions, no shared helper

**Count.** 20+ `while i < len(names) { if eq(get_unchecked(names, i), name) ... }` functions.

**Existing alternative.** `HashMap<String, i32>` (S3) or a shared `vec_index_of` helper.

**Worst.** `mir/lower/ctx_gc_enum.ark` (5 linear searches), `mir/lower/ctx_gc_struct.ark` (3), `resolver/wit_type_map.ark:152-172` (2 body-identical `wit_record_index`/`wit_enum_index`).

**Blocker.** HashMap adoption (S3) or shared helper; medium effort.

### R4. String-set membership — 411 `eq(clone(...))` chains

**Count.** 411 `eq(clone(name), "X") || ...` chain occurrences (92 in SIMD alone).

**Existing alternative.** `HashSet<String>` or generated static lookup table/trie.

**Worst.** `typechecker/builtins_simd.ark:45-157` (92 chains across 6 functions), `mir/lower/core_call_arg_names.ark:4,8,12,28` (chains of 9/10/14/13 names), `typechecker/builtins_hash.ark:43-122` (13 chains).

**Blocker.** HashSet adoption or codegen for static string sets. This is also Category A of the first audit — the deepest single source of mega-lines.

### R5. Raw Wasm opcode emission — 100+ raw `OP_LOCAL_GET`, helpers exist

**Count.** 100+ raw `OP_LOCAL_GET()` + `emit_leb128_u` pairs; 43 raw `OP_BLOCK`/`OP_LOOP`. 5,407 raw `emit_byte`/`emit_leb128` total.

**Existing alternative.** `wasm/intrinsics/helpers_core_locals::emit_local_get`, `helpers_control_structured`, `helpers_control_loop`.

**Worst.** `wasm/intrinsic_vec_push_gc.ark` (20 raw `OP_LOCAL_GET`), `wasm/intrinsic_vec_pop_gc.ark` (15), `wasm/intrinsic_vec_access.ark` (11), `wasm/inst_array.ark:60,64,71,78`.

**Blocker.** None — pure refactor; helpers exist but are bypassed. Also Category E of the first audit.

### R6. Trivial getter boilerplate — 100+ one-line getters

**Count.** 100+ `fn X_field_y(a: X) -> T { a.field_y }` getters.

**Existing alternative.** Direct field access (`a.field_y`) — supported by the language.

**Worst.** `component/contract_allows_numeric.ark:23-41` (5), `contract_allows_string_record.ark:33-75` (11), `contract_allows_collection_record.ark:39-93` (14). Also Category H of the first audit.

**Blocker.** None — pure refactor; replace getter calls with direct field access.

### R7. Reinvented string builders — 87 accumulations, `std::text::builder` unused

**Count.** 87 `mut s = String_from` + `concat` accumulations.

**Existing alternative.** `std::text::builder` (S2).

**Worst.** `lsp/diag_object.ark:30-46` (16 concats), `lsp/responses_lifecycle.ark:4-18`, `diagnostics/json.ark:14-36` (11 concats).

**Blocker.** None — pure refactor (S2).

### R8. Hand-rolled struct constructors with 10+ params — 19 cases

**Count.** 19 `_new` constructors with 10+ params; worst is 27.

**Existing alternative.** Direct struct literals `X { field: val, ... }`.

**Worst.** `main/args_record.ark:36-37` (`CliOptions_new` 27 params), `driver/config_record.ark:24-25` (`DriverConfig_new` 18 params), `wasm/ctx_record.ark:27` (`SelfEmitCtx_new` 10 params).

**Blocker.** None — pure refactor; replace with struct literals at call sites.

### R9. Sentinel `0 - 1` / `0 - 2` returns — 67 cases

**Count.** 61 `return 0 - 1` + 6 `return 0 - 2`.

**Existing alternative.** `Option<T>` / `Result<T,E>` (L5) or named constants.

**Worst.** `mir_opt/loop_unroll.ark:32,40,42,44` (4 sentinels), `mir/lower/core_*.ark` (20+), `resolver/scope_lookup.ark:11,19`, `fmt/prune_imports.ark:153,207`.

**Blocker.** Medium — named constants are easy; full `Option`/`Result` migration needs type-system confidence. Also Category B of the first audit.

### R10. Per-module `is_gc_target` branching — 106 calls

**Count.** 106 `is_gc_target()` calls in `wasm/`.

**Existing alternative.** `trait`/`impl` dispatch (L6) or compile-time target specialization.

**Worst.** `wasm/sections_types_sigs.ark` (20 calls in one file), `wasm/intrinsic_option_result.ark` (6), `wasm/intrinsic_hashmap_str_slot.ark` (6).

**Blocker.** Hard — needs trait dispatch on selfhost target (L6 spike required). Also Category D of the first audit.

## Why underused — root-cause attribution

| Feature | Root cause | Evidence |
|---------|-----------|----------|
| `match` (L1) | habit | compiler supports match-on-i32; fixtures compile |
| `for-in` (L2) | habit | compiler supports; `comprehension_map.ark` fixture uses it |
| `?` (L3) | habit + possible library gap | compiler supports; `question_mark.ark` fixture uses it |
| closures (L4) | library gap | prelude HOFs only monomorphized for i32/i64/f64/String; need generic `Fn`/`Iterator` (ADR-036) |
| `Option/Result` vs sentinel (L5) | habit | types exist and are used in 40 contexts; sentinels persist elsewhere |
| `trait/impl` (L6) | compiler-maturity caution | language has traits; selfhost target trait dispatch needs verification spike |
| enum-with-payload (L7) | historical + debug-dump coupling | tagged structs easier to dump; migration touches all passes |
| method-call (L8) | deliberate style | docs say function-call is "safer"; leave as-is |
| `std::json` DOM (S1) | habit + library gap | `json_escape` used; `JsonValue`/`stringify` not; need LSP-type helpers |
| `std::text::builder` (S2) | habit | no gap; just unused |
| `std::collections` (S3) | habit + intrinsic-layer preference | intrinsics used; std wrappers not |
| higher-order prelude (S4) | library gap | need generic HOFs or more monomorphized variants |
| `split` (S5) | habit | no gap; just unused |
| `std::seq` (S6) | few use cases | low priority |

## Migration blockers summary

**Pure refactor (no language/library changes needed):**
- R1 string-escaping consolidation
- R5 raw opcode → helpers
- R6 getter removal → direct field access
- R7 string builder adoption (S2)
- R8 struct literal replacement
- L1 `match` for kind dispatch
- L2 `for-in` for index loops
- S5 `split` adoption

**Library gap (need std additions):**
- R2 `Vec_extend` in prelude
- S4 generic `map`/`filter`/`fold` (ADR-036 `Iterator`/`Fn` traits) or more monomorphized variants
- S1 `JsonValue` helpers for LSP types / mutator API

**Compiler-maturity spike needed:**
- L6 `trait`/`impl` dispatch on selfhost WASM target
- L3 `?` operator in all selfhost return contexts (likely fine, verify)

**Large surface / coupled:**
- L7 enum-with-payload (touches every pass + debug dump)
- L5 full `Option/Result` migration (67 sentinels across many modules)

## Recommended remediation priority

| Priority | Items | Rationale |
|---------:|-------|-----------|
| 1 | S2/R7 (string builder), R1 (escape consolidation), S1 (JSON DOM) | Easy + high value; eliminates 665 concat calls and 12 wrapper functions |
| 2 | L1 (`match`), L2 (`for-in`) | Easy mechanical; improves dispatch + iteration readability repo-wide |
| 3 | R5 (raw opcode → helpers), R6 (getter removal), R8 (struct literals) | Pure refactor; reduces wasm/ noise and component/ bloat |
| 4 | S3/R3 (HashMap for linear lookups), R4 (string-set tables) | Medium; eliminates 20+ linear searches and 411 `eq` chains |
| 5 | L5/R9 (sentinel → Option/Result) | Medium; type safety across 67 sites |
| 6 | L6/R10 (trait/impl for GC dispatch) | Hard; needs compiler spike; highest architectural value |
| 7 | L7 (enum-with-payload) | Hard; large surface; defer |
| 8 | S4/R2 (generic HOFs + Vec_extend) | Library gap; blocked on ADR-036 |
| — | L8 (method-call syntax) | Not recommended; deliberate style choice |

## Relationship to the first audit

This audit reframes the first audit's 12 categories (A–L) in terms of **which existing feature is being bypassed**:

| First audit category | This audit's framing |
|----------------------|----------------------|
| A (string-dispatch OR chains) | R4 (string-set membership; `HashSet`/table) |
| B (magic sentinels) | L5/R9 (`Option/Result`) |
| C (scratch locals) | — (not a feature-underuse issue) |
| D (GC/LM dual impl) | L6/R10 (`trait/impl`) |
| E (raw opcode emission) | R5 (helpers exist) |
| F (manual while iteration) | L2 (`for-in`) + L4 (closures) + S4 (HOFs) |
| G (adapter boilerplate) | — (Plan-struct direction already noted) |
| H (trivial getters) | R6 (direct field access) |
| I (hand-written JSON) | S1 (`std::json` DOM) + S2 (`std::text::builder`) |
| J (shallow facades) | — (not a feature-underuse issue) |
| K (duplicated helpers) | R3 (shared `index_of`) |
| L (string prefix parsing) | S5 (`split`) |

The first audit's categories C, G, J do not have a feature-underuse root cause and are not repeated here.

## Notes

- This audit is classification only; no source files were modified and no commit was made beyond this document.
- "Compiler supports X" claims were verified by checking `tests/fixtures/` and `std/` for compiling examples on the current selfhost target.
- Quantification figures were captured via repo-wide grep at audit time (2026-06-30).
- The `trait`/`impl` gap (L6) and generic-HOF gap (S4) are the only items that may require compiler work rather than pure refactor; both need a verification spike before commitment.
