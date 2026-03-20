# Wasm GC Backend Spike

This document records a feasibility spike for a separate Wasm GC backend track.
It is intentionally exploratory. It does not change the current `wasm-js` or `wasm-wasi` production path, which still assumes linear memory plus helper functions.

## Why this is a separate track

Today `lang-backend-wasm` treats all heap-bearing surface values as linear-memory pointers:

- `WasmAbi::wasm_type()` maps `String`, `List`, `Tuple`, `Seq`, `Result`, `Option`, and heap-backed ADTs to `i32`
- `emit_construct()` allocates `4 + field_count * 4` bytes with `__alloc`, stores a numeric tag at offset `0`, then stores fields after that
- helpers such as `__option_unwrap_or` assume `Option<T>` is a heap cell in exported memory
- the exported `wasm-js` contract is "functions plus exported `memory`", and `wasm-wasi` also depends on linear memory for strings and host I/O scratch buffers

That makes Wasm GC a poor fit for the current backend shape, but still a plausible high-upside backend fork because it can remove wrapper allocations for some internal values.

## Concrete feasibility target

The smallest realistic spike is `Option<Int>` on a new GC-capable JavaScript-host target.

Current lowering:

- `Some(7)` becomes a heap allocation
- word `0` stores a tag
- word `1` stores payload `7`
- `None` is another heap object with only the tag
- `unwrap_or` calls `__option_unwrap_or`, which reads the tag from memory

GC-backed lowering:

```wat
(type $option_i32 (struct
  (field (mut i32))
))

(func $make_some_i32 (param $value i32) (result (ref $option_i32))
  local.get $value
  struct.new $option_i32
)

;; `None` is just `ref.null`.
(func $maybe_inc (param $flag i32) (result (ref null $option_i32))
  local.get $flag
  (if (result (ref null $option_i32))
    (then
      i32.const 7
      struct.new $option_i32
    )
    (else
      ref.null $option_i32
    )
  )
)

(func $unwrap_or_zero (param $opt (ref null $option_i32)) (result i32)
  local.get $opt
  (if (result i32)
    (ref.test (ref $option_i32))
    (then
      local.get $opt
      ref.cast (ref $option_i32)
      struct.get $option_i32 0
    )
    (else
      i32.const 0
    )
  )
)
```

The exact instruction spelling may vary with the final emitter/toolchain, but the representation claim is concrete:

- `Some(value)` can be one GC object with just the payload field
- `None` can be `ref.null`
- no linear-memory tag word is required
- no `__alloc` call is required
- no `__option_unwrap_or` helper is required

This is the cleanest candidate because `Option<T>` already behaves like a nullable wrapper semantically, and `Option<Int>` avoids the extra complexity of GC-backed strings or lists.

## Why `Option<Int>` is a good first slice

- It is already present in the backend surface through `strip_suffix(...).unwrap_or(...)` and `option.map(...)`.
- The current implementation pays extra allocation and helper cost for a value that GC refs can express directly.
- It exercises constructor emission, match/branch behavior, helper elimination, and type lowering without forcing a full redesign of string/list host interop.

This spike does not claim that `String` or `List<T>` should move first. Those types are entangled with the current memory export contract and host shims.

## Required backend changes

The current backend cannot express this with a small local patch. At minimum, these areas would need to change:

### 1. Target model

Add a separate experimental target instead of mutating `wasm-js`:

- likely shape: `wasm-js-gc`
- keep current `wasm-js` and `wasm-wasi` behavior unchanged
- gate the GC path on an explicit target contract so users do not accidentally depend on runtime support the old targets never promised

Reason: the existing targets are defined around linear memory, `i32`-shaped exported values, and helpers that assume `memory` exists.

### 2. Wasm type representation

`WasmAbi::wasm_type()` currently returns `Option<&'static str>`, which is enough for primitive WAT types like `i32` but not for:

- `(ref $type)`
- `(ref null $type)`
- emitted `(type ...)` definitions

