# Stdlib Implementation Quality Audit — Issue 514

**Status**: complete
**Date**: 2026-04-22
**Scope**: hash family, parser family, collection family, host facade family
**Source issue**: issues/open/514-stdlib-implementation-quality-audit.md

## Overview

This document consolidates the quality audit findings for the stdlib families
named in issue 514. Detailed evidence matrices live in:

- `docs/stdlib/modernization/514-quality-audit-matrix.md` — hash, collection, json, toml, fs
- `docs/stdlib/modernization/514-parser-host-quality-audit.md` — deep-dive on json, toml, fs

The present document summarizes each family on the 4-axis framework and presents
a single prioritized follow-up list.

## Risk scale

| Rank     | Meaning                                                                       |
|----------|-------------------------------------------------------------------------------|
| Critical | Likely user-visible misbehavior or contract break under ordinary inputs.      |
| High     | Real production risk or severe quality gap; schedule follow-up soon.          |
| Medium   | Noticeable limitation or footgun, but not a frequent hard failure.            |
| Low      | Acceptable for now; document and revisit later.                               |

---

## Hash family audit summary

Modules: `std::core::hash`, `std::collections::hash`

### Correctness axis

- `hashmap_get` returns `0` for both missing keys and stored zero values; callers
  cannot distinguish the two cases without using `hashmap_get_option`.
- `hashmap_set` silently stops inserting when the table is full; no error is
  returned to the caller.
- Fixed capacity with no rehash path means high-occupancy maps silently lose
  inserts rather than growing.
- `hash_i32` is defined independently in both `std/core/hash.ark` and
  `std/collections/hash.ark` with different mixing strategies; the canonical
  policy is undefined.

### Collision axis

- `std::core::hash` — `hash_i32` uses lower-byte multiplicative mix only; high
  bits are not incorporated, leading to above-average clustering for aligned
  integer key spaces.
- `hash_string` applies sign normalization (`abs`) after each byte mix, losing
  entropy accumulated in the negative range.
- `combine` / `hash_combine` is `h1 * 31 + h2` — no avalanche, no domain
  separation.  Composite keys compound upstream collision pressure.
- Collection-side `hash_i32` uses `key * 1000003 + abs(key)` — a different
  distribution from the core helper; independent drift makes guarantees harder
  to preserve.

### Performance axis

- Linear probing with no load-factor cap; cluster length grows unbounded with
  occupancy.
- `hashmap_remove` rebuilds the entire table on each deletion (O(n) per call).
- `hashset_str_*` variants use a Vec of Strings with linear scan — name is
  "HashSet" but behaviour is array search.

### Contract axis

- The monomorphic `i32`-only contract is stated in module comments but not
  enforced at the type level; callers may misread API names as generic.
- Flat `Vec<i32>` layout `[cap, size, keys…, values…, flags…]` is documented
  in comments only — no machine-checkable invariant or version tag.
- `hashmap_get_option` exists alongside `hashmap_get`, splitting the API into
  two overlapping zero-handling stories with no deprecation note.

**Overall priority**: P0 (correctness and collision critical)

---

## Parser family audit summary

Modules: `std::json`, `std::toml`

### `std::json`

#### Correctness

- `parse` accepts the first recognizable JSON value and does not exhaust the
  input; trailing garbage is silently ignored.
- `json_get` / `json_get_index` use substring heuristics over stored raw text;
  escaped delimiters and nested structures can produce incorrect results.
- `stringify_pretty` is documented as pass-through — no formatting is applied,
  contradicting the name.

#### Collision

- No hash-style collision surface.  The risk is parser robustness: adversarial
  or deeply nested inputs trigger repeated re-scanning rather than a hard budget.

#### Performance

- Arrays and objects store raw JSON text; every nested access re-parses the
  entire stored literal.  Lookup cost scales with source size, not result size.

#### Contract

- Two number handling paths coexist: legacy `json_parse_i32` and the structured
  `JsonValue` path.  Acceptance rules and error cases differ between paths.
- Raw-text storage is an implementation workaround, not a stable API promise,
  but it is not labelled as temporary at the API boundary.

**Overall priority**: P0

### `std::toml`

#### Correctness

- `toml_parse` returns `Ok` for almost any non-empty input, including
  structurally invalid TOML.  Callers cannot detect malformed documents.
- `toml_parse_value` classifies unrecognised bare values as integers; datetimes
  and unsupported numeric forms are silently misclassified rather than rejected.

#### Collision

- No collision surface.  Robustness risk: unsupported TOML constructs (table
  headers, arrays of tables) are accepted but silently produce wrong results.

