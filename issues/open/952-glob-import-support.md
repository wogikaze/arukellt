---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 952
Track: parser
Depends on: none
Orchestration class: design-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: "ABC481 arukellt port + WAT emission session (2026-10-28, HEAD 0a64bdb62)"
Related: "ADR-072, #878, #917"
---

# 952 — glob import (`use path::*`) is not supported, so the v2 explicit prelude policy requires long name lists

## Summary

`use std::prelude::*` is a parse error (`E0001|parse: expected token`). There is no
wildcard form for imports at all. Since v2 narrowed auto-import to the tiny stable
prelude (ADR-072, #878), programs that use several prelude helpers must enumerate
every name:

```ark
use std::prelude::{to_string, join, push, len, get_unchecked, set, byte_len, byte_at, i32_to_i64, i64_to_i32, String_from}
```

A glob form restores ergonomics without widening the stable auto-import set, which is
the policy ADR-072 protects.

## Repro

`tmp/glob.ark`:

```ark
use std::host::stdio
use std::prelude::*

fn main() {
    stdio::println(to_string(1))
}
```

```
$ scripts/run/arukellt-selfhost.sh compile --target wasm32 --emit wasm -o .build/glob.wasm tmp/glob.ark
error[E0001|parse]: expected `token`
```

Expected: parse succeeds and every public `std::prelude` declaration that may be
imported explicitly becomes visible (or a documented subset).

## Scope

1. Parser / AST: `use path::*` (and nested `use a::b::*`) in `src/compiler/parser/`.
2. Resolver: expand glob to public declarations only; define duplicate/shadowing and
   ambiguity diagnostics (a glob must not silently shadow a local or another import).
3. Formatter and LSP import handling, highlighting fixtures.
4. Docs: ADR-072 note that glob does not change the stable auto-import contract.

## Acceptance

- [ ] Positive fixture: `use std::prelude::*` + bare `to_string`/`join` compiles and runs on `wasm32` and `wasm32-gc`.
- [ ] Diagnostic fixture: ambiguity or shadowing through glob is rejected with a stable code.
- [ ] `python3 scripts/manager.py fmt --check` and `verify lane` pass for changed files.
