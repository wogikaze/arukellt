# Issue 514: parser and host follow-up audit

## Scope

This note is a companion to [`514-quality-audit-matrix.md`](514-quality-audit-matrix.md). It is intentionally limited to the family surfaces that still need a follow-up-ready audit trail for the parser and host sides of issue 514:

- `std::json`
- `std::toml`
- `std::fs`

The goal is not to change implementations here. The goal is to record current behavior, rank the risks, and leave a short list of implementation slices that can be consumed without re-reading the same source files.

## Risk scale

| Rank | Meaning |
|------|---------|
| Critical | Likely user-visible misbehavior or contract break under ordinary inputs. |
| High | Real production risk or severe quality gap; follow-up should be scheduled soon. |
| Medium | Noticeable limitation or footgun, but not a frequent hard failure. |
| Low | Acceptable for now; document and revisit later. |

## Family matrix

| Family | Correctness | Robustness / collision | Performance | Contract ambiguity | Priority |
|--------|-------------|------------------------|-------------|--------------------|----------|
| `std::json` | Critical | Medium | High | High | P0 |
| `std::toml` | Critical | Medium | Medium | Critical | P0 |
| `std::fs` | Medium | Low | Low | Critical | P1 |

## `std::json`

### Evidence anchors

- `std/json/mod.ark:5-8` stores arrays and objects as raw text and reparses nested access on demand.
- `std/json/mod.ark:187-230` parses the first recognizable JSON value after leading whitespace, then returns without any explicit full-input exhaustion check.
- `std/json/mod.ark:238-240` documents `stringify_pretty` as pass-through behavior.
- `std/json/mod.ark:254-320` shows the accessors derive values by re-reading the raw literal rather than traversing a parsed tree.

### Four-axis audit

| Surface | Correctness | Robustness / collision | Performance | Contract ambiguity | Follow-up-ready action |
|---------|-------------|------------------------|-------------|--------------------|------------------------|
| `parse` | The parser accepts the first recognizable JSON value after leading whitespace, but it does not require the rest of the input to be consumed. | No collision surface, but malformed or trailing-garbage input can slip through as a partially parsed top-level value. | The parser itself is linear, but repeated parse-and-extract flows re-do work later. | The name `parse` suggests a whole-document contract, not a prefix parse. | Define whole-document parse semantics and add rejection cases for trailing non-whitespace content. |
| Array/object representation | Arrays and objects carry raw JSON text instead of a parsed node structure. | Deeply nested or adversarial documents are still handled by string scanning, not structural validation. | Every nested lookup can re-scan and re-parse the same source text. | The raw-text storage is an implementation workaround, not a stable API promise. | Introduce a parsed-node or cached-index representation, or explicitly label the current model as a temporary bridge. |
| `json_get` / `json_get_index` | Field and element access depend on re-parsing raw literals. | Substring heuristics are fragile around nested content and escaped delimiters. | Lookups are effectively linear in the size of the stored literal, not just the requested key or index. | The accessor names imply normal object/array navigation, but the contract is currently heuristic-driven. | Replace substring-driven lookup with structural scanning, then back it with rejecting fixtures for malformed nesting and escaped content. |
| Number helpers | `json_parse_i32` is legacy decimal-only logic, while `parse` accepts JSON number text into raw storage. | Unsupported number forms and integer edge cases are easy to accept inconsistently across the legacy and newer paths. | Number handling is cheap per call, but the split contract invites duplicate parsing in callers. | The module currently exposes two number stories: legacy primitive helpers and structured JSON values. | Publish one numeric contract for `std::json`, including integer range behavior and float syntax acceptance. |
| `stringify_pretty` | The API name implies formatting, but the implementation currently passes through the raw text. | No collision issue. | No extra formatting work is done, so the surface does not pay for pretty output yet. | The signature promises indentation semantics that are not delivered. | Either implement indentation or relabel the surface so callers do not depend on a formatting guarantee that does not exist. |

**Assessment**: `std::json` is the highest-risk parser family in this slice. The main issues are not syntax sugar; they are top-level parse semantics, repeated reparsing, and a contract that still reads more like a temporary bridge than a stable JSON surface.

## `std::toml`

### Evidence anchors

- `std/toml/mod.ark:5-6` states that arrays and tables store raw serialized text.
- `std/toml/mod.ark:88-115` documents a parser that only checks for at least one line containing `=` and then returns `Ok` for the original source.
- `std/toml/mod.ark:59-84` shows type detection is based on quote / boolean / `[` / `.` heuristics.
- `std/toml/mod.ark:149-196` performs line-by-line rescans for both lookup and key enumeration.

### Four-axis audit

| Surface | Correctness | Robustness / collision | Performance | Contract ambiguity | Follow-up-ready action |
|---------|-------------|------------------------|-------------|--------------------|------------------------|
| `toml_parse` | The parser does not reject malformed TOML documents; it only checks for a non-comment line containing `=` and then returns `Ok`. | No collision surface, but unsupported TOML forms are silently treated as if they were acceptable input. | The validation pass is cheap, but it does not buy real parse validation. | The function name says “parse a TOML document”; the implementation is closer to a permissive line filter. | Introduce real parse failure cases for the supported subset and document what is intentionally unsupported. |
| `toml_parse_value` | Bare values that are not quoted, boolean, or array-like are classified as integers unless a dot is present. | Invalid literals, datetimes, and richer TOML forms can be misclassified rather than rejected. | The classifier is O(1), but it postpones real validation to downstream consumers. | The type-tag model suggests a typed TOML value, while the implementation only approximates TOML syntax. | Tighten scalar classification and add negative fixtures for unsupported forms. |
| `toml_get` / `toml_table_keys` | Lookup is performed by rescanning the original document line by line on each call. | No collision issue; the bigger risk is robustness against sectioned or multi-line TOML that this subset cannot represent. | Repeated lookups scale with source size instead of table size. | The API reads like a table accessor, but the implementation is really a raw-text key scanner. | Move to a parsed table representation or explicit index for the supported subset. |
| Module contract | The module still advertises parser/serializer behavior while supporting only shallow `key = value` lines, with tables and arrays-of-tables skipped. | Supported input is narrow enough that users can believe they are feeding valid TOML when they are not. | The subset keeps code simple, but it also makes every unsupported form a latent bug source. | The docs and name overstate the current grammar. | Rename or narrow the docs until the supported subset is explicit at import time. |

**Assessment**: `std::toml` is still closer to a permissive configuration-line helper than a TOML parser. The main follow-up is to make rejection behavior explicit before the surface grows further.

## `std::fs`

### Evidence anchors

- `std/fs/mod.ark:6-9` calls out `exists` as a best-effort stub and ties the current implementation to a later WASI P2 plan.
- `std/fs/mod.ark:11-24` shows `read_string` and `write_string` are thin intrinsic facades.
- `std/fs/mod.ark:27-32` implements `exists` as a read probe via `is_ok(__intrinsic_fs_read_file(path))`.

### Four-axis audit

| Surface | Correctness | Robustness / collision | Performance | Contract ambiguity | Follow-up-ready action |
|---------|-------------|------------------------|-------------|--------------------|------------------------|
| `read_string` / `write_string` | The intrinsic-backed read/write calls are straightforward, and the local contract matches the return type shape. | No collision surface. Robustness depends on the host/runtime path, not on local algorithmic behavior. | These are thin wrappers, so there is little local overhead beyond the intrinsic call itself. | The functions still sit in a module whose naming reads like a general filesystem facade rather than a host-bound bridge. | Keep covered by host-capability rollout work; no local redesign issue is required from this slice. |
| `exists` | The stub checks readability, not true path existence, so directories and write-only files can report `false`. | No collision issue, but the probe is intentionally lossy and can produce false negatives. | The implementation is cheap because it piggybacks on a read attempt. | The function name implies existence semantics that the current implementation does not provide. | Split existence, readability, and file-type checks into distinct APIs or rename the probe helper until full host support lands. |
| Module contract | The header itself says this is a stub and points at later WASI P2 filesystem intrinsics. | No collision issue. | No notable local cost. | The docs leave a temporary-bridge smell at module scope, even though the names are stable-looking. | Align the docs and naming with the `std::host::*` rollout so the limitation is visible at import time. |

**Assessment**: `std::fs` is less algorithmically risky than the parser families, but `exists` is still a contract trap. Its current behavior is acceptable only if callers know it is a readability probe.

## Prioritized follow-up list

1. **P0 - `std::toml` parse contract**: make malformed TOML reject deterministically, and document the supported subset instead of returning `Ok` for almost any non-empty document.
2. **P0 - `std::json` top-level parse contract**: require full-input consumption, then add fixtures for trailing garbage and unsupported numeric forms.
3. **P1 - `std::json` nested access strategy**: replace raw-text substring lookup with structural scanning or cached parsed nodes so repeated access is not re-parsing the same content.
4. **P1 - `std::fs::exists` naming/semantics**: split or rename the current readability probe so it does not masquerade as general path existence.
5. **P2 - `std::toml` / `std::json` documentation alignment**: make the docs state the supported subset and temporary bridge behavior explicitly until the implementation catches up.

## Follow-up readiness

This note is meant to be consumed directly by an implementation slice owner. It records the current source behavior, the highest-risk follow-ups, and the contract gaps that should be resolved before the next expansion step.