#### Performance

- `toml_get` and `toml_table_keys` rescan the raw source on every call;
  repeated lookup scales with source size.

#### Contract

- Module name and doc header say "parser/serializer" while only shallow
  `key = value` extraction is implemented.  Grammar limits are not communicated
  at import time.

**Overall priority**: P0

---

## Host facade family audit summary

Module: `std::fs`

### Correctness

- `exists` is implemented as a readability probe (`is_ok(read_file(path))`);
  directories, write-only files, and unreadable paths return `false` even if
  they exist.  The function name implies path-existence semantics.

### Collision

- No hash or parser collision surface.

### Performance

- `read_string`, `write_string` are thin intrinsic facades with no local
  overhead.
- `exists` performs a full file read to check existence — unnecessary I/O for
  the common existence-check case.

### Contract

- `exists` is documented as a "read probe / readable-file check" in comments,
  but the function name and common expectations diverge from that contract.
- The module header explicitly calls out a WASI P2 migration plan and points at
  `std::host::fs`, but the `std::fs` namespace still reads as a general
  filesystem facade.

**Overall priority**: P1

---

## 4-axis prioritized follow-up list

Priority assignments use: Correctness (C), Performance (P), Collision (K),
Contract (T).

### P0 — Critical, schedule immediately

1. **Hash collection safety — C + K**
   `std::collections::hash`: add explicit load-factor management and a
   non-silent insertion contract (resize or failure return); replace
   rebuild-based deletion with tombstones or backward-shift deletion.
   Evidence: `std/collections/hash.ark:90-117`, `211-233`.

2. **TOML parse contract — C + T**
   `std::toml`: replace unconditional-success `toml_parse` with real failure
   cases for malformed input; document the supported subset explicitly at
   import time rather than in inline comments.
   Evidence: `std/toml/mod.ark:88-115`.

3. **JSON top-level parse contract — C + T**
   `std::json`: require full-input consumption in `parse`; add rejecting
   fixtures for trailing non-whitespace content and unsupported numeric forms.
   Evidence: `std/json/mod.ark:187-230`.

### P1 — High risk, schedule in the next sprint

1. **JSON nested access strategy — P + T**
   Replace raw-text substring lookup in `json_get` / `json_get_index` with
   structural scanning or a cached parsed-node representation so repeated
   nested access is not re-parsing the same content.
   Evidence: `std/json/mod.ark:254-320`.

2. **Hash core mixer redesign — K + C**
   Replace `hash_i32`, `hash_string`, and `combine` in `std::core::hash` with
   an audited non-cryptographic policy (e.g., FNV-1a without mid-stream sign
   normalization, and a SipHash-style combiner).  Add collision-distribution
   smoke tests.
   Evidence: `std/core/hash.ark:4-47`.

3. **Unify hash policy ownership — T**
   Remove the collection-local `hash_i32` and delegate to `std::core::hash`
   so there is exactly one audited mixer in the codebase.
   Evidence: `std/collections/hash.ark:34-37`.

4. **`std::fs::exists` semantics — C + T**
   Split or rename the current readability probe so callers are not misled by
   the `exists` name; expose separate `is_readable` / `path_exists` helpers
   once the WASI P2 metadata intrinsics land.
   Evidence: `std/fs/mod.ark:27-32`.

### P2 — Medium risk, document and schedule when P1 is done

1. **`std::toml` / `std::json` documentation alignment — T**
   Make module-level docs state the supported subset and temporary-bridge
   status explicitly before either surface expands further.

2. **`std::fs` naming alignment with `std::host::*` rollout — T**
   Add a visible deprecation notice or migration pointer at the `std::fs`
   module boundary so call sites are aware of the planned transition.

3. **`hashset_str_*` naming accuracy — P + T**
    Rename or replace linear-scan string set helpers to reflect their actual
    O(n) behaviour; the `hashset` prefix implies O(1) average, which is not
    delivered.
    Evidence: `std/collections/hash.ark:336-377`.

4. **Family-wide adversarial fixtures — C + K**
    Add property, differential, and adversarial test fixtures covering: hash
    collision distribution, parser rejection for malformed inputs, and
    collection occupancy edge cases.

---

## References

- `docs/stdlib/modernization/514-quality-audit-matrix.md`
- `docs/stdlib/modernization/514-parser-host-quality-audit.md`
- `issues/open/514-stdlib-implementation-quality-audit.md`
- `issues/done/044-std-collections-hash.md`
- `issues/done/392-stdlib-error-result-conventions.md`
