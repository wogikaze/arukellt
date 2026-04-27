---
Status: done
Created: 2026-03-27
Updated: 2026-03-30
ID: 17
Track: gc-native
Depends on: 16
Orchestration class: implementation-ready
---
# GC-native user structs: struct.new / struct.get / struct.set
**Blocks v1 exit**: no

## Summary

Implement GC-native struct allocation and field access. User-defined structs
become real WasmGC struct types with typed fields. `StructInit` emits
`struct.new`, `FieldAccess` emits `struct.get`, field mutation emits
`struct.set`. Function parameters and return values use `(ref $StructName)`.

## Design

```wat
;; Example: struct Point { x: i32, y: i32, z: f64 }
(type $Point (struct
  (field (mut i32))    ;; x
  (field (mut i32))    ;; y  
  (field (mut f64))    ;; z
))
```

**StructInit mapping:**

```
Operand::StructInit { name: "Point", fields: [("x", val_x), ("y", val_y), ("z", val_z)] }
→ emit val_x, emit val_y, emit val_z, struct.new $Point
```

Fields must be emitted in declaration order (matching struct_defs layout).

**FieldAccess mapping:**

```
Operand::FieldAccess { object, struct_name: "Point", field: "y" }
→ emit object (produces (ref $Point)), struct.get $Point 1
```

**Field mutation (MirStmt::FieldAssign or similar):**

```
→ emit object, emit value, struct.set $Point $field_idx
```

## Acceptance Criteria

- [x] GcTypeRegistry creates a `(type $Name (struct ...))` for every entry
      in `type_table.struct_defs`, with correctly typed fields (i32/i64/f64
      for scalars, `(ref $T)` for reference-typed fields like String/Struct).
- [x] `Operand::StructInit` emits `Instruction::StructNew(type_idx)` with
      fields pushed in declaration order.
- [x] `Operand::FieldAccess` emits `Instruction::StructGet { struct_type_index, field_index }`.
- [x] Field mutation emits `Instruction::StructSet { struct_type_index, field_index }`.
- [x] Functions accepting/returning structs use `(ref null $StructName)` in signatures.
- [x] All `t3-compile:structs/*` fixtures compile successfully.
- [x] All `run:structs/*` fixtures pass execution with correct output.
- [x] Nested structs (struct containing struct fields) work correctly.

## Key Files

- `crates/ark-wasm/src/emit/t3_wasm_gc.rs` — StructInit, FieldAccess emission
- `crates/ark-mir/src/mir.rs` — TypeTable.struct_defs definition

## Notes

- Field order in `struct_defs` is `Vec<(String, String)>` — (name, type_name).
  The GC struct field order must match exactly.
- Reference-typed fields (e.g., a struct containing a String) produce
  `(field (mut (ref $string)))` — the GcTypeRegistry must resolve type names
  to GC type indices for nested references.
- Struct printing (println of a struct) is not required at this phase — it's
  handled when I/O builtins are complete.