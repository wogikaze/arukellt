# Selfhost Code-Smell Audit — 2026-06-30

Audit scope: `src/compiler/` (selfhost compiler, ~90,000 lines across `wasm/`, `mir/`, `parser/`, `resolver/`, `typechecker/`, `hir/`, `corehir/`, `component/`, `driver/`, `lsp/`, `loader/`, `fmt/`, `lint/`, `analysis/`, `dap/`, `main/`, `compiler/`, `diagnostics/`).
**Status: complete (2026-06-30).** This is a classification/typology audit — no code changes were made.

Method: three parallel read-only subagent sweeps (wasm/; frontend+MIR; component/driver/lsp/loader/fmt/lint), cross-checked with repo-wide quantification (`eq(clone(...))` = 1,384 occurrences; raw `emit_byte`/`emit_leb128_*` = 5,407 occurrences; longest line = 1,683 chars).

## Executive summary

Twelve recurring bad-code categories were identified, ordered by severity × frequency below. The codebase already contains the *correct* solution patterns for several of them (Plan structs in `component/adapters_*_general.ark`, `helpers_*` modules in `wasm/`, predicate-based dispatch in `typechecker/`); the gap is that these good patterns have not been propagated across the whole codebase.

| ID | Category | Severity | Frequency | Primary location |
|----|----------|---------:|----------|------------------|
| A | String-dispatch giant `eq(clone(...))` OR chains | HIGH | very high | mir/lower, wasm, typechecker |
| B | Magic integer sentinels (`0 - 1`, `0 - 2`) | HIGH | high | mir/lower, resolver |
| C | Numbered scratch locals `s0..s11` | HIGH | very high (wasm) | wasm/intrinsic_* |
| D | GC vs linear-memory dual implementation copy-paste | HIGH | high (wasm) | wasm/intrinsic_* |
| E | Raw opcode/byte emission bypassing helpers | MEDIUM | very high (5,407) | wasm |
| F | Manual `while` + `get_unchecked` iteration | MEDIUM | very high | mir/lower, driver, lint |
| G | Copy-pasted adapter boilerplate | HIGH | very high | component/adapters_* |
| H | Trivial getter proliferation | MEDIUM | very high | component/contract_allows_* |
| I | Hand-written JSON string concatenation | MEDIUM | high | lsp/responses_*, lsp/diagnostics |
| J | Shallow facade forwarding functions | LOW | medium | mir/lower, typechecker |
| K | Duplicated helper functions | LOW | medium | mir/lower, resolver |
| L | Manual string prefix/suffix parsing | LOW | medium | mir/lower, resolver |

## Category A — String-dispatch giant `eq(clone(...))` OR chains

**Pattern.** Function-name dispatch written as `if eq(clone(name), "X") || eq(clone(name), "Y") || ... { ... }`. O(n) lookup, `clone()` allocation on every comparison, single lines exceeding 1,600 chars, and adding a new entry means hunting for the right place in a giant chain.

**Why bad.** Maintainability (additions are painful), readability (mega-lines), performance (repeated clone + linear scan), no compile-time validation of dispatch targets.

**Examples.**
- `src/compiler/mir/lower/call_types.ark:20` — 33 callee names OR-chained on one 1,474-char line.
- `src/compiler/mir/lower/ctx_fn_return_vt.ark` (1,683-char line) — 40+ intrinsic names OR-chained; longest line in the codebase.
- `src/compiler/typechecker/builtins_simd.ark:45-51` — 92 `if eq(clone(name), String_from("__simd_...")) { return true }` one-per-line.
- `src/compiler/mir/lower/call_type_hash.ark` — hashmap-name OR chains spanning multiple lines.

**Quantification.** 1,384 `eq(clone(...))` occurrences. Top files: `wasm/call_simd_native.ark` (156), `wasm/call_simd_scalar.ark` (132), `typechecker/builtins_simd.ark` (92), `typechecker/builtins_simd_lanes.ark` (87), `mir/module_host_calls.ark` (45).

**Fix direction.** Declare builtins as a table `[(name, kind, handler_id), ...]` and dispatch by table lookup or hash. Eliminate `clone()` either by making `eq` borrow or by interning names once and comparing IDs.

## Category B — Magic integer sentinels (`0 - 1`, `0 - 2`)

**Pattern.** Raw `i32` sentinels flow through return paths: `0 - 1` means "not found"/"error", `0 - 2` means "not handled". No type safety, no documentation, indistinguishable from arithmetic results.

**Why bad.** Type safety zero; `-1` vs an arithmetic result is invisible; semantics are undocumented; easy to misuse the wrong sentinel.

**Examples.**
- `src/compiler/mir/lower/core_match.ark:17-18` — `return 0 - 1` for "too few children".
- `src/compiler/mir/lower/body_dispatch_expr.ark:29-96` — `0 - 2` used 5× as "not handled".
- `src/compiler/mir/lower/core_call_direct.ark:15-16` — `if payload_result != 0 - 2 { return payload_result }`.
- `src/compiler/resolver/scope_lookup.ark:11-45` — `return 0 - 1` 3× as "not found".

