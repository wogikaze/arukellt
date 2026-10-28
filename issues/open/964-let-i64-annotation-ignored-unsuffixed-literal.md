---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 964
Track: typecheck
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: "ABC481 arukellt refactor session (2026-10-28, HEAD 0a64bdb62)"
Related: "#937, #933, #936, ADR-067"
---

# 964 — `let x: i64 = 0` binds x as i32 (annotation ignored for unsuffixed numeric literals)

## Summary

A `let` annotation does not influence an unsuffixed numeric literal: `let x: i64 = 0`
binds `x` as `i32`, and later i64 use fails.

```
error[E0200|typecheck]: type mismatch: i32 vs i64; no implicit numeric conversion (ADR-067); try `i32_to_i64(x)`
```

Writing `let x: i64 = 0i64` works, so the literal suffix wins over the annotation.
#937 covers annotations overriding *mismatching inferred* types for containers; this is
the scalar-literal half and blocks the natural `let mut cur: i64 = 0` idiom.

## Repro

```ark
use std::host::stdio
use std::prelude::i64_to_i32

fn main() {
    let x: i64 = 0
    let y: i64 = x + 1i64
    stdio::println(i64_to_i32(y).to_string())
}
```

```
$ scripts/run/arukellt-selfhost.sh compile --target wasm32 --emit wasm -o .build/leti64.wasm tmp/leti64.ark
error[E0200|typecheck]: type mismatch: i32 vs i64 ... (x is i32)
error[E0200|typecheck]: type mismatch in call to `i64_to_i32`: argument 1 has type i32, expected i64
```

## Fix direction

- When a numeric literal has no suffix and the binding has a concrete numeric
  annotation, type the literal by the annotation (standard bidirectional inference).
- Keep ADR-067's no-implicit-conversion rule for non-literals.

## Acceptance

- [ ] Repro prints `1` on both targets.
- [ ] Fixture: `let a: i64 = 0`, `let b: i64 = 5 + a`, `let c: f64 = 0` behave as annotated.
- [ ] Unsuffixed literal still errors when the annotation is incompatible (e.g. `let s: String = 0`).
