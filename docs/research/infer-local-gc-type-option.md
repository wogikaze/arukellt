# GC local infer: Option&lt;i32&gt; contract

Canonical contract for wasm GC local-type inference (`infer_*` in
`src/compiler/wasm/code_ref_locals*.ark`).

## Inference API

Every GC-type `infer_*` (and `resolve_named_struct_gc_type`) returns
`Option<i32>`:

| Value | Meaning |
|---|---|
| `Option::Some(idx)` | Wasm GC type index (`idx >= 0`) |
| `Option::None` | No GC type could be determined |

Callers must not treat raw `-1` as absence. Use `match` / `Option::Some` /
`Option::None`.

## Cache API (`local_gc_infer_cache`)

Three-state contract (equivalent to `Option<Option<i32>>`):

| API | Meaning |
|---|---|
| `!cached_infer_is_computed` | Uncomputed |
| `cached_infer_as_option` → `None` when computed | Computed miss |
| `cached_infer_as_option` → `Some(idx)` | Computed hit |

`cached_infer_set(ctx, idx, Option<i32>)` stores a computed result only.
Storage keeps a private `Vec<i32>` encoding (`-2` / `-1` / `>=0`) behind
these APIs. That encoding must not leak into inference return types.

## Out of scope

These `-1` (or other) encodings stay as-is:

- MIR operands (`dest` / `arg*` / `variant_slot` = no local / stack)
- `resolve_fn_index` miss
- `find_stack_value_source` instruction index
- `name_index` type cache (`type_idx+2`)
- layout lookup helpers that still return `i32` (convert at the Option boundary)

## SelfEmitCtx + Option match trap

Matching prelude `Option` inside a function that also has `SelfEmitCtx` in
scope currently traps (`unreachable`) in the selfhost wasm emitter. Returns
of `Option<i32>` from such functions are fine; convert via helpers in
`code_ref_locals_option.ark` (`option_i32_is_some` / `option_i32_unwrap`)
instead of `match` at the call site.

Cache API uses the same three-state contract as `Option<Option<i32>>`,
exposed as `cached_infer_is_computed` + `cached_infer_as_option` over a
private `Vec<i32>` encoding (`-2` / `-1` / `>=0`).
