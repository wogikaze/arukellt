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

---

## Wrapper-preferred family list

The following stdlib families are the **canonical replacement targets** for
prelude helpers. When writing docs examples, cookbook recipes, or stdlib
internal code, prefer the family path over the bare prelude symbol.

### std::text

Covers string manipulation that the prelude exposes as top-level helpers.

| Prelude helper         | Preferred wrapper              | Notes |
|------------------------|--------------------------------|-------|
| `concat(a, b)`         | `text::concat(a, b)`           | identical semantics |
| `slice(s, lo, hi)`     | `text::slice_bytes(s, lo, hi)` | byte-safe; prelude version is bytes already |
| `char_at(s, i)`        | `text::char_at(s, i)`          | same; module-qualified preferred in examples |
| `contains(s, sub)`     | `text::contains(s, sub)`       | same |
| `split(s, sep)`        | `text::split(s, sep)`          | same |
| `join(parts, sep)`     | `text::join(parts, sep)`       | same |
| `trim(s)`              | `text::trim(s)`                | same |
| `eq(a, b)` (strings)   | `text::eq(a, b)` or `==`       | use `==` where the compiler supports it |

### std::core::convert

Covers numeric / boolean / string conversion helpers.

| Prelude helper         | Preferred wrapper                       | Notes |
|------------------------|-----------------------------------------|-------|
| `i32_to_string(n)`     | `core::convert::i32_to_string(n)`       | |
| `i64_to_string(n)`     | `core::convert::i64_to_string(n)`       | |
| `f64_to_string(n)`     | `core::convert::f64_to_string(n)`       | |
| `bool_to_string(b)`    | `core::convert::bool_to_string(b)`      | |
| `parse_i32(s)`         | `core::convert::parse_i32(s)`           | returns `Option<i32>`; prelude returns `Result<i32,String>` — add adapter if needed |
| `parse_f64(s)`         | `core::convert::parse_f64(s)`           | same Option vs Result caveat |

### std::io

The prelude exposes I/O helpers (`print`, `println`, `read_line`, etc.) that
are already thin wrappers; prefer the explicit module path in doc examples.

| Prelude helper   | Preferred wrapper    | Notes |
|------------------|----------------------|-------|
| `print(s)`       | `io::print(s)`       | |
| `println(s)`     | `io::println(s)`     | |
| `read_line()`    | `io::read_line()`    | |
| `eprintln(s)`    | `io::eprintln(s)`    | |

### std::collections (Vec helpers)

Low-level `Vec` manipulation is legitimately in the tiny prelude for now.
New examples should prefer the typed helpers from `std::collections` when
operating on `Vec<T>` above the byte-buffer level.

| Prelude helper             | Preferred wrapper                     | Notes |
|----------------------------|---------------------------------------|-------|
| `push(v, x)` on `Vec<T>`  | `collections::vec::push(v, x)`        | tiny-prelude `push`/`pop`/`len` may remain for internal impl |
| `get_unchecked(v, i)`      | index syntax or `collections::vec::get(v, i)` | prefer bounds-checked form in examples |

### Families where the prelude is still acceptable

- `panic(msg)` — no module-qualified version; keep using prelude form.
- `assert(cond, msg)` — same.
- `String_new()` / `String_from(s)` — tiny-prelude allocation; no facade needed yet.
- `len(s)` on `String` — keep using; module-qualified form is verbose for no gain.

---

## Migration plan: phased removal of deprecated prelude helpers from docs examples

### Rationale

Docs and cookbook examples are the highest-leverage place to teach good habits.
Removing prelude direct usage from those files does not break any runtime
behavior but requires updating example snippets and checking cross-references.

### Phase 1 — New examples only (immediate, no existing files edited)

Target: all new cookbook recipes and stdlib reference pages written after
2026-04-22 must use module-qualified wrappers per the family list above.
No backfill of existing files yet.

Checklist:
- [ ] Add a linter note to `docs/stdlib/expansion-policy.md` stating the rule
- [ ] Update `docs/stdlib/prelude.md` "usage guidance" section to reference this inventory

### Phase 2 — Cookbook backfill (next stdlib milestone)

Scope: `docs/cookbook/` — all `.md` recipe files.

Steps:
1. Grep for `concat(`, `slice(`, `i32_to_string(`, `parse_i32(`, `print(`,
   `println(` that are NOT already qualified with a module prefix.
2. Replace each with the wrapper form from the family list.
3. Where `parse_i32` return type changes (Result → Option), update the example
   to use `match` on `Option` or add a `.expect()` / `unwrap_or_else` call.
4. Run `python3 scripts/check/check-docs-consistency.py` after each file batch.
5. Commit as: `docs(cookbook): migrate prelude calls to wrapper families (#513)`.

Estimated files: ~6–10 cookbook recipes based on current `docs/cookbook/` contents.

### Phase 3 — stdlib reference pages backfill (stdlib hardening milestone)

Scope: `docs/stdlib/modules/` and `docs/stdlib/*.md` example blocks.

Steps:
1. Same grep pattern as Phase 2 applied to `docs/stdlib/`.
2. Special care for `io.md` — examples there currently use bare `println`; update
   to show `use std::io; io::println(...)` pattern.
3. Update `docs/stdlib/core.md` convert examples to use `core::convert::*`.
4. Regenerate docs: `python3 scripts/gen/generate-docs.py`.
5. Verify: `python scripts/manager.py verify quick`.
6. Commit as: `docs(stdlib): migrate reference examples to wrapper families (#513)`.

### Phase 4 — Deprecation annotation in prelude.ark (future, out-of-scope for this slice)

Once Phases 1–3 are complete, each replaceable prelude helper can receive a
`@deprecated` annotation pointing at its wrapper family. This phase requires
compiler support and is explicitly out-of-scope for the docs-only slice.

Tracking: open a follow-up issue referencing #513 when Phase 3 is merged.

### Summary timeline

| Phase | Scope                          | Gate                       |
|-------|--------------------------------|----------------------------|
| 1     | New docs only                  | Merged with this PR        |
| 2     | `docs/cookbook/` backfill      | Next stdlib milestone       |
| 3     | `docs/stdlib/` backfill        | stdlib hardening milestone  |
| 4     | `@deprecated` annotations      | Compiler support available  |
