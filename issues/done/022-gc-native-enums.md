---
Status: done
Created: 2026-03-27
Updated: 2026-03-30
ID: 18
Track: gc-native
Depends on: 17
Orchestration class: implementation-ready
---
# GC-native enums: subtype hierarchy + br_on_cast
**Blocks v1 exit**: no

## Summary

Implement GC-native enum encoding using WasmGC subtype hierarchies. Each
enum becomes a base struct type (empty, non-final) with each variant as a
final subtype carrying its own payload fields. Pattern matching uses
`br_on_cast` for dispatch. Option<T> and Result<T,E> follow the same
pattern as regular enums.

## Design

```wat
;; enum Shape { Circle(f64), Square(f64), Rect(f64, f64) }
(type $Shape        (sub (struct)))                                ;; base, non-final
(type $Shape.Circle (sub final $Shape (struct (field f64))))       ;; variant 0
(type $Shape.Square (sub final $Shape (struct (field f64))))       ;; variant 1
(type $Shape.Rect   (sub final $Shape (struct (field f64) (field f64)))) ;; variant 2

;; Option<i32>
(type $Option      (sub (struct)))
(type $Option.Some (sub final $Option (struct (field i32))))
(type $Option.None (sub final $Option (struct)))                   ;; empty

;; Result<i32, String>
(type $Result      (sub (struct)))
(type $Result.Ok   (sub final $Result (struct (field i32))))
(type $Result.Err  (sub final $Result (struct (field (ref $string)))))
```

**EnumInit mapping:**

```
Operand::EnumInit { enum_name: "Shape", variant: "Circle", tag: 0, payload: [3.14] }
→ f64.const 3.14, struct.new $Shape.Circle
;; Result type is (ref $Shape.Circle), which widens to (ref $Shape) by subtyping
```

**Pattern matching with br_on_cast:**

```
Operand::EnumTag + br_table  →  replaced by:
  (block $v0 (result (ref $Shape.Circle))
    (block $v1 (result (ref $Shape.Square))
      (block $v2 (result (ref $Shape.Rect))
        (local.get $shape)
        (br_on_cast $v0 (ref $Shape) (ref $Shape.Circle))
        (br_on_cast $v1 (ref $Shape) (ref $Shape.Square))
        (br_on_cast $v2 (ref $Shape) (ref $Shape.Rect))
        (unreachable)
      ) ;; $v2: (ref $Shape.Rect) on stack
      ... handle Rect ...
    ) ;; $v1: (ref $Shape.Square) on stack
    ... handle Square ...
  ) ;; $v0: (ref $Shape.Circle) on stack
  ... handle Circle ...
```

**EnumPayload mapping:**

```
Operand::EnumPayload { object, index: 0, enum_name: "Shape", variant_name: "Circle" }
→ ref.cast (ref $Shape.Circle) (emit object)
  struct.get $Shape.Circle 0
```

**EnumTag compatibility:**
If MIR still emits `EnumTag` + `br_table`, support via `ref.test` chain:

```
(ref.test (ref $Shape.Circle) (local.get $shape))  ;; → 1 if Circle
(if (i32) (then (i32.const 0)) (else
  (ref.test (ref $Shape.Square) (local.get $shape))
  (if (i32) (then (i32.const 1)) (else (i32.const 2)))
))
```

But prefer rewriting match emission to use br_on_cast directly.

## Acceptance Criteria

- [x] GcTypeRegistry creates subtype hierarchies for every enum in enum_defs:
      one non-final base type + one final subtype per variant.
- [x] `EnumInit` emits `struct.new $Enum.Variant` with payload fields.
- [x] `EnumPayload` emits `ref.cast (ref $Enum.Variant)` then `struct.get`.
- [x] `EnumTag` either works via ref.test chain or match emission is rewritten
      to use `br_on_cast` chains.
- [x] Match expressions on enums work correctly with exhaustive checking.
- [x] Nullary variants (None, unit variants) use empty struct subtypes.
- [x] Option<i32>, Option<String>, Result<i32, String>, Result<i64, String>
      all work as enum subtype hierarchies.
- [x] All `t3-compile:enums/*` fixtures compile successfully.
- [x] All `run:enums/*` fixtures pass execution with correct output.
- [x] All `t3-compile:stdlib_option_result/*` fixtures compile.
- [x] All `run:stdlib_option_result/*` fixtures pass execution.
- [x] All `t3-compile:match_extensions/*` fixtures compile.
- [x] All `run:match_extensions/*` fixtures pass execution.

## Key Files

- `crates/ark-wasm/src/emit/t3_wasm_gc.rs` — EnumInit, EnumTag, EnumPayload, match emission
- `crates/ark-mir/src/mir.rs` — TypeTable.enum_defs

## Notes

- The `tag` field in `EnumInit` MIR operand becomes irrelevant for codegen
  (variant identity is carried by the struct type, not an integer). However,
  EnumTag backward compat may still need the ordinal.
- Specialized Result variants (Result_i64_String, Result_f64_String) each
  get their own subtype hierarchy.
- Nullary variant singletons (e.g., None) could be cached in a global to
  avoid allocation on every construction. This is an optimization, not
  required for correctness.