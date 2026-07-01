# T3 Backend: Read-Modify-Write Optimization Analysis

## Status

**Resolved**: T3 backend already generates optimal RMW code for struct fields.
No additional peephole optimization is needed at this time.

## Pattern Analysis

### RMW pattern: `x.field = x.field op value`

The T3 Wasm GC backend compiles struct field read-modify-write as:

```wasm
;; x.count = x.count + 1
local.get $x          ;; ref for struct.set (bottom of stack)
local.get $x          ;; ref for struct.get
struct.get $T $field   ;; read field (consumes top ref)
i32.const 1            ;; operand
i32.add                ;; compute new value
struct.set $T $field   ;; write back (consumes bottom ref + value)
```

This is **6 instructions**, which is the minimum for a stack machine without a `dup`
instruction. Both `struct.get` and `struct.set` consume the struct reference, so two
`local.get` instructions are required.

### Multi-field RMW: `scale(p, factor)`

```wasm
;; p.x = p.x * factor; p.y = p.y * factor
local.get 0          ;; ref for struct.set
local.get 0          ;; ref for struct.get
struct.get 12 0      ;; read p.x
local.get 1          ;; factor
i32.mul              ;; p.x * factor
struct.set 12 0      ;; write p.x
local.get 0          ;; ref for struct.set
local.get 0          ;; ref for struct.get
struct.get 12 1      ;; read p.y
local.get 1          ;; factor
i32.mul              ;; p.y * factor
struct.set 12 1      ;; write p.y
```

Each field update is 6 instructions — optimal.

### Why further reduction is not possible

Wasm's stack machine has no `dup` instruction. To have a struct reference available
for both `struct.get` (which consumes it) and `struct.set` (which also consumes it),
two separate `local.get` instructions are required. The only alternative would be
`local.tee` to save a copy, but that uses a scratch local and doesn't reduce
instruction count.

## Existing peephole optimizations (peephole.rs)

| Pattern | Replacement | Condition |
|---------|------------|-----------|
| `local.set X; local.get X` | `local.tee X` | opt_level >= 1 |

## Remaining optimization opportunities

1. **Dead local elimination**: Functions emit 12+ scratch locals (i32, i64, f64, ref, anyref
   slots) even when unused. Reducing this would shrink function bodies.
2. **Stdlib-internal RMW**: Hand-written stdlib functions (vec.push, hashmap.insert) in
   `stdlib.rs` use explicit temporaries. These could be tightened within the Rust emit code.
3. **Cross-statement `local.tee`**: When `local.set X; local.get Y; local.get X` appears
   (from consecutive MIR statements), the `local.set X; local.get X` peephole already
   converts this to `local.tee X; local.get Y` when adjacent.

## Verification

Tested with `--target wasm32-wasi-p2` on struct field update fixtures.
Wasm-tools print confirms optimal instruction sequences.
