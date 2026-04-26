# Issue 511: Numeric-tag modernization inventory

## Scope

This artifact inventories the current public numeric-tag sum-type surfaces in
`std::json`, `std::toml`, `std::wit`, and the adjacent raw-value helpers in
`std::bytes` that set the boundary for what should and should not migrate under
issue #511.

The goal is not to implement the migrations here. The goal is to make the next
implementation slice follow-up-ready by separating:

- public tagged-sum surfaces that should move to `enum + match`
- explicit interop boundaries that may continue to use raw integers
- adjacent raw byte / wire helpers that are not enum-tag surfaces

## Policy rule

New stdlib APIs must not expose raw numeric tags by default.

If a module needs numeric discriminants for ABI, wire format, forward
compatibility, or introspection, that raw representation must sit behind an
explicit conversion helper or compatibility facade rather than being the primary
public value model.

## Inventory summary

| Family | Current public surface | Numeric-tag shape today | Classification | Why |
|---|---|---|---|---|
| `std::json` | `JsonValue { tag: i32, raw: String }` plus `is_*` / `json_as_*` helpers | Public tagged struct with six raw numeric kinds (`0..5`) | Migrate to enum | This is a user-facing sum type, not an ABI boundary. The numeric tag leaks representation instead of intent. |
| `std::toml` | `TomlValue { tag: i32, raw: String }` plus `toml_as_*` / table helpers | Public tagged struct with six raw numeric kinds (`0..5`) | Migrate to enum | Same problem as JSON: the public model is a tagged struct encoding a logical variant type. |
| `std::wit` | `WitType` enum plus `wit_type_id` / `wit_type_from_id` | Typed enum is primary; raw `i32` ids remain as explicit conversions | Keep raw for interop | WIT ids are protocol-facing identifiers. Raw integers are justified only as an explicit boundary, not as the default API. |
| `std::bytes` | Endian, LEB128, byte, and wire helpers over `i32` / `i64` | Raw numeric values, but not variant tags | Keep raw | These APIs encode byte-level data, not sum-type discriminants. They are adjacent evidence, not numeric-tag modernization targets. |

## Detailed inventory

### `std::json`

Current public representation:

| Surface | Current shape | Public risk |
|---|---|---|
| `JsonValue` | `pub struct JsonValue { tag: i32, raw: String }` | Callers can couple to `0..5` instead of variant meaning. |
| `parse` | Returns `Result<JsonValue, String>` where variant identity is carried by `tag` | The parsed value model remains representation-first. |
| `stringify` / `stringify_pretty` | Operate on `JsonValue` | Serialization keeps the tagged-struct model alive across the whole surface. |
| `is_null` / `is_bool` / `is_number` / `is_string` / `is_array` / `is_object` | Compare `v.tag` against fixed literals | Public helpers encode the tag table into API behavior. |
| `json_as_bool` / `json_as_string` / `json_as_i32` / `json_as_f64` | Guard on `v.tag` before decoding `raw` | Extraction is variant-driven, but the variant is still a raw integer. |
| `json_get` / `json_get_index` | Return `JsonValue` values built from tag literals | Nested access propagates the numeric-tag model. |

Current tag mapping in public docs and code:

| Tag | Meaning |
|---|---|
| `0` | `null` |
| `1` | `bool` |
| `2` | `number` |
| `3` | `string` |
| `4` | `array` |
| `5` | `object` |

Migration note:

Move `JsonValue` to an enum-first public model. The follow-up implementation
should preserve the current non-recursive storage strategy where useful, but the
raw text carrier must become internal variant payload, not a public `tag: i32`
contract. Compatibility wrappers such as `is_*` and `json_as_*` can remain, but
they should dispatch via `match` over variants.

Recommended migration shape:

| Stage | Change |
|---|---|
| 1 | Introduce enum-backed `JsonValue` as the public type. |
| 2 | Keep raw-text payloads for array/object if recursive representation is still intentionally deferred. |
| 3 | Re-express `is_*`, `json_as_*`, `json_get`, and `json_get_index` in terms of variant matches. |
| 4 | Remove or hide direct raw-tag access from the public surface. |

### `std::toml`

Current public representation:

