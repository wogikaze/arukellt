# Stdlib Raw / Facade / Adapter Boundary Policy

**Issue**: #516
**Status**: design complete
**Created**: 2026-04-22
**Track**: stdlib

## Overview

The Arukellt stdlib mixes internal-representation helpers with user-facing facades in
the same public surface.  This document captures the agreed three-layer model, naming
policy, and per-family API tiering that will guide the migration.

---

## Three-Layer Model

| Layer | Definition | Typical responsibilities |
|-------|-----------|--------------------------|
| **Raw helper** | Exposes internal representation, layout, or monomorphic intrinsics directly.  The caller is expected to understand invariants (index offsets, tag values, masks, etc.). | `Vec<i32>` reader/writer layout; `HashMap_*` / `hash::*` direct calls; WIT numeric ID pass-through. |
| **Facade** | A stable, semantically meaningful user-facing surface.  Internal representation is hidden; behaviour is contracted via `Result`, doc comments, and semver. | Opaque handles / high-level types, `String`/`Option`-based operations, recommended entry points. |
| **Adapter** | The boundary between the stdlib and the outside world (host, runtime, FFI).  Translates between external contracts (intrinsics, WASI) and stdlib-internal representation. | `__intrinsic_*` calls; stdin/stdout fd-tag semantics; component ABI bridging. |

**Raw vs Adapter**: Raw means "touch stdlib's own internal representation directly".
Adapter means "conform to an external protocol".  Both can appear in the same file
(e.g. I/O builds a `Vec<i32>` writer (Raw) and then emits it via an intrinsic (Adapter)).

---

## Naming Policy

Facade is the **default recommended path**.  Low-level APIs must be distinguishable
by name prefix.

| Prefix / rule | Intent | Example (policy level) |
|---------------|--------|------------------------|
| **`raw_`** | Exposes internal layout or wire format.  Caller must follow layout spec. | `raw_reader_from_bytes` — current `reader_from_bytes` demoted to explicit low-level name. |
| **`unchecked_`** | Omits the bounds checks / invariant validation that the facade would perform.  Placed just inside the safe wrapper.  Kept to a minimum; only added where a performance-sensitive, known-safe context exists. | Aligned with existing `get_unchecked` idiom; only created per family when justified. |
| **`internal_`** | Not part of the stable semver surface.  Marked non-recommended in docs and manifest even if `pub`.  Ideal end-state: `pub(crate)` or module-private. | `internal_reader_read` — used across modules during transition, before full encapsulation. |

**Migration rule**: Existing unprefixed names are either promoted to Facade (stable
contract, docs improved) or demoted to `raw_` / `internal_` per the family tiering
table below.  `unchecked_` names are added only where a profiled hot-path justifies
skipping safety checks.

---

## Per-Family API Tiering

"Proposed tier" is the **target after this issue**, not the current state.

### `std::io` (`std/io/mod.ark`)

| Representative API | Proposed tier | Note | Source |
|--------------------|--------------|------|--------|
| `reader_from_bytes` | Raw → rename `raw_reader_from_bytes` | Constructs `[cursor, b0..]` layout directly | `std/io/mod.ark:27` |
| `reader_read` / `reader_read_exact` | Raw | Destructive cursor-and-slice operation | `std/io/mod.ark:49`, `:65` |
| `stdin` / `stdout` / `stderr` | Adapter (+ Raw handle) | Returns fd-tagged `Vec<i32>`; host bridge | `std/io/mod.ark:125`, `:132`, `:139` |
| `print_bytes` | Adapter | Stringifies bytes then calls intrinsic | `std/io/mod.ark:154` |
| `read_stdin_line` | Facade | Semantic return type; currently placeholder | `std/io/mod.ark:149` |
| `writer_write_str` | Facade (Adapter branch inside) | `String` contract; intrinsic for stdout/stderr | `std/io/mod.ark:205` |
| `buf_reader_new` / `buf_writer_new` | Raw | Exposes `buf_cap` and buffer layout | `std/io/mod.ark:276`, `:319` |
| `copy_bytes` | Facade | Composed operation over raw handles; recommended use-case path | `std/io/mod.ark:392` |

