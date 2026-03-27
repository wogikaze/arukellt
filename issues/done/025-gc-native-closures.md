# GC-native closures: call_ref + ref.func

**Status**: open
**Created**: 2026-03-27
**Updated**: 2026-03-27
**ID**: 025
**Depends on**: 022
**Track**: gc-native
**Blocks v1 exit**: no

## Summary

Replace `call_indirect` (table-based dispatch) with `call_ref` (typed
function references). Remove TableSection and ElementSection from the
module. `FnRef` operands produce `ref.func`, indirect calls use `call_ref`.

Current MIR represents closures as synthetic functions (`__closure_N`) with
captures as extra parameters — this model is retained. The change is purely
at the Wasm instruction level.

## Design

```wat
;; Before (bridge mode):
(table 10 funcref)                           ;; function table
(elem (i32.const 0) $fn0 $fn1 ...)          ;; populate table
(call_indirect (type $sig) (local.get $idx)) ;; indirect call by table index

;; After (GC-native):
(ref.func $fn0)                              ;; produces (ref $sig)
(call_ref $sig)                              ;; typed direct call via ref
```

**FnRef mapping:**

```
Operand::FnRef("foo") → ref.func $foo  ;; produces (ref $func_type_of_foo)
```

**CallIndirect mapping:**

```
Operand::CallIndirect { callee, args, .. }
→ emit args, emit callee (ref.func), call_ref $sig
```

## Acceptance Criteria

- [ ] `TableSection` is not emitted in GC-native mode.
- [ ] `ElementSection` is not emitted in GC-native mode.
- [ ] `Operand::FnRef(name)` emits `Instruction::RefFunc(func_idx)`.
- [ ] `Operand::CallIndirect { callee, args }` emits args + callee ref
      then `Instruction::CallRef(type_idx)`.
- [ ] Closure captures still work via extra parameters to synthetic functions.
- [ ] All `t3-compile:closure_capture/*` fixtures compile.
- [ ] All `run:closure_capture/*` fixtures pass execution.
- [ ] All `t3-compile:integration/*` fixtures compile (those using closures).
- [ ] All `run:integration/*` fixtures pass execution.
- [ ] HOF (map/filter/fold in Vec) work with call_ref (coordinates with 024).

## Key Files

- `crates/ark-wasm/src/emit/t3_wasm_gc.rs` — FnRef, CallIndirect emission, module build
- `crates/ark-mir/src/lower.rs` — closure synthesis

## Notes

- `ref.func` requires the function to be declared in a `DeclarativeElementSection`
  if it's used as a value (not just called directly). Check if wasm_encoder
  needs this. If so, emit a minimal declarative element segment for referenced
  functions.
- `call_ref` is typed — the type index must match the function signature.
  The `func_types` map in GcTypeRegistry handles this.
- Functions referenced by `ref.func` must have their indices known at
  module construction time — ensure the function index assignment is stable.