| Surface | Current shape | Public risk |
|---|---|---|
| `TomlValue` | `pub struct TomlValue { tag: i32, raw: String }` | The logical value kind is represented as `0..5` in the public model. |
| `toml_parse` | Returns `Result<TomlValue, String>` with tables encoded as tag `5` | Parsing publishes storage details rather than a typed variant API. |
| `toml_stringify` | Special-cases `tag == 0`, otherwise passes `raw` through | Serialization behavior depends on numeric tags instead of variants. |
| `toml_as_string` / `toml_as_int` / `toml_as_bool` | Guard on `v.tag` | User-facing accessors preserve the tagged-struct contract. |
| `toml_get` / `toml_table_keys` | Require `tag == 5` for tables | Table semantics are encoded by integer convention rather than type shape. |

Current tag mapping in public docs and code:

| Tag | Meaning |
|---|---|
| `0` | string |
| `1` | integer |
| `2` | float |
| `3` | boolean |
| `4` | array |
| `5` | table |

Migration note:

`TomlValue` should follow the same enum-first modernization as `JsonValue`.
The module is not serving an ABI contract, so the tag table should not remain a
public API surface. As with JSON, non-recursive raw storage for arrays or tables
can remain an implementation detail if needed for current compiler/runtime
constraints.

Recommended migration shape:

| Stage | Change |
|---|---|
| 1 | Introduce enum-backed `TomlValue` as the public value model. |
| 2 | Keep string/int/float/bool extraction helpers as convenience wrappers over `match`. |
| 3 | Preserve current table parsing behavior while moving table identity from tag `5` to an explicit variant. |
| 4 | Hide raw tag numbers from docs, constructors, and downstream callers. |

### `std::wit`

Current public representation:

| Surface | Current shape | Classification |
|---|---|---|
| `WitType` | Public enum with named variants | Preferred modern shape |
| `wit_type_bool()` ... `wit_type_string()` | Typed constructors returning `WitType` | Preferred modern shape |
| `wit_type_id(ty)` | Explicit enum-to-`i32` conversion | Keep raw for interop |
| `wit_type_from_id(id)` | Explicit `i32`-to-enum conversion | Keep raw for interop |
| `Unknown(i32)` | Forward-compatible raw-id escape hatch inside the enum | Keep raw for forward compatibility |

Migration note:

`std::wit` is already the target shape for issue #511: enum-first public API,
with raw numeric ids isolated to explicit conversion points. The remaining work
here is policy, not structural migration.

Required policy for future `std::wit` additions:

- New helpers should accept and return `WitType` by default.
- Raw ids should only appear in explicitly named interop functions.
- Forward-compatible unknown ids should remain representable through
  `Unknown(i32)` rather than forcing callers back to a bare `i32` surface.

### `std::bytes`

Current public representation:

| Surface family | Current shape | Classification |
|---|---|---|
| byte buffers and cursors | `Vec<i32>` / `i64` wire-oriented helpers | Keep raw |
| endian helpers | `u16_*`, `u32_*`, `read_u*`, `buf_push_u*` style numeric conversion helpers | Keep raw |
| hex / base64 / LEB128 helpers | Integer and byte conversion routines | Keep raw |

Migration note:

`std::bytes` is adjacent evidence because it uses raw numbers heavily, but those
numbers are data bytes and wire values, not discriminant tags for public sum
types. Issue #511 should not widen into rewriting these helpers into enums.

This family sets the exemption rule: raw numeric values are acceptable when the
API is explicitly about wire-format data. They are not acceptable as the default
representation of a user-facing variant type.

## Migration policy

Use this decision rule for follow-up slices:

| Question | If yes | If no |
|---|---|---|
| Is the raw integer representing a logical variant kind in a public value model? | Migrate to `enum + match` | Continue evaluation |
| Is the raw integer required by an ABI, protocol, wire format, or forward-compatible unknown-id path? | Keep raw only behind explicit conversion helpers | Continue evaluation |
| Is the raw integer just internal implementation state, not public API? | Make it private or internal | Continue evaluation |
| Is the API in a byte/wire helper family rather than a sum-type family? | Keep raw | Do not force enum conversion |

## Follow-up-ready recommendations

1. Treat `std::json` as the first migration target because its public tagged
   struct is the clearest user-facing raw-tag surface.
2. Apply the same representation strategy to `std::toml` immediately after JSON
   so the parsing families converge on one policy.
3. Use `std::wit` as the reference pattern: enum-first public API, explicit raw
   conversion helpers only where interop requires them.
4. Do not open a `std::bytes` modernization task from this issue unless the
   problem is specifically about wire-level correctness or naming, not enum-tag
   cleanup.
