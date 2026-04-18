# Stdlib prelude call-site inventory (facade migration)

Working note for [#513](../../issues/open/513-stdlib-prelude-safety-and-wrapper-surface.md):
which **stdlib implementation files** still rely on **prelude-auto-imported**
helpers that have a **module-local or `std::*` facade** equivalent. This is a
shelf list for future refactors, not a commitment to change every row immediately.

**Scope:** implementation modules under `std/`. The tiny prelude (`panic`,
`assert`, `String_from` / `String_new`, core `len` / `push` / `pop` on `Vec`,
etc.) may remain even after facade work; rows call out *replaceable* patterns
where an explicit `std::text`, `std::core::convert`, or `std::collections`
path already exists.

**Caveat:** `std::core::convert::parse_*` returns `Option<_>` while prelude
`parse_*` returns `Result<_, String>`. Call sites need a small adapter when
moving.

| File | Approximate pattern | Suggested wrapper target |
|------|---------------------|--------------------------|
| `std/json/mod.ark` | `concat`, `slice`, `eq`, `char_at`, `len`; `i32_to_string`; `parse_i32` (`Result`) | `use std::text` for `concat` / `slice_bytes` / `contains` where applicable; `text::format_i32` (or `core::convert::i32_to_string`); `core::convert::parse_i32` + `Result`/`Option` shim |
| `std/path/mod.ark` | `concat`, `slice`, `eq`, `char_at`, `len` (mixes `text::is_empty` with prelude); `normalize` / `components` use `__intrinsic_split` / `__intrinsic_join` and `Vec` cursor idioms | `text::concat`, `text::split`, `text::join` instead of raw intrinsics; keep or centralize `last_index_of` in-module; `collections::vec` patterns for `Vec<String>` scratch buffers |
| `std/test/mod.ark` | Message building via `concat` chains; `i32_to_string`, `i64_to_string`, `f64_to_string`, `bool_to_string`; `eq`, `contains`; `String_new` | `text::concat`, `text::format_*` (or `core::convert::*`); `text::contains`; keep `panic` / `String_new` as tiny-prelude if desired |
| `std/toml/mod.ark` | Same shape as JSON: `split`, `trim`, `concat`, `slice`, `char_at`, `contains`, `parse_i32` | Same family as `std/json/mod.ark`: `std::text` + `std::core::convert` |
| `std/csv/mod.ark` | `split`, `concat`, `slice`, `char_at` on line fields | `text::split`, `text::concat`, byte-safe slicing via `text::slice_bytes` where appropriate |
| `std/io/mod.ark` | Heavy `len` / `push` / `pop` / `get_unchecked` / `set` on `Vec<i32>` used as cursors and byte buffers | Prefer **module-local** private helpers (already the right “facade” boundary) so I/O layout stays in `std::io`; optional `std::bytes` alignment for byte vectors |

`std/prelude.ark` is the **definition** of the prelude, not a consumer; it is
intentionally excluded from the table above.

## See also

- [prelude.md](prelude.md) — tiny vs legacy prelude design
- [prelude-migration.md](prelude-migration.md) — historical v3 mapping
- [migration-guidance.md](migration-guidance.md) — deprecated API replacements