The backend needs a richer representation layer, for example:

- a backend value-repr enum describing scalar vs nullable-ref vs concrete ref
- a place to register emitted GC type definitions such as `$option_i32`
- function signature emission that can print those richer types

Without this, GC support would turn the current string-based emitter into a fragile pile of special cases.

### 3. Constructor and match lowering

`emit_construct()` currently hard-codes two modes:

- fieldless enums on `wasm-js` become integer tags
- everything else becomes a linear-memory allocation plus stores

A GC path would need a third mode:

- `Option<Int>::Some` -> `struct.new`
- `Option<Int>::None` -> `ref.null`
- matches / `unwrap_or` / `map` must branch on nullability or use `struct.get`, not `i32.load`

### 4. Helper emission

These helpers are linear-memory-specific and would need to become backend-specific or disappear on the GC path:

- `__alloc`
- `__option_unwrap_or`
- any future option/result/list helper that assumes a tag word at offset `0`

This is a strong argument for separating helper selection by backend representation instead of by target name alone.

### 5. Memory/export policy

The GC backend should not promise the same boundary contract as current `wasm-js`.

A realistic first contract is:

- exported params/results remain scalar-only at the host boundary
- GC-backed values are allowed only for internal function calls inside the module
- string I/O and host calls can continue to use linear memory or be rejected on the GC target until adapters exist

That keeps the first slice meaningful without forcing immediate JS host adapters for arbitrary GC refs.

## Test impact

The existing backend tests mostly assume:

- WAT can be parsed to binary through `wat::parse_str`
- exported functions can be instantiated in Node and return scalar values
- emitted helpers can be asserted textually

A GC spike would need targeted additions:

### Codegen tests

Add WAT assertions for a tiny program such as:

```arukel
fn maybe(flag: Bool) -> Option<Int>:
  if flag:
    Some(7)
  else:
    None

fn main() -> Int:
  maybe(true).unwrap_or(0)
```

Checks:

- emitted WAT contains a GC type definition for the option payload case
- emitted WAT contains `struct.new`, `struct.get`, and a null-ref path
- emitted WAT does not contain `__option_unwrap_or`
- emitted WAT does not contain `call $__alloc` for the option value path

### Integration tests

Keep host-visible results scalar in the first slice:

- compile a program whose exported `main` still returns `Int`
- let internal helpers/functions traffic in `Option<Int>`
- instantiate under a GC-capable JS runtime only for that explicit target

### Tooling risk

If the current `wat` crate path cannot assemble the chosen GC text form, the first spike can still land as WAT-text emission tests only.
Broader adoption would then need either:

- a toolchain update that accepts the GC proposal syntax used here, or
- a binary emitter path that knows how to encode GC types/instructions directly

That risk is one more reason not to overload the current production targets.

## Target contract changes before broader adoption

Before this can be more than an experiment, the repo would need to define:

- which target name exposes GC semantics
- whether that target requires a JS host only, or any engine with the relevant proposal enabled
- whether exported GC refs are part of the supported ABI or intentionally forbidden at first
- whether linear `memory` is still exported for strings/host I/O on the hybrid path
- how docs describe feature availability versus current `wasm-js` and `wasm-wasi`

The safest first answer is: a new `wasm-js-gc` target, scalar-only public ABI, internal GC refs allowed.

## Feasibility verdict

Feasible as a separate backend track.

Not a near-term optimization for the current production backend.

The upside is real:

- fewer wrapper allocations for `Option`-shaped values
- fewer helper functions
- a path toward payload-bearing ADTs without encoding every variant as a manual heap record

The near-term mismatch is also real:

- current backend architecture assumes `i32` for all non-unit wasm values
- current helper library assumes linear memory
- current target contracts do not distinguish GC-capable hosts

So the migration path should be:

1. define a separate experimental target contract
2. add a GC-aware backend value representation layer
3. prove `Option<Int>` end to end with scalar exports only
4. only then evaluate broader adoption for payload ADTs, strings, or collections