**Migration sketch**: rename `reader_from_bytes` → `raw_reader_from_bytes`; add a
Facade `reader_new(bytes: &[u8]) -> Reader` that wraps it.  Mark `buf_reader_new` /
`buf_writer_new` as `raw_` until a Facade buffered reader type lands.

### `std::collections` — `hash_map` / `hash_set`

#### `hash_map` (`std/collections/hash_map.ark`)

| Representative API | Proposed tier | Note | Source |
|--------------------|--------------|------|--------|
| `hashmap_str_i32_new` and other `hashmap_*_*` | Facade | Semantic `HashMap<K,V>` operations backed by intrinsic | `std/collections/hash_map.ark:15`, `:20` |
| `HashMap_String_i32_insert` etc. (monomorphic generated names) | Raw | Direct monomorphic stub; should stay wrapper-internal | `std/collections/hash_map.ark:21` |

**Migration sketch**: ensure `HashMap_*` generated names are not re-exported or are
prefixed `internal_` in the public surface; Facade `hashmap_*` names become the only
documented entry points.

#### `hash_set` (`std/collections/hash_set.ark`)

| Representative API | Proposed tier | Note | Source |
|--------------------|--------------|------|--------|
| `hashset_i32_new` / `hashset_i32_insert` | Facade | `HashSet<i32>` intent is surface-visible | `std/collections/hash_set.ark:9`, `:14` |
| `hashset_str_new` and other `hashset_str_*` | Raw → Facade | Currently backed by `Vec<String>` carrier; target `HashSet<String>` facade | `std/collections/hash_set.ark:59`, `:64` |

**Migration sketch**: promote `hashset_str_*` to full Facade once the `HashSet<String>`
monomorphic type stabilises; until then treat as Raw with a deprecation note pointing
to the future Facade name.

### `std::wit` (`std/wit/mod.ark`)

| Representative API | Proposed tier | Note | Source |
|--------------------|--------------|------|--------|
| `WitType` (enum) | Facade (type) | User-visible semantic classification of WIT primitives | `std/wit/mod.ark:6` |
| `wit_type_bool` … `wit_type_string` (constructor fns) | Facade | Readable enum constructors; stable named entry points | `std/wit/mod.ark:23` ff. |
| `wit_type_id` | Raw / interop | Returns integer wire ID; caller depends on numeric mapping | `std/wit/mod.ark:37` |
| `wit_type_from_id` | Raw / interop | Inverse integer-to-enum; exposes numeric wire contract | `std/wit/mod.ark:56` |
| `wit_type_name` | Facade | Stable display string for logging and diagnostics | `std/wit/mod.ark:74` |

**Migration sketch**: `wit_type_id` and `wit_type_from_id` are narrow interop helpers;
rename to `raw_wit_type_id` / `raw_wit_type_from_id` (or `internal_`) once a higher-level
WIT reflection API exists.  Constructor functions and `wit_type_name` stay Facade.

---

## Manifest Metadata

The `std/manifest.toml` entries for Raw helpers should carry:

```toml
stability = "unstable"
tier      = "raw"
# or
deprecated = "prefer facade: <facade_name>"
```

Adapter-layer entries carry `tier = "adapter"`.  Facade entries carry `stability = "stable"`
or `"experimental"` per the standard stability-policy.

---

## Summary

1. Facade is the default surface.  New users should never need Raw or Adapter names.
2. Raw helpers are renamed with `raw_` prefix or demoted to `internal_` during migration.
3. Adapter names stay as-is but are clearly documented as host/ABI boundary code.
4. Per-family migration sketches above serve as the implementation roadmap for follow-on issues.

See also:
- `docs/stdlib/stability-policy.md`
- `issues/done/384-stdlib-api-admission-gate.md`
