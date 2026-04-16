# Stdlib: property / differential / parity test を family 横断で拡張する

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-16
**ID**: 519
**Depends on**: none
**Track**: stdlib
**Blocks v1 exit**: no
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

stdlib の現在の fixture 群は regression には有効だが、境界条件・代数法則・実装差分検出には弱い。
JSON/TOML/CSV/collections/text/path/hash などに property test / round-trip / differential test の観点を導入し、
「なんとなく動く」実装を継続的に押し上げる。

## Repo evidence

- round-trip baseline は family 限定で、hash / bytes / path / io の法則検証は薄い
- `std/core/hash.ark` や `std/collections/hash.ark` は collision / parity の継続検証が弱い
- parser families は malformed input / normalization / lossy cases の coverage が uneven

## Concrete test matrix (planning slice)

### Harness anchors (how these rows run today)

- **Primary entry**: `cargo test -p arukellt --test harness` — integration binary defined in `crates/arukellt/tests/harness.rs` (manifest-driven `run:`, `module-run:`, `t3-run:`, …; see module docs at the top of that file).
- **Fixture list**: `tests/fixtures/manifest.txt` — each new `.ark` program needs a `run:` (or `t3-run:` when wasm parity is in scope) line plus a checked-in `.expected` stdout golden where applicable.
- **Differential / property tests in Rust**: the workspace root `Cargo.toml` files searched for this slice do not yet declare `proptest`, `quickcheck`, or similar; adding in-process generators implies **new dev-dependencies** and a small `#[test]` module (often under `crates/arukellt/tests/` or a dedicated thin crate), separate from the manifest harness unless folded behind a custom `cargo test` target.

Default stdlib parity work should extend **`run:`** fixtures first. **Wasm / T3 parity** uses matching `t3-run:` / `t3-compile:` lines when the same program must behave on `wasm32-wasi-p2` (today several **hashmap** fixtures are registered for T3; **JSON** has `run:` only—see `manifest.txt`).

The rows below propose **property-style or differential** additions on top of those baselines—not new production code in this slice.

| Family | Existing regression anchors (fixtures) | Property / differential idea | Harness entry | Dependency / risk |
|--------|----------------------------------------|------------------------------|---------------|-------------------|
| **JSON** (`std/json`) | `tests/fixtures/stdlib_json/json_roundtrip.ark`, `json_basic.ark`, `json_parse.ark`, `json_stringify.ark`, `json_nested.ark`, `json_escape.ark`, `json_pretty.ark` (all `run:` in `manifest.txt`) | **Round-trip**: generate structured values (objects/arrays/scalars), `stringify` → `parse` → compare; **differential**: same inputs vs `serde_json` in a small Rust `#[test]` module (dev-dependency) or corpus-driven golden files | `cargo test -p arukellt --test harness`; new `run:` lines in `manifest.txt` (add `t3-run:` only once JSON is validated on T3) | Differential needs a defined normalization (key ordering, float formatting). T3 adds wasm/host variance—keep first slice on T1 `run:` while JSON lacks `t3-run:` entries |
| **TOML** (`std/toml`) | `tests/fixtures/stdlib_toml/toml_basic.ark`, `toml_extended.ark` | **Round-trip**: document-level encode/decode on generated tables; **differential**: parse/emit parity vs Rust `toml` crate on a shared corpus (valid + edge cases from upstream `toml` test vectors) | `cargo test -p arukellt --test harness` + `manifest.txt`; optional corpus dir `tests/fixtures/stdlib_toml/corpus/` | Spec vs implementation quirks (datetime, inline tables, `\u` escapes). Lossy emit must be explicitly excluded or compared structurally |
| **CSV** (`std/csv`) | `tests/fixtures/stdlib_csv/csv_basic.ark`, `csv_parse_row.ark` | **Property**: delimiter/quote escaping invariants (`parse` ∘ `format` on rows without embedded newlines); **differential**: vs Python `csv` or Rust `csv` on RFC4180-shaped inputs | New `run:` fixtures under `tests/fixtures/stdlib_csv/`; register in `manifest.txt` | Excel vs RFC semantics; newline-in-field behavior. Start with POSIX newline, no BOM |
| **Path** (`std/path`) | `tests/fixtures/stdlib_path/path_basic.ark`, `path_edge_cases.ark`, `path_normalize.ark`, `path_join.ark`, `path_stem_ext.ark` | **Property**: `join` associativity / `normalize` idempotence on generated POSIX segments; **differential**: same path ops vs `std::path::Path` (Rust) on a POSIX-only corpus | `cargo test -p arukellt --test harness` (`run:` fixtures) | Windows vs POSIX: scope v1 corpus to `/` semantics or gate with target tags. UNC and `..` collapse need explicit rules |
| **Collections / hash** (`std/collections/hash`, core hash) | `tests/fixtures/stdlib_hashmap/hashmap_basic.ark`, `hashmap_extended.ark`, `hashset_ops.ark`, `hashset_basic.ark`, `hashmap_string_i32.ark`, …; `tests/fixtures/stdlib_core/hash.ark` | **Property**: `insert` commutativity for disjoint keys; `remove`/`insert` balance; **differential**: replay random op sequences against `std::collections::HashMap`/`HashSet` keyed by sorted debug snapshot (iteration order not compared) | `cargo test -p arukellt --test harness`; subset also `t3-run:` / `t3-compile:` in `manifest.txt` for hashmap fixtures | Hash randomization: seed both sides. Collision stress is valuable but may need a timing-stable CI profile |

**Text** (`std/text`): regression anchors include `tests/fixtures/stdlib_text/string_api.ark`, `string_chars.ark`, `builder_basic.ark`, `utf8_byte_semantics.ark`, `string_replace.ark`, plus trim/search/lines/recipes in the same directory (all `run:` in `manifest.txt`). **Property** follow-ups: `Builder` concat associativity; UTF-8 edge vectors vs Rust `str`/`String` behavior on the same bytes. Harness: same `cargo test -p arukellt --test harness`.

**WIT / component tooling** (serialization-adjacent, not stdlib JSON): `tests/fixtures/stdlib_wit/wit_basic.ark`, `wit_types.ark`; `crates/ark-driver/tests/wit_import_roundtrip.rs` exercises import wiring; `crates/ark-wasm/src/component/wit.rs` holds golden `.expected.wit` comparisons under `tests/fixtures/component/`—useful for component-shape parity, not a substitute for stdlib format families above.

## Acceptance

- [x] family ごとの test strategy matrix (regression / property / differential / fuzz-ish) が作成される
- [x] `json`, `toml`, `csv`, `path`, `collections/hash`, `text` の優先ケースが列挙される
- [x] どの family は Rust or external reference と比較できるかが整理される（表中の differential 列）
- [ ] follow-up fixture / harness issue が必要なら派生する（実装フェーズで起票）

## Primary paths

- `tests/fixtures/stdlib_*`
- `crates/arukellt/tests/harness.rs`
- `tests/fixtures/manifest.txt`
- `std/json/mod.ark`
- `std/toml/mod.ark`
- `std/collections/hash.ark`

## References

- `issues/done/389-stdlib-serialization-roundtrip-baselines.md`
- `issues/done/388-stdlib-collections-seq-parity-tests.md`