**Fix direction.** Introduce named constants `NOT_FOUND: i32 = 0 - 1` / `NOT_HANDLED: i32 = 0 - 2` per module. Long-term: migrate to an `Option<i32>` / `Result`-style wrapper.

## Category C — Numbered scratch locals `s0..s11`

**Pattern.** `let s0 = scratch_local(ctx, 0)` ... `s11`, then pass them positionally as `f(w, s0, s2, s3, s4)`. The meaning of `s7` is unknowable without tracing the whole function; copy-paste index bugs are easy.

**Why bad.** Maintainability (no semantic naming), bug-risk (wrong index on paste), readability (10+ unnamed params).

**Examples.**
- `src/compiler/wasm/intrinsic_string_replace_all.ark:27-38` — 12 locals `s0..s11`.
- `src/compiler/wasm/intrinsic_string_trim.ark:25-29` — `s0..s4` then `emit_trim_copy_result(w, s0, s2, s3, s4)`.
- `src/compiler/wasm/intrinsic_vec_access.ark:38-40` — inconsistent: `s_idx2`, `s_vec3` (named) mixed with unnamed.

**Fix direction.** Give each intrinsic's locals semantic names (`src_ptr`, `dst_ptr`, `len`, `idx`, `result`). Group into a `ScratchRegs` struct and use field access instead of positional args.

## Category D — GC vs linear-memory dual implementation copy-paste

**Pattern.** Each intrinsic opens with `if is_gc_target() { emit_X_gc(...); return }` then re-implements the same algorithm for linear memory with only the memory primitives swapped (`emit_gc_array_get` vs `emit_i32_load8_u`).

**Why bad.** Algorithm duplicated; fixing one path and forgetting the other is a standing risk; new intrinsics require writing two versions.

**Examples.**
- `src/compiler/wasm/intrinsic_string_pad.ark:19-45` — `emit_pad_left` paired with `emit_pad_left_gc` (77-129).
- `src/compiler/wasm/intrinsic_string_trim.ark:20-43` — `emit_trim_start` paired with `emit_trim_start_gc` (70-102).
- `src/compiler/wasm/intrinsic_string_basic.ark:34-50` — `emit_bool_to_string` / `_gc`.

**Fix direction.** Abstract memory access behind a trait-like interface (load_byte, store_byte, array_get, ...). Write each intrinsic once against the abstraction; provide GC and LM implementations of the abstraction.

## Category E — Raw opcode/byte emission bypassing helpers

**Pattern.** `emit_byte(w, opcodes::OP_LOCAL_GET())` + `emit_leb128_u(w, idx)` written by hand even where `helpers_core_locals::emit_local_get` exists. Also `emit_byte(w, 64)` for block-type void with no named constant.

**Why bad.** Readability (`emit_byte(w, 73)` is meaningless), maintainability (no single source of opcode numbers), inconsistency (helpers exist but are bypassed).

**Examples.**
- `src/compiler/wasm/intrinsic_math.ark:136-158` — "load length field" written as 9 raw lines.
- `src/compiler/wasm/intrinsic_parse_f64_scan.ark:6-16` — block/loop header raw despite `helpers_control_loop`.
- `src/compiler/wasm/intrinsic_memory.ark` — 500+ char deeply-indented raw emission lines.

**Quantification.** 5,407 raw `emit_byte`/`emit_leb128_*` occurrences repo-wide.

**Fix direction.** Enforce existing helpers; add missing helpers (block/loop header, field load); progressively replace raw usage.

## Category F — Manual `while` + `get_unchecked` iteration

**Pattern.** `let mut i = 0; while i < len(v) { let x = get_unchecked(v, i); ...; i = i + 1 }` boilerplate repeated everywhere, including `extend_vec`-style vector-concat loops rewritten each time.

**Why bad.** Boilerplate bloat, manual bounds management, duplicated logic.

**Examples.**
- `src/compiler/mir/lower/core_match_payload_bind_core.ark:20-23`
- `src/compiler/mir/lower/core_match_payload_info.ark:108-128`
- `src/compiler/driver/errors_phase.ark:31-72` — `resolve_errors_result` and `type_errors_result` are near-identical loop bodies duplicated.
- `src/compiler/lint/checks.ark:130-174` — six `extend_vec`-style concat loops in a row.

**Fix direction.** Add `foreach` / `extend` / `map` iteration helpers to std and replace the boilerplate.

## Category G — Copy-pasted adapter boilerplate (component/)

**Pattern.** Each `adapters_*.ark` function rewrites the same skeleton: `comp_new_core_module_writer → type/import/func/export/code sections → writer_bytes`, differing only in type codes (`0x7d`/`0x7f`) and parameter counts.

**Why bad.** Dozens of hand-copied functions; copy mistakes; large code volume; each variant tested separately.

**Examples.**
- `src/compiler/component/adapters_numeric.ark:7-21` — `comp_f32_unary/binary/to_i32/i32_to_f32` differ by one line.
- `src/compiler/component/adapters_option.ark:8-56` — `option_i32_to_i32` / `i64_to_i64` / `i32_to_option_i32` share skeleton.
- `src/compiler/component/adapters_record.ark:22-70` — `point_distance` same skeleton again.

