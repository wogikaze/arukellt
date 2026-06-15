# WIT resource handles (`resource`, `own<T>`, `borrow<T>`)

Design for Tier 3 component emission of WIT resource types at the canonical ABI
boundary. Tracked by [#473](../../issues/done/473-wit-resource-handles.md).

## Scope (phase 1)

Phase 1 supports **fixture-backed** resource shapes only:

| Shape | Export round-trip | Import binding | Notes |
|-------|-------------------|----------------|-------|
| `resource` declaration in `--wit` | via Ark `resource` decl + `--emit wit` | WIT preflight accepts | Text emitter emits `resource { ... }` |
| `own<T>` in export signature | `pass_handle(i32) -> i32` adapter | `take(own<handle>)` as `i32` param | Handle is opaque `i32` at core Wasm |
| `borrow<T>` in import signature | — | `peek(borrow<handle>)` as `i32` param | No ownership transfer |
| `stream<T>` / `future<T>` | — | — | Still **E0402** ([#474](../../issues/open/474-async-component-support-v5.md)) |

General resource descriptors, handle-table lifecycle, and `std::component` surface
remain future work ([#054](../../issues/open/054-std-wit-component.md), [#121](../../issues/done/121-wasi-p2-canonical-abi-hardening.md)).

## Lifecycle model

WIT resources have identity at the component boundary. Arukellt maps them to GC
`resource` declarations in source; across the canonical ABI they are **opaque i32
handle indices** (Component Model spec: `own<T>` / `borrow<T>` lower to `i32`).

### `own<T>` — ownership transfer

- **Export lift**: core Wasm returns `i32` → component boundary presents `own<T>`.
- **Import lower**: caller passes `own<T>` → guest receives `i32` and consumes the handle.
- Phase 1 does **not** emit `resource.drop`; handles are treated as caller-managed
  indices in fixture adapters (same pragmatic stance as early flags support in #651).

### `borrow<T>` — temporary borrow

- **Import lower**: caller passes `borrow<T>` → guest receives `i32` for the call duration.
- The guest must not retain the index past the call; no handle-table removal on return.

### Handle table (future)

Full lifecycle (create, retain, drop, borrow rules, `resource.drop` canonical builtin)
requires a per-component handle table mapping `i32 → GC ref`. The retired Rust
`ark-wasm` prototype sketched `$__handle_table`; selfhost T3 will adopt the same
model when #054 lands. Phase 1 adapters pass handles through as raw `i32`.

## Fixture contract

### Export: `export_resource_roundtrip.ark`

- Ark: `resource Handle { id: s32 }` + `pub fn pass_handle(h: i32) -> i32`.
- Component wrapper: type section `resource` + `own<handle>`, canon lift on `own<handle>`.
- `--emit wit` golden: `export_resource_roundtrip.expected.wit`.

### Import: `import_resource_handle_type.ark`

- WIT: `test:host/handles` with `take(own<handle>)` and `peek(borrow<handle>)`.
- Ark: `host::take(1)` — both import functions use `i32` at the language surface.

### Boundary: `import_async_resource_type.ark`

- WIT with `stream<T>` / `future<T>` remains **E0402** until [#474](../../issues/open/474-async-component-support-v5.md).

## Diagnostics

| Code | When |
|------|------|
| E0402 | Async resource shapes (`stream<T>`, `future<T>`) in `--wit` files |
| E0402 | (removed for supported `resource` / `own<T>` / `borrow<T>` fixture shapes) |

## Verification

```bash
python3 scripts/manager.py verify quick
python3 scripts/check/gate-473-wit-resource-handles.py
```
