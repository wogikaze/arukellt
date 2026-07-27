---
Status: done
Created: 2026-07-14
Updated: 2026-07-24
ID: 808
Track: compiler
Depends on: "686"
Orchestration class: blocked
Orchestration upstream: 686
Blocks v{N}: none
Priority: 3
Source: CQ-18 audit — unresolved verify full failures need open owner
---

# 808 — T3/Wasm validation failures

## Summary

`verify quick` reported an aggregate T3 fixture WASM validation failure.
The selfhost compiler now emits Wasm that `wasm-tools validate` accepts for
the full T3 fixture set (modulo intentional skips).

## Exact failure scope (resolved)

Final gate (2026-07-24, commit `e18c09aa`):

```text
T3 WASM validation: 428 pass, 0 validate-fail, 0 compile-fail, 23 skip (total 451)
```

Command:

```bash
rm -rf .build/t3-cache && python3 scripts/check/check-t3-wasm-validate.py
```

## Closure evidence

Root causes addressed across the #808 series:

1. Nested enum / Result payload ABI used a boxed `GS_f0_ref14` instead of
   open-enum `ref14` (ref-vs-ref on match/`?`).
2. Nested generic mono keys rejected `Vec_i32_` suffixes, leaving stale param
   types (ref-vs-ref on `count__Vec_i32_`).
3. Assert / loop temps kept stale open-enum local storage while the body
   stored i64 (`i64-vs-ref` on `stdlib_trait/*`). Demotion is gated so string
   concat, string `NEQ`, and array/format ref locals are not demoted.

## Validation command

```bash
python3 scripts/check/check-t3-wasm-validate.py
```

## Removal condition

Met: validate-fail count is 0; `check_t3_wasm_validate` in
`docs/data/release-guarantees.toml` is `pass`.
