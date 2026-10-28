---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 963
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: "ABC481 arukellt refactor session (2026-10-28, HEAD 0a64bdb62)"
Related: "#936, #953, #933, ADR-036, ADR-046"
---

# 963 — `i64.to_string()` is not usable (method dispatch picks the i32 impl)

## Summary

`std/core/convert.ark` implements `to_string` for i64 (`impl i64 { fn to_string(self: i64) -> String }`),
but calling it fails to typecheck:

```
error[E0200|typecheck]: type mismatch: i32 vs i64; no implicit numeric conversion (ADR-067); try `i32_to_i64(x)`
```

`i32.to_string()` works, so the method name is dispatched to the `i32` implementation
and the argument is rejected. The workaround is `i64_to_i32(x).to_string()` plus the
`i64_to_i32` import, which truncates values above 2^31 (or needs splitting).

## Repro

```ark
use std::host::stdio

fn main() {
    let x: i64 = 42
    stdio::println(x.to_string())
}
```

```
$ scripts/run/arukellt-selfhost.sh compile --target wasm32 --emit wasm -o .build/i64ts.wasm tmp/i64ts.ark
error[E0200|typecheck]: type mismatch: i32 vs i64; no implicit numeric conversion (ADR-067); try `i32_to_i64(x)`
```

## Notes

- Same family as #936 (core symbols dispatched by name), #953 (`Ord::cmp` on primitives),
  and #933 (MIR re-infers types by name). Method resolution for same-named impls across
  numeric types is the concrete trigger.
- `let x: i64 = 0` also binds i32 (see #964), so both problems often appear together.

## Acceptance

- [ ] Repro prints `42` on `wasm32` and `wasm32-gc`.
- [ ] Fixture covers `to_string` for i32/i64/f64/bool/char.
