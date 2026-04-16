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

Manifest-driven regression today lives under `tests/fixtures/stdlib_*` and is executed by the integration harness (`crates/arukellt/tests/harness.rs`, `tests/fixtures/manifest.txt`). The rows below propose **property-style or differential** additions on top of those baselines—not new production code in this slice.

| Family | Existing regression anchors (fixtures) | Property / differential idea | Harness entry | Dependency / risk |
|--------|----------------------------------------|------------------------------|---------------|-------------------|
| **JSON** (`std/json`) | `tests/fixtures/stdlib_json/json_roundtrip.ark`, `json_basic.ark`, `json_parse.ark`, `json_stringify.ark`, `json_nested.ark` | **Round-trip**: generate structured values (objects/arrays/scalars), `stringify` → `parse` → compare; **differential**: same inputs vs `serde_json` in a small Rust test crate or corpus-driven golden file | `cargo test -p arukellt --test harness` (add new `run:` lines in `manifest.txt` when fixtures exist); optional `cargo test` in a dedicated `crates/*` proptest helper if in-process gen is easier | Differential needs a defined normalization (key ordering, float formatting). T3 runs add wasm/host variance—keep first slice on T1 `run:` unless JSON is host-only |
| **TOML** (`std/toml`) | `tests/fixtures/stdlib_toml/toml_basic.ark`, `toml_extended.ark` | **Round-trip**: document-level encode/decode on generated tables; **differential**: parse/emit parity vs Rust `toml` crate on a shared corpus (valid + edge cases from `toml` test vectors) | Same harness + `manifest.txt`; corpus can live under `tests/fixtures/stdlib_toml/corpus/` | Spec vs implementation quirks (datetime, inline tables, `\u` escapes). Lossy emit must be explicitly excluded or compared structurally |
| **CSV** (`std/csv`) | `tests/fixtures/stdlib_csv/csv_parse_row.ark` | **Property**: delimiter/quote escaping invariants (`parse` ∘ `format` on rows without embedded newlines); **differential**: vs Python `csv` or Rust `csv` on RFC4180-shaped inputs | New `run:` fixtures next to `csv_parse_row.ark` | Excel vs RFC semantics; newline-in-field behavior. Start with POSIX newline, no BOM |
| **Path** (`std/path`) | `tests/fixtures/stdlib_path/path_normalize.ark`, `path_join.ark`, `path_stem_ext.ark` | **Property**: `join` associativity / `normalize` idempotence on generated POSIX segments; **differential**: same path ops vs `std::path::Path` (Rust) on a POSIX-only corpus | Harness `run:` fixtures | Windows vs POSIX: scope v1 corpus to `/` semantics or gate with target tags. UNC and `..` collapse need explicit rules |
| **Collections / hash** (`std/collections/hash`, core hash) | `tests/fixtures/stdlib_hashmap/*.ark` (e.g. `hashmap_extended.ark`, `hashset_ops.ark`), `tests/fixtures/stdlib_core/hash.ark` | **Property**: `insert` commutativity for disjoint keys; `remove`/`insert` balance; **differential**: replay random op sequences against `std::collections::HashMap`/`HashSet` keyed by sorted debug snapshot (iteration order not compared) | Harness `run:` or small Rust proptest that drives Ark via compile-once if needed | Hash randomization: seed both sides. Collision stress is valuable but may need dedicated timing-stable build for CI |

**Text** (`tests/fixtures/stdlib_text/string_api.ark`, `string_chars.ark`, `builder_basic.ark`): treat as the next row—**property** ideas include `Builder` concat associativity and grapheme-safe indexing invariants vs explicit UTF-8 edge vectors; same harness path as above.

**WIT / driver round-trips** (related compiler surface): `tests/fixtures/stdlib_wit/wit_basic.ark`, `wit_types.ark`; `crates/ark-driver/tests/wit_import_roundtrip.rs` for import wiring—useful if serialization parity tests need component-model shapes, but not a substitute for stdlib JSON/TOML coverage.

## Acceptance

- [ ] family ごとの test strategy matrix (regression / property / differential / fuzz-ish) が作成される
- [ ] `json`, `toml`, `csv`, `path`, `collections/hash`, `text` の優先ケースが列挙される
- [ ] どの family は Rust or external reference と比較できるかが整理される
- [ ] follow-up fixture / harness issue が必要なら派生する

## Primary paths

- `tests/fixtures/stdlib_*`
- `tests/harness.rs`
- `std/json/mod.ark`
- `std/toml/mod.ark`
- `std/collections/hash.ark`

## References

- `issues/done/389-stdlib-serialization-roundtrip-baselines.md`
- `issues/done/388-stdlib-collections-seq-parity-tests.md`
