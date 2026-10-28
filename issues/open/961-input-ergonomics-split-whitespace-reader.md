---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 961
Track: stdlib-api
Depends on: none
Orchestration class: design-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: "ABC481 arukellt port ergonomics review (2026-10-28, HEAD 0a64bdb62)"
Related: "#957, #960, ADR-046, ADR-072, #936"
---

# 961 — Competitive-programming input ergonomics: no `split_whitespace`, no unwrap for `Result<i32, String>`, and stdin is documented as unavailable

## Summary

The ABC481 solutions carry ~60 lines of tokenizer and explicit `use std::prelude::{...}`
for what should be a 5-line program. The intended target is:

```ark
use std::host::stdio

fn main() {
    let x: i32 = stdin.read_i32()
    let y: i32 = stdin.read_i32()
    if x + y >= 10 { stdio::println("2") } else { stdio::println("1") }
}
```

Measured blockers (2026-10-28, s2 built from HEAD 0a64bdb62):

1. **No whitespace tokenizer.** `String::split(" ")` exists but is broken on both
   targets (#960) and does not handle newlines. There is no `split_whitespace`.
2. **No `.unwrap()` for `Result<i32, String>`.** `String::parse_i32()` exists
   (`std/core/raw.ark` L178) and returns `Result<i32, String>`, but
   `std/core/container.ark` only implements `Result<i32, i32>`, so `.unwrap()` does
   not typecheck (`E0200: type mismatch: ? vs ?`) and callers must write a `match`.
3. **`stdio::read_stdin()` is documented as returning empty** (`std/host/stdio.ark`;
   issue #957) even though the runtime reads stdin, so tooling avoids it and falls
   back to file-path arguments.

Already ergonomic today (verified): `stdio::println("2")` accepts a string literal,
`n.to_string()` is a method and needs no import, `push`/`len` are auto-imported, and
`x: i64 >= 10` infers the literal.

## Proposed minimal surface (method-first, ADR-046/ADR-072)

1. `String::split_whitespace(self) -> Vec<String>` in `std/collections/string.ark`
   (plus manifest entry and fixtures).
2. `impl Result<i32, String> { fn unwrap(self) -> i32 }` (panic on `Err`), or a
   `unwrap_or_zero`-style method; same for `parse_i64`.
3. A tiny stdin reader, e.g. `std::host::stdio::Scanner` with `next_i32()` /
   `next_i64()` / `next_token()`, built on `read_stdin` (issue #957), so streaming
   parsers do not need to materialize the whole input twice.
4. Optional: a `std::cp`-style prelude module for contest code that re-exports (1)-(3)
   so solutions do not need three imports.

With (1)+(2) the A problem becomes:

```ark
use std::host::stdio

fn main() {
    let t: Vec<String> = stdio::read_stdin().split_whitespace()
    let x: i32 = t[0].parse_i32().unwrap()
    let y: i32 = t[1].parse_i32().unwrap()
    if x + y >= 10 { stdio::println("2") } else { stdio::println("1") }
}
```

## Acceptance

- [ ] A problem (`A - Digits of A+B`) fits in <= 10 lines without a manual tokenizer,
      and runs on `wasm32` with stdin-only input.
- [ ] `split_whitespace` fixture covers `\n`, `\r\n`, multiple spaces, empty input.
- [ ] Public API additions are recorded in `std/manifest.toml` with stability and have
      `$implementation-strategy` / ADR-046 review notes.
