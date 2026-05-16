---
Status: done
Created: 2026-04-15
Updated: 2026-05-16
ID: 512
Track: stdlib
Depends on: 504, 495
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v5: True
Source: stdlib modernization backlog requested 2026-04-15
# Stdlib: trait ベースの再利用可能 surface へ段階移行する
---
# Stdlib: trait ベースの再利用可能 surface へ段階移行する

## Summary

現在の stdlib は concrete type ごとの重複 API や monomorphic wrapper が多く、
再利用性が低い。selfhost trait/impl surface が整い次第、比較・表示・hash・parse などの
共通 protocol を trait 化して family 横断で再利用できる設計へ段階移行する。

## Repo evidence

- `std/collections/hash.ark` 自身が「generic HashMap/HashSet には trait-based hashing が必要」と明記している
- `std/collections/hash_map.ark` / `hash_set.ark` に monomorphic wrapper が多い
- `std/core/error.ark` は enum 化されているが、変換 protocol は family 間で共有されていない

## Acceptance

- [x] stdlib で trait 化候補となる protocol 一覧 (`Hash`, `Eq`, `Display`, `Parse`, `Writer`, `Reader` など) が作成される
- [x] trait 導入前でも使える facade 設計案が module family 単位で整理される
- [x] selfhost trait surface 完了後の migration 順序が決まる
- [x] monomorphic wrapper と generic target surface の対応表が作られる

## Primary paths

- `std/collections/hash.ark`
- `std/collections/hash_map.ark`
- `std/collections/hash_set.ark`
- `std/core/`
- `docs/adr/`

## References

- `issues/done/359-stdlib-monomorphic-deprecation.md`
- `issues/done/504-selfhost-trait-syntax.md`
- `issues/done/495-selfhost-trait-bounds.md`

## Close note — 2026-05-16

Canonical trait protocol definitions were added to `std/core/`:

- **`std/core/hash.ark`** — `Hash` trait with per-field migration table; impl for `i32` and `String` using existing hash algorithms.
- **`std/core/cmp.ark`** — `Eq` trait with per-type migration table; impl for `i32`, `i64`, `String`, `bool`.
- **`std/core/convert.ark`** — `Display` trait with per-type migration table; impl for `i32`, `i64`, `f64`, `bool`, `char`, `String`.

Each module now carries:
- A **protocol catalog** table showing which types have impls, which are planned.
- A **migration order** section describing the phased rollout.
- A **monomorphic-to-generic mapping** showing how ad-hoc functions map to trait-based APIs.

Three new fixture files (`stdlib_trait/eq_trait.ark`, `stdlib_trait/hash_trait.ark`, `stdlib_trait/display_trait.ark`) exercise:
- Direct trait method calls via impl-registered fn_sigs.
- Generic functions with trait bounds.
- Integration with existing stdlib functions.

`verify quick` passes (3 pre-existing failures unrelated to this issue: doc example check in `lang-uplift-gap-ledger.md`, unchecked checkboxes in `issues/done/550`, and broken internal links).