**Fix direction.** Define an `AdapterConfig` table and one generic emitter. `component/adapters_f32_general.ark` and `adapters_string_general.ark` already use `F32GeneralPlan` / `StringGeneralPlan` — extend that approach to all adapters.

## Category H — Trivial getter proliferation

**Pattern.** `fn X_allow_f32_square(a: X) -> bool { a.allow_f32_square }` — one-line getters generated for every struct field. `contract_allows_*` alone has dozens.

**Why bad.** Code bloat (3-5 lines per field), noise hiding real logic, manual sync on field add/remove. These getters then feed giant OR-chain expressions (e.g. `contract_type_support_named.ark` 968-char line).

**Examples.**
- `src/compiler/component/contract_allows_numeric.ark:23-41` — 5 getters.
- `src/compiler/component/contract_allows_string_record.ark:33-75` — 11 getters.
- `src/compiler/component/contract_allows_collection_record.ark:39-93` — 14 getters.
- `src/compiler/component/contract_type_support_named.ark` (968-char line) — OR-chain of these getters.

**Fix direction.** Use direct field access if the language supports it, or generate getters. Removing the getters also shortens the OR-chain expressions.

## Category I — Hand-written JSON string concatenation (lsp/)

**Pattern.** JSON responses built via `concat(body, String_from(",\"result\":{..."))` with `to_string` spliced in for numbers. Escape/comma/brace mistakes are easy; not type-safe; a 705-char JSON literal on one line exists.

**Why bad.** Error-prone escaping, no compile-time JSON validation, unreadable, hard to extend.

**Examples.**
- `src/compiler/lsp/responses_lifecycle.ark:4-7` — `build_initialize_response` with 705-char literal.
- `src/compiler/lsp/responses_navigation.ark:7-20` — hover response built with 14 `concat` calls.
- `src/compiler/lsp/diagnostics.ark:8-22` — diagnostics array concatenated in a loop.

**Fix direction.** Introduce a type-safe JSON builder (`std::json` with `JsonObject_set_string/int/object`) and build structuredly.

## Category J — Shallow facade forwarding functions

**Pattern.** `fn f(...) { g(...) }` — pure forwarding with no logic.

**Why bad.** Unnecessary indirection, code bloat, navigation friction, fuzzy module boundaries.

**Examples.**
- `src/compiler/mir/lower/core_match_payload.ark:8-10`
- `src/compiler/mir/lower/comprehension.ark:7-9`
- `src/compiler/typechecker/call_generic.ark:7-9`

**Fix direction.** Inline, or merge modules if the separation adds no value.

## Category K — Duplicated helper functions

**Examples.**
- `core_match_arm_has_guard` defined identically in `mir/lower/core_match_arm_branch.ark:14` and `core_match_arm_last.ark:12`.
- `src/compiler/resolver/wit_type_map.ark:152-172` — `wit_record_index` and `wit_enum_index` are body-identical.

**Fix direction.** Move shared helpers to a common module and import.

## Category L — Manual string prefix/suffix parsing

**Examples.**
- `src/compiler/mir/lower/core_match_payload_info.ark:68-82` — `starts_with("Option_")` → `substring(7, len)` to decompose type names.
- `src/compiler/resolver/expr_ident.ark:21-24` — `name_len >= 8 && substring(0,8)=="Vec_new_"`.

**Fix direction.** Parse type names into structured data (tag + parameters) early; stop decomposing strings at use sites.

## Good patterns to preserve

- **Modular file organization** — `mir/lower/core_match_*.ark` split by responsibility; keeps compilation units manageable.
- **Struct-based context passing** — `LowerCtx`, `ResolveCtx`, `SelfEmitCtx` avoid giant parameter lists.
- **Predicate-based dispatch** — `ast_expr_predicates::node_is_*()` is cleaner than direct kind comparison.
- **`helpers_control_structured` / `helpers_core_locals` / `helpers_memory_access`** — the correct answer to Category E; raw emission should converge on these.
- **Plan-struct adapter generation** — `adapters_f32_general.ark` / `adapters_string_general.ark` (`F32GeneralPlan` / `StringGeneralPlan`) already show the correct answer to Category G; extend to all adapters.
- **Separate `_record` modules** — types have dedicated data-layout files (`symbol_record.ark`, `node_record.ark`), separating interface from implementation.

## Recommended remediation priority

| Priority | Categories | Rationale |
|---------:|-----------|-----------|
| 1 | A, B | Pervasive across the codebase; bug hotbed; type-safety gap |
| 2 | C, D | Direct hit on wasm/ maintainability; blocks intrinsic extension |
| 3 | G, H | Order-of-magnitude component/ code reduction; Plan direction already exists |
| 4 | E, F, I | Mechanically fixable by adding helpers |
| 5 | J, K, L | Local; handle opportunistically during other refactors |

## Notes

- This audit is classification only; no source files were modified and no commit was made.
- Line numbers cite the state at 2026-06-30; minor drift may occur as files are edited.
- Quantification figures (`eq(clone(...))` = 1,384; raw emit = 5,407) were captured via repo-wide grep/awk at audit time.
