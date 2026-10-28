---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 959
Track: docs-audit
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 3
Source: "ABC481 arukellt port + WAT emission session (2026-10-28, HEAD 0a64bdb62)"
Related: "ADR-072, #878, #868"
---

# 959 — `std/prelude.ark` header still says 'auto-imported into every module', contradicting the ADR-072 v2 policy

## Summary

The file header of `std/prelude.ark` still claims:

```
// Arukellt Standard Prelude — auto-imported into every Arukellt module
...
// ## Tiny Prelude ... Functions provided by THIS file ...
```

After ADR-072 / #878 only the tiny stable set is auto-imported
(`std/manifest.toml [prelude_policy]`, `src/compiler/resolver/prelude_stable.ark`);
every other prelude declaration requires `use std::prelude::{name}` and bare calls fail
with `E0100` plus a migration hint. `docs/stdlib/prelude.md` already describes this
split correctly, so only the source comment is stale.

This misleads readers (and agents) into expecting `to_string`, `join`, `byte_len`,
`args`, ... to be in scope; they are not.

## Fix

- Rewrite the header comment to state the v2 contract and link ADR-072 /
  `docs/stdlib/prelude.md` / `std/manifest.toml`.
- Sweep other sources for the same stale wording (`grep -rn "auto-imported" std src docs`).

## Acceptance

- [ ] `std/prelude.ark` header matches ADR-072 and `docs/stdlib/prelude.md`.
- [ ] No remaining "auto-imported" claim that contradicts `[prelude_policy]`.
- [ ] No behavior change (docs-only diff).
