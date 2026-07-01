# Issue 514: stdlib quality audit matrix

## Scope

This audit is bounded to the family surfaces named by issue 514:

- hash family: `std::core::hash`, `std::collections::hash`
- parser family: `std::json`, `std::toml`
- collection family: `std::collections::hash`
- host facade family: `std::fs`

The goal is not to fix implementations here. The goal is to record where current behavior is below expected stdlib quality, rank the risks explicitly, and leave follow-up slices that can be implemented without re-auditing the same files.

## Risk scale

| Rank | Meaning |
|------|---------|
| Critical | Likely user-visible misbehavior or contract break under ordinary inputs. |
| High | Real production risk or severe quality gap; follow-up should be scheduled soon. |
| Medium | Noticeable limitation or footgun, but not a frequent hard failure. |
| Low | Acceptable for now; document and revisit later. |

## Family audit matrix

| Family | Primary modules | Evidence snapshot | Correctness risk | Perf risk | Collision risk | Contract ambiguity | Overall priority |
|--------|------------------|-------------------|------------------|-----------|----------------|--------------------|------------------|
| Hash family | `std::core::hash`, `std::collections::hash` | `std::core::hash` uses simple arithmetic hashes with sign normalization; `std::collections::hash` uses fixed-capacity linear probing with no resize policy. | High | High | Critical | Medium | P0 |
| Parser family | `std::json`, `std::toml` | `std::json` reparses raw substrings on access and accepts partial parse surfaces; `std::toml` currently treats the original document as an opaque table and only supports shallow `key = value` extraction. | Critical | Medium | Low | Critical | P0 |
| Collection family | `std::collections::hash` | HashMap/HashSet are monomorphic `i32` containers backed by flat vectors; removal is rebuild-based and insertion does not guarantee progress once occupancy approaches capacity. | High | Critical | High | High | P0 |
| Host facade family | `std::fs` | `exists()` is explicitly a read-probe stub, while the module name and function names read like a stable filesystem contract. | Medium | Low | Low | Critical | P1 |

## Module findings

### `std::core::hash`

| Surface | Finding | Why it matters | Follow-up-ready action |
|---------|---------|----------------|------------------------|
| `hash_i32` | The implementation is a byte-wise multiplicative mix over absolute value. `-x` and `x` intentionally collapse to the same hash, except for edge cases around integer limits. | Signed integer domains lose entropy and invite avoidable clustering when negative and positive values coexist. | Create a focused issue to replace `hash_i32` with a stable signed-aware mixer and document the stability contract separately from quality expectations. |
| `hash_string` | The routine is FNV-like but forces the intermediate state non-negative after each byte. | Repeated sign normalization discards state information and increases collision pressure relative to the underlying mix. | Create a focused issue to define a stable non-cryptographic string hash contract with final-state normalization only, plus differential tests against a reference implementation. |
| `combine` / `hash_combine` | Combination is `h1 * 31 + h2` without avalanche behavior or domain separation. | Composite keys built from weak primitives compound existing clustering and make future generic hashing harder to trust. | Create a focused issue to specify a compositional hash combiner suitable for tuples / structs and add collision-distribution smoke tests. |

**Assessment**: quality is adequate only for temporary monomorphic helpers. It is not strong enough to serve as the long-term hash contract for broader stdlib collection work.

### `std::collections::hash`

| Surface | Finding | Why it matters | Follow-up-ready action |
|---------|---------|----------------|------------------------|
| `hash_i32` | Collection-local hashing duplicates the same weak pattern as the core helper instead of delegating to one canonical policy. | Two hash policies can drift independently and make future behavior changes harder to reason about. | Create a focused issue to unify hash policy ownership so collections consume a single audited mixer from `std::core::hash`. |
| `hashmap_set` | The map has fixed capacity and no resize path. When probing exhausts the table, insertion silently stops without returning an error or status. | This is a direct correctness risk: callers can lose writes while the API shape suggests unconditional insertion/update. | Create a focused issue to add explicit load-factor management and a non-silent insertion contract, either by resizing or by returning a failure signal. |
| `hashmap_remove` | Removal collects keys and values separately, clears the table, then re-inserts everything except the removed key. | This turns single-key removal into whole-table work and couples correctness to key/value iteration order remaining perfectly aligned. | Create a focused issue to replace rebuild-based deletion with tombstones or backward-shift deletion, plus occupancy regression tests. |
| `hashmap_keys` / `hashmap_values` | The implementation exposes only materialized snapshots, which `remove` then depends on for internal reconstruction. | API and internals are conflated; performance degrades and invariants are harder to protect. | Create a focused issue to separate internal iteration primitives from public snapshot helpers. |
| HashSet facade | `HashSet` inherits all HashMap weaknesses and additionally encodes membership as a magic `1` value. | Sentinel-backed reuse makes the surface harder to generalize and obscures the true set invariant. | Create a focused issue for a dedicated set storage invariant and façade wording aligned with the eventual generic collection plan. |

**Assessment**: this family carries both collection and hashing risk. It is the highest-priority correctness and performance hotspot in the audited set.

### `std::json`

