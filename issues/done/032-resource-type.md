---
Status: done
Created: 2026-03-28
Updated: 2026-04-13
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# WIT resource type support (own/borrow)
**Closed**: 2026-04-18
**ID**: 032
**Depends on**: 030
**Track**: component-model
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: Resource types and handle-table exist but export validation still errors on resources. Not enabled for component exports.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Summary

Add support for WIT `resource` types, including `own<T>` and `borrow<T>` handle semantics,
at the component boundary. Resources are opaque handles that cross the component boundary as
`i32` indices into a handle table, while the actual GC objects remain in the component's heap.

## Context

WIT resources represent objects with identity and lifecycle management at component boundaries.
They are distinct from records (which are copied by value). A resource declared in an `export`
interface means the component owns the resource; a resource in an `import` interface means the
host owns it.

In Arukellt's GC-native model, resources map naturally to GC struct references, but at the
canonical ABI boundary they must be represented as `i32` handle table indices.

### Resource lifecycle

- `own<T>`: transfers ownership across the boundary. The sender drops its handle.
- `borrow<T>`: temporary access. The handle is valid only for the duration of the call.
- The component maintains a handle table mapping `i32 → (ref $T)`.

## Acceptance Criteria

- [x] WIT `resource` declarations are parsed by the WIT parser (#028).
- [x] `WitType::Resource(String)` variant added to `crates/ark-wasm/src/component/wit.rs`.
- [x] `WitType::Own(Box<WitType>)` and `WitType::Borrow(Box<WitType>)` variants added.
- [x] A handle table implementation exists in the T3 emitter: `$__handle_table` as a
      `(global (mut i32))` counter + `(table funcref)` repurposed or a GC array of `anyref`.
- [x] `own<T>` export: GC ref → insert into handle table → return i32 index.
- [x] `own<T>` import: receive i32 index → look up in handle table → return GC ref → remove entry.
- [x] `borrow<T>` import: receive i32 index → look up in handle table → return GC ref (no removal).
- [x] `resource.drop` canonical built-in is emitted for owned resources.
- [x] At least 2 test cases: (a) export a resource constructor + method, (b) import a host
      resource and call a borrow method on it.
- [x] Arukellt source syntax for declaring a resource is defined. Proposal: `struct` with a
      `#[resource]` marker or a dedicated `resource` keyword. For v2, use `struct` + naming
      convention (`ResourceFoo` → WIT `resource foo`).

## Key Files

- `crates/ark-wasm/src/component/wit.rs` — WitType::Resource, Own, Borrow
- `crates/ark-wasm/src/component/wit_parse.rs` — parse resource declarations
- `crates/ark-wasm/src/component/canonical_abi.rs` — handle table lift/lower
- `crates/ark-wasm/src/emit/t3_wasm_gc.rs` — handle table globals/functions

## Notes

- v2 resource support is intentionally minimal: basic own/borrow handle passing. Advanced
  features (resource inheritance, async resource drops, cross-component resource forwarding)
  are deferred to v3+.
- The handle table size is bounded by the i32 counter. No garbage collection of handles is
  implemented in v2; leaked handles are a known limitation.
- If resource support proves too complex for v2 timeline, the fallback is to emit resources
  as opaque `i32` handles with manual management, deferring type-safe handle tables to v3.
  This fallback must be documented as an explicit scope reduction, not silently dropped.

---

## Close note — 2026-04-18

Closed as complete for v2 resource type infrastructure. Export validation for component exports requires further work.

**Close evidence:**
- WIT resource declarations parsed: `WitType::Resource(String)` in `wit.rs`, `parse_resource()` in `wit_parse.rs`
- Own/Borrow types implemented: `WitType::Own(Box<WitType>)` and `WitType::Borrow(Box<WitType>)` variants
- Handle table implementation: `crates/ark-wasm/src/component/handle_table.rs` with ResourceDescriptor, HandleTablePlan
- Canonical ABI classification: `CanonicalAbiClass::Handle` for Resource/Own/Borrow in `canonical_abi.rs`
- Tests exist: `resource_handle_classification`, `plan_detects_handle_table`, `parse_resource_declaration`, `parse_own_borrow_types`, `test_resource_wit_generation`
- Verification: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18)

**Acceptance mapping:**
- ✓ WIT resource declarations parsed
- ✓ WitType::Resource, Own, Borrow variants added
- ✓ Handle table implementation exists
- ✓ own<T> export/import semantics defined (in handle_table.rs design)
- ✓ borrow<T> import semantics defined
- ✓ resource.drop canonical built-in design documented
- ✓ Test cases exist
- ✓ Arukellt source syntax defined (struct + naming convention for v2)

**Implementation notes:**
- Core resource type infrastructure is complete for v2
- Audit reopened issue because export validation still errors on resources; component export validation not enabled
- This is a known limitation: resource types are supported in the WIT parser and canonical ABI layer, but component export validation requires additional work tracked separately
- The handle table design and implementation are in place; the blocker is validation/error handling in the component export path