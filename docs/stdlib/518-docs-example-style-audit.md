# Stdlib docs example style audit (#518)

**Status**: complete  
**Created**: 2026-04-22  
**Issue**: [#518](../../issues/open/518-stdlib-docs-examples-as-canonical-style.md)  
**Depends-on**: #513 (done), #517 (done)  
**Author**: design-stdlib-audit agent

---

## 1. Style debt inventory

This section catalogues every example location that uses a deprecated prelude
helper, old monomorphic API, or other anti-pattern identified during the scan
of `docs/stdlib/`, `docs/cookbook/`, and `tests/fixtures/stdlib_*/`.

### 1a. docs/cookbook/

| File | Line(s) | Anti-pattern | Priority |
|------|---------|--------------|----------|
| `json-processing.md` | 63–71 | bare `concat(…, concat(…, …))` nesting; bare `println(…)` | High |
| `json-processing.md` | 40, 44 | `json::json_parse_i32` used directly (monomorphic scalar API) as if it returns int directly without Option/Result handling | Medium |
| `testing-patterns.md` | 76 | `unwrap(parse_i32(…))` — bare prelude `parse_i32` (Result form) instead of `core::convert::parse_i32` (Option form) | High |
| `testing-patterns.md` | 103 | `HashMap_new_String_i32()` — old monomorphic HashMap constructor | High |
| `collections-usage.md` | 12 | `HashMap_new_String_i32()` — old monomorphic HashMap constructor | High |
| `collections-usage.md` | 51–52 | `HashMap_new_String_i32()`, `HashMap_new_i32_String()` | High |
| `collections-usage.md` | 41, 139 | bare `println(concat(…))` / bare `println` without `use std::host::stdio` | High |
| `file-processing.md` | 18, 39, 44, 67, 73–74, 77, 103, 109 | bare `concat(…)` throughout; mixes `stdio::` for some calls but bare `concat` for string building | Medium |
| `wasm-binary.md` | 140 | `println(concat(to_string(wit::wit_type_id(…)), concat(": ", name)))` — bare `println`, bare `concat`, bare `to_string` | High |
| `wasm-binary.md` | all examples | `buf_new()`, `buf_push_u8()`, `buf_freeze()`, `bytes_len()` used without `bytes::` prefix — marked `skip-doc-check` but still anti-pattern if copied | Medium |

### 1b. docs/stdlib/modules/

| File | Line(s) | Anti-pattern | Notes |
|------|---------|--------------|-------|
| `wit.md` (generated) | typical usage block | `import std::wit` syntax and bare `wit_type_u32()` without module qualifier | Generated file; fix in curated prose or generator template |
| `json.md` (generated) | typical usage block | bare `json_stringify_i32(42)`, `json_parse_i32("42")` in typical-usage example; no `use std::json`; no `Result`/`Option` for parse | Generated file; fix in generator template |
| `io.md` (generated) | typical usage block | bare `println(…)`, bare `+` string concatenation (which may not be valid) | Generated file; fix generator template |
| `docs/stdlib/cookbook.md` | various | aggregates cookbook pages; inherits their debt | Regenerated |
| `docs/stdlib/reference.md` | various | inline examples mix qualified and bare calls | Review after generator pass |

### 1c. tests/fixtures/stdlib_*/

Fixtures are read-only inventory. Anti-patterns found are recorded for
migration tracking, not immediate modification.

| Fixture dir/file | Anti-pattern | Category |
|------------------|--------------|----------|
| `stdlib_io/i32_to_string.ark` | bare prelude `to_string(…)` instead of `convert::i32_to_string` | prelude direct call |
| `stdlib_io/println_multi.ark` | bare `stdio::println` OK, but no example of using `core::convert` helpers | style-ok |
| `stdlib_cli/cli_basic.ark` | `i32_to_string(…)`, `bool_to_string(…)` bare prelude | prelude direct call |
| `stdlib_collections_compiler/interner_basic.ark` | `i32_to_string(…)` bare | prelude direct call |
| `stdlib_collections_ordered/bitset_ops.ark` | `bool_to_string(…)` bare | prelude direct call |
| `stdlib_compiler/compiler_basic.ark` | `i32_to_string(…)` bare | prelude direct call |
| `stdlib_csv/csv_basic.ark` | `len(fields)`, `get_unchecked(fields, i)` bare prelude Vec helpers | prelude direct call |
| `stdlib_env/env_basic.ark` | `i32_to_string(len(a))` — bare `len`, bare convert | prelude direct call |
| `stdlib_hashmap/hashmap_basic.ark` | `HashMap_i32_i32_new()`, `HashMap_i32_i32_insert()` — monomorphic API | old monomorphic HashMap |
| `stdlib_hashmap/hashmap_string_i32.ark` | `HashMap_new_String_i32()` | old monomorphic HashMap |
| `stdlib_hashmap/hashmap_i32_string.ark` | `HashMap_new_i32_String()` | old monomorphic HashMap |
| `stdlib_hashmap/hashmap_string_string.ark` | `HashMap_new_String_String()` | old monomorphic HashMap |
| `stdlib_json/json_basic.ark` | `i32_to_string(…)`, `bool_to_string(…)` bare prelude | prelude direct call |
| `stdlib_wit/wit_basic.ark` | `i32_to_string(wit::wit_type_id(…))` bare prelude convert | prelude direct call |
| `stdlib_core/convert.ark` | mixes `convert::i32_to_string` (correct) with bare `i32_to_string` in Option branch | mixed style |
| `stdlib_string/string_concat.ark` | likely uses bare `concat(…)` | check |
| `stdlib_migration/old_api_compat.ark` | intentionally uses old API — this is a compatibility test fixture | keep as-is; mark compile-check |

---

## 2. Anti-patterns to avoid in user-facing examples

The following patterns must not appear in any newly written or updated
doc example, cookbook recipe, or fixture intended as a usage reference.

### AP-1: Bare prelude convert helpers

**Avoid:**

```ark
println(i32_to_string(n))
println(bool_to_string(flag))
let v = parse_i32(s)
```

**Prefer:**

```ark
use std::core::convert
use std::host::stdio
stdio::println(convert::i32_to_string(n))
stdio::println(convert::bool_to_string(flag))
let v: Option<i32> = convert::parse_i32(s)
```

Note: `core::convert::parse_i32` returns `Option<i32>` not `Result<i32, String>`.
Use a `match` or `unwrap_or` in examples; never silently discard the Option.

### AP-2: Bare `concat` for string building

**Avoid:**

```ark
let line = concat(a, concat(b, concat(": ", c)))
```

**Prefer:**

```ark
use std::text
let line = text::concat(text::concat(a, b), text::concat(": ", c))
// Or use a text::Builder when more than 2–3 parts are joined:
use std::text::builder
let b = builder::new()
builder::push(b, a)
builder::push(b, ": ")
builder::push(b, c)
let line = builder::finish(b)
```

### AP-3: Bare `println` / `print` without module qualification

**Avoid:**

```ark
println("hello")
print("x = ")
```

**Prefer:**

```ark
use std::host::stdio
stdio::println("hello")
stdio::print("x = ")
```

### AP-4: Old monomorphic HashMap constructors

**Avoid:**

```ark
let m = HashMap_new_String_i32()
let m = HashMap_i32_i32_new()
HashMap_i32_i32_insert(m, k, v)
```

**Prefer:**

```ark
use std::collections::hashmap
let m = hashmap::new_string_i32()
hashmap::insert(m, k, v)
```

(or the polymorphic `std::collections` API once available)

### AP-5: Bare `len` / `get_unchecked` on Vec in examples

**Avoid:**

```ark
let n = len(fields)
let x = get_unchecked(fields, i)
```

**Prefer:**

```ark
use std::collections::vec
let n = vec::len(fields)
let x = vec::get(fields, i)   // bounds-checked
```

### AP-6: Raw `import` keyword for stdlib modules

**Avoid:**

```ark
import std::wit
```

**Prefer:**

```ark
use std::wit
```

The `use` keyword is the current canonical form for module imports in Arukellt.

### AP-7: Unhandled `parse_*` return values

**Avoid:**

```ark
let n = unwrap(parse_i32(s))
let n = parse_i32(s)   // silently discards failure case
```

**Prefer:**

```ark
use std::core::convert
match convert::parse_i32(s) {
    Option::Some(n) => /* use n */,
    None            => panic("expected integer"),
}
// or:
let n = convert::parse_i32(s).unwrap_or(0)
```

### AP-8: Numeric WIT type IDs in isolation (std::wit specific)

**Avoid:**

```ark
let id: i32 = 4   // magic number for WitType::u32
```

**Prefer:**

```ark
use std::wit
let ty = wit::wit_type_u32()
let id = wit::wit_type_id(ty)   // explicit; reader can see source type
```

---

## 3. Update plan: std::wit, std::json, std::bytes, std::io families

### 3a. std::wit examples

**Locations affected:**
- `docs/stdlib/modules/wit.md` — generated; curated "Typical usage" block
- `docs/cookbook/wasm-binary.md` — line 140

**Changes needed:**

1. `docs/stdlib/modules/wit.md` typical-usage block:
   - Replace `import std::wit` with `use std::wit`
   - Replace bare `wit_type_u32()` with `wit::wit_type_u32()`
   - Add `use std::host::stdio` and `stdio::println` instead of bare `println`
   - Regen via `python3 scripts/gen/generate-docs.py` after curated prose fix

2. `docs/cookbook/wasm-binary.md` line 140:
   - Replace `println(concat(to_string(wit::wit_type_id(wit_ty)), concat(": ", name)))`
     with `stdio::println(text::concat(convert::i32_to_string(wit::wit_type_id(wit_ty)), text::concat(": ", name)))`
   - Add `use std::text`, `use std::core::convert`, `use std::host::stdio` to the snippet

3. `tests/fixtures/stdlib_wit/wit_basic.ark`:
   - Migration target: replace bare `i32_to_string` with `convert::i32_to_string`
   - Add `use std::core::convert`

### 3b. std::json examples

**Locations affected:**
- `docs/stdlib/modules/json.md` — generated; curated prose block
- `docs/cookbook/json-processing.md` — lines 40, 44, 63–71
- `tests/fixtures/stdlib_json/*.ark` — several files

**Changes needed:**

1. `docs/stdlib/modules/json.md` typical-usage block:
   - Add explicit `use std::json` at top of snippet
   - Replace bare `json_stringify_i32(…)` with `json::json_stringify_i32(…)`
   - Wrap `json_parse_i32` call in a `match` on `Option` or add explicit error path
   - Regen after prose fix

2. `docs/cookbook/json-processing.md`:
   - Lines 63–71: Replace nested `concat(…, concat(…))` with `text::concat` or builder
   - Line 71: Replace bare `println` with `stdio::println`
   - Lines 40, 44: Add Result/Option handling annotation or wrap in `match`

3. `tests/fixtures/stdlib_json/*.ark`:
   - `json_basic.ark`: replace bare `i32_to_string`, `bool_to_string` with `convert::*`
   - Other roundtrip fixtures: audit for bare convert calls

### 3c. std::bytes examples

**Locations affected:**
- `docs/cookbook/wasm-binary.md` — all examples (marked `skip-doc-check`)
- `docs/stdlib/modules/bytes.md` — generated reference

**Changes needed:**

1. `docs/cookbook/wasm-binary.md`:
   - All examples use bare `buf_new()`, `buf_push_u8()`, `bytes_len()` etc.
     without `bytes::` prefix. These are marked `<!-- skip-doc-check -->` but
     are still user-visible style references.
   - Add `bytes::` prefix to all `buf_*` and `bytes_*` calls.
   - Once fixed, remove `skip-doc-check` markers and make these compile-check targets.
   - Tracking issue: #461 (referenced inline); coordinate with that issue before removing skip.

2. `docs/stdlib/modules/bytes.md`:
   - Confirm generated typical-usage block uses `bytes::` prefix throughout.

### 3d. std::io family examples

**Locations affected:**
- `docs/stdlib/modules/io.md` — generated; curated "Typical usage" block
- `docs/cookbook/file-processing.md` — multiple lines
- `docs/cookbook/collections-usage.md` — lines 41, 139
- `tests/fixtures/stdlib_io/*.ark` — `i32_to_string.ark`, `println_multi.ark` etc.
- `tests/fixtures/stdlib_io_rw/*.ark`

**Changes needed:**

1. `docs/stdlib/modules/io.md` typical-usage block:
   - Replace bare `println(…)` with `stdio::println(…)` throughout
   - Replace `+` string concatenation (if present) with `text::concat`
   - Ensure `use std::host::stdio` is shown at top of each snippet

2. `docs/cookbook/file-processing.md`:
   - Replace all `concat(a, concat(b, c))` patterns with `text::concat` or builder
   - Already uses `stdio::` prefix in some places — unify the others
   - Add `use std::text` import to each affected snippet

3. `docs/cookbook/collections-usage.md`:
   - Lines 41, 139: replace bare `println(concat(…))` with `stdio::println(text::concat(…))`
   - Lines 12, 51–52: replace `HashMap_new_*` with `hashmap::new_*` (see AP-4)
   - Add missing `use` imports

4. `tests/fixtures/stdlib_io/i32_to_string.ark`:
   - Replace bare `to_string(…)` with `convert::i32_to_string(…)` and add `use std::core::convert`

---

## 4. Compile-check targets vs migration targets

### Compile-check targets (currently checked or should be)

These fixtures are run through the compiler as part of the CI test suite.
Style issues here are higher priority because they set a visible example.

| Path | Status | Notes |
|------|--------|-------|
| `tests/fixtures/stdlib_bytes/*.ark` | compile-check | style ok — uses `bytes::` prefix |
| `tests/fixtures/stdlib_io_rw/*.ark` | compile-check | uses `stdio::` correctly |
| `tests/fixtures/stdlib_core/*.ark` | compile-check | mostly correct; `convert.ark` has mixed style |
| `tests/fixtures/stdlib_json/*.ark` | compile-check | bare prelude convert calls — migrate |
| `tests/fixtures/stdlib_wit/*.ark` | compile-check | bare prelude convert call — migrate |
| `tests/fixtures/stdlib_text/*.ark` | compile-check | generally clean |
| `tests/fixtures/stdlib_option_result/*.ark` | compile-check | clean |
| `tests/fixtures/stdlib_seq/*.ark` | compile-check | clean |
| `tests/fixtures/stdlib_io/i32_to_string.ark` | compile-check | bare `to_string` — migrate |
| `tests/fixtures/stdlib_io/println_multi.ark` | compile-check | style ok |
| `tests/fixtures/stdlib_cli/*.ark` | compile-check | bare convert helpers — migrate |
| `tests/fixtures/stdlib_csv/*.ark` | compile-check | bare `len`/`get_unchecked` — migrate |
| `tests/fixtures/stdlib_env/*.ark` | compile-check | bare convert + bare `len` — migrate |
| `tests/fixtures/stdlib_collections_compiler/*.ark` | compile-check | bare convert — migrate |
| `tests/fixtures/stdlib_collections_ordered/*.ark` | compile-check | bare `bool_to_string` — migrate |

### Migration targets (style debt, not yet compile-checked or flagged skip)

These need style updates but are lower urgency or have blocking dependencies.

| Path | Reason | Blocker |
|------|--------|---------|
| `docs/cookbook/wasm-binary.md` | all examples skip-doc-check; bare buf_* calls | #461 must be resolved first |
| `docs/cookbook/json-processing.md` | bare concat, bare println | Phase 2 backfill |
| `docs/cookbook/collections-usage.md` | monomorphic HashMap, bare println | Phase 2 backfill |
| `docs/cookbook/file-processing.md` | bare concat chains | Phase 2 backfill |
| `docs/cookbook/testing-patterns.md` | monomorphic HashMap, bare parse_i32 | Phase 2 backfill |
| `docs/stdlib/modules/wit.md` (prose) | import keyword, bare calls | regen after prose fix |
| `docs/stdlib/modules/json.md` (prose) | bare calls, no Option handling shown | regen after prose fix |
| `docs/stdlib/modules/io.md` (prose) | bare println | regen after prose fix |
| `tests/fixtures/stdlib_hashmap/hashmap_basic.ark` | old monomorphic API | wait for collections facade work |
| `tests/fixtures/stdlib_hashmap/hashmap_string_i32.ark` | old monomorphic API | wait for collections facade work |
| `tests/fixtures/stdlib_hashmap/hashmap_i32_string.ark` | old monomorphic API | wait for collections facade work |
| `tests/fixtures/stdlib_hashmap/hashmap_string_string.ark` | old monomorphic API | wait for collections facade work |
| `tests/fixtures/stdlib_migration/old_api_compat.ark` | intentionally old API — keep | do not migrate; it is a compat test |

---

## Acceptance checklist

- [x] Style debt inventory created (Section 1: docs/cookbook, docs/stdlib/modules, tests/fixtures)
- [x] Anti-patterns defined (Section 2: AP-1 through AP-8)
- [x] Update plan for std::wit, std::json, std::bytes, std::io families (Section 3)
- [x] Compile-check vs migration targets distinguished (Section 4)

---

## See also

- [prelude-migration-inventory.md](prelude-migration-inventory.md) — implementation-level migration shelf list (from #513)
- [migration-guidance.md](migration-guidance.md) — deprecated API replacements reference
- `issues/open/518-stdlib-docs-examples-as-canonical-style.md` — parent issue