| Surface | Finding | Why it matters | Follow-up-ready action |
|---------|---------|----------------|------------------------|
| `parse` | The parser returns the first recognizable JSON value after leading whitespace, but it does not require the remainder of the input to be exhausted. | Inputs with trailing garbage can be accepted as valid top-level parses, which is a correctness and contract problem. | Create a focused issue to define full-document parse semantics and add rejecting fixtures for trailing non-whitespace content. |
| Array/object representation | Arrays and objects store raw JSON text and reparse on access via `json_get` / `json_get_index`. | Nested access becomes repeatedly linear and alloc-heavy; performance degrades with depth and repeated field lookups. | Create a focused issue for an explicit parsed-node representation or cached index table, with a STOP_IF note if recursive type support is still missing. |
| `json_get` | Field lookup scans for a quoted key substring, then reparses from the following colon position. | This is structurally fragile: substring matching is not a real object-member parser and depends on reparsing to recover validity. | Create a focused issue to implement object-member scanning that respects commas and nesting rather than string-substring heuristics. |
| Number handling | `json_parse_i32` is legacy decimal-only logic, while `parse` accepts JSON numbers into raw text and later extraction relies on `parse_i32` / `parse_f64`. | The surface mixes legacy and newer behavior, leaving number acceptance and error cases under-specified. | Create a focused issue to publish one numeric contract for `std::json`, including integer range behavior and float syntax acceptance. |
| `stringify_pretty` | Pretty-printing is documented as deferred and currently behaves as pass-through. | The name implies formatting semantics that the implementation does not provide. | Create a focused issue to either implement indentation semantics or rename/relabel the API as a temporary alias. |

**Assessment**: the parser family is correctness-sensitive and currently too permissive at the top level. The current raw-text representation is also a likely performance footgun once stdlib users do repeated nested access.

### `std::toml`

| Surface | Finding | Why it matters | Follow-up-ready action |
|---------|---------|----------------|------------------------|
| `toml_parse` | The parser validates almost nothing and always returns `Ok(TomlValue { tag: 5, raw: s })`, including for structurally invalid documents. | Callers cannot distinguish valid TOML from malformed input, which is a direct correctness failure for a parser API. | Create a focused issue to introduce real parse failure cases and a documented supported subset instead of unconditional success. |
| Supported grammar | Comments, blank lines, and shallow `key = value` extraction work, but table headers and arrays-of-tables are explicitly skipped. | The module name suggests TOML parsing, while the actual behavior is a partial line-oriented accessor. | Create a focused issue to rename the current subset in docs and define the next grammar increments explicitly. |
| Type detection | `toml_parse_value` classifies by quote, bool literal, `[`, or presence of `.`; all other bare values are treated as integers. | Datetime, inline tables, special numeric forms, and invalid literals are all misclassified or silently accepted. | Create a focused issue to harden scalar classification and add negative fixtures for unsupported TOML forms. |
| Table access | `toml_get` performs independent line scanning over raw source each call. | Repeated lookup scales poorly and ignores TOML constructs that span context or sections. | Create a focused issue for a parsed table representation or indexed key map for the supported subset. |

**Assessment**: `std::toml` is currently closer to a permissive configuration-line helper than a parser. This is a contract-ambiguity hotspot with direct correctness impact.

### `std::fs`

| Surface | Finding | Why it matters | Follow-up-ready action |
|---------|---------|----------------|------------------------|
| `read_string` / `write_string` | These are thin intrinsic facades with a plausible `Result` contract and no obvious extra policy gap in this file. | Risk is mostly in host/runtime coverage rather than local source logic. | No direct redesign issue needed from this slice; keep covered by existing host capability rollout work unless semantics diverge in runtime evidence. |
| `exists` | The function is explicitly implemented as `is_ok(__intrinsic_fs_read_file(path))`, and the doc comment calls it a stub. | A function named `exists` normally implies path-existence semantics, but this implementation is really `is readable regular file`. | Create a focused issue to split existence, readability, and file-type checks into distinct APIs or rename `exists` to a probe-style helper until full host support lands. |
| Module contract | The module header already documents a later WASI P2 filesystem plan, but the current `std::fs` namespace still reads as a general filesystem facade. | Users can mistake a temporary bridge for a durable host contract. | Create a focused issue to align module-level docs and naming with the `std::host::*` rollout plan so the limitation is visible at import time, not just in doc comments. |

**Assessment**: the host facade is less algorithmically risky than the hash and parser families, but the naming-to-behavior gap is large enough to justify follow-up before the surface expands.

## Follow-up candidates

1. Hash contract hardening: replace the current stable-but-weak mixers in `std::core::hash` with an audited non-cryptographic policy and add collision-distribution tests.
2. Hash collection safety: add resize or explicit insertion failure semantics to `std::collections::hash`, then replace rebuild-based deletion with an invariant-preserving deletion strategy.
3. JSON parser contract: require full-input consumption, define numeric acceptance precisely, and separate temporary raw-text storage from the recommended access path.
4. TOML subset clarification: either implement real parse validation for the documented subset or narrow the API/docs so the module no longer presents itself as a general TOML parser.
5. Host facade naming pass: split `std::fs::exists` into capability-accurate probes or rename it until full filesystem metadata support exists.
6. Family-wide quality tests: add property, differential, and adversarial fixtures that target collisions, parser rejection, and collection occupancy edge cases.

## Recommended sequencing

1. P0: `std::collections::hash` correctness and insertion contract.
2. P0: `std::toml` unconditional-success parser contract.
3. P0: `std::json` full-document parse contract and nested-access strategy.
4. P1: `std::core::hash` mixer/combiner redesign with test backing.
5. P1: `std::fs` contract/naming cleanup aligned with host facade rollout.
