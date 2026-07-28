---
Status: done
Created: 2026-07-28
Updated: 2026-07-28
ID: 844
Track: compiler-internal
Depends on: none
Related: "ADR-007, #819"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: Repro while porting AtCoder practice2_k LazySegmentTree to Ark (wasm32)
---

# wasm32: `const` loads mis-lower (`expected i32, found ref`)

## Summary

Top-level `const` bindings type-check and compile on `--target wasm32`, but the
emitted module is invalid / fails wasmtime validation: const loads are lowered
as GC / ref types where an `i32` (or scalar) is required.

Workarounds today: typed locals (`let m: i64 = 998244353`) or a trivial helper
`fn MOD() -> i64 { ... }` instead of `const MOD`.

## Exact failure

Minimal fixture:

```ark
use std::host::stdio

const MOD: i32 = 998244353

fn main() {
    stdio::println((MOD).to_string())
}
```

```bash
arukellt compile fixture.ark --target wasm32 --wasi-version wasi-p1 -o fixture.wasm
wasmtime run fixture.wasm
```

Observed (wasmtime 46):

```text
Error: failed to compile: wasm[0]::function[...]::main
Caused by:
    Invalid input WebAssembly code at offset ...:
    type mismatch: expected i32, found (ref $type)
```

Also fails when widening via an intermediate local:

```ark
const MOD: i32 = 998244353

fn modulus() -> i64 {
    let mod_i32: i32 = MOD
    i32_to_i64(mod_i32)
}
```

`const MOD: i64 = 998244353` is rejected at typecheck (integer literals are
i32-typed), which is separate from this emit bug.

## Non-goals

- Changing `const` surface syntax or requiring i64 literal suffixes.
- Fixing unrelated wasm32 GC/ref leaks outside const load paths.

## Acceptance

- [x] The minimal `const MOD: i32` fixture above compiles and runs under
      `wasmtime run` on `--target wasm32 --wasi-version wasi-p1`
- [x] `let x: i32 = MOD` and `i32_to_i64` after loading `MOD` produce valid
      wasm32 (no ref/`$type` mismatch on const load)
- [x] A regression fixture lives under `tests/fixtures/` and is covered by the
      wasm32 / T3 (or equivalent) gate used for scalar lowering
- [x] Stdlib files that already use `const` (e.g. `std/collections/trie.ark`)
      remain valid; no new skip is added for this bug

## Validation command

```bash
# after adding the regression fixture
arukellt compile tests/fixtures/<const-wasm32-fixture>.ark \
  --target wasm32 --wasi-version wasi-p1 -o /tmp/const-wasm32.wasm
wasmtime run /tmp/const-wasm32.wasm
python3 scripts/manager.py verify lane --gate t3
```

## Owner

compiler team (wasm32 scalar / const emit)

## Removal condition

Close when the acceptance checks pass and the workaround comment in user
programs (e.g. `fn MOD()`) is no longer required for wasm32.

## Close note

- **Status**: done (moved 2026-07-28)
- **Fix commit**: `cbb22d45`
- **Fix summary**: `src/compiler/mir/lower/core_names.ark` now emits a zero-argument
  `MIR_CALL` for `const` functions whose value type is not `VT_FUNCREF`,
  allocating the return value with the correct scalar local type instead of
  emitting a bare function reference.
- **Regression fixture**: `tests/fixtures/scalar/const_i32.ark`
- **Regression test**: `scripts/tests/test_wasm32_const_load.py`
- **Verification**: `verify lane --gate t3` PASS, `verify quick` PASS (master 147/147)
