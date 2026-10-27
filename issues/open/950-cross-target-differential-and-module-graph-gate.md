---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 950
Track: selfhost-verification
Depends on: none
Orchestration class: verification-contract
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-029, ADR-057, #931, #933, #935, #936, #937, #938, #939, #940, #941, #942, #943, #944, #945, #946, #947, #948"
---

# 950 — verification never exercises the failure classes found in the P0 audit

## Summary

The defects in #935–#948 survive the current gates for structural reasons:

1. `s2 == s3` (ADR-029) proves that the compiler output is deterministic,
   not that the generated code is correct.
2. The selfhost compiler is built through the flat-src overlay, which merges
   namespaces (#931). The largest Ark program therefore never exercises
   module-qualified type identity (#940).
3. No gate runs the same program on `wasm32` and `wasm32-gc` and compares
   stdout. Several defects are invalid Wasm on one target and silently wrong
   output on the other (#936–#941).
4. Negative type-soundness cases (ill-typed programs that must be rejected)
   are not systematically gated.

## Evidence (2026-10-27)

All repros in #935–#948 were run with
`.build/selfhost/arukellt-s2-runtime.wasm` (sha256 `1e11e8e7f2c5…`) using
`scripts/run/arukellt-selfhost.sh compile` for both targets and
`wasmtime run`. Every one of them compiled ("compilation succeeded") on at
least one target.

## Acceptance

- [ ] Each repro from #935–#948 is registered in `tests/fixtures/manifest.txt`
      as a compile-error or run fixture. While its owning issue is open, it
      is marked as a known failure linked to that issue (no SKIP without an
      issue).
- [ ] A cross-target differential gate runs every positive run fixture on
      `wasm32` and `wasm32-gc` and fails on stdout / exit-code divergence
      (wired into `verify fixtures` or a named lane in
      `docs/data/verification-commands.toml`).
- [ ] A negative type-soundness suite (argument, annotation, return, unknown,
      prelude-name cases) runs on both targets.
- [ ] A module-graph compile of a multi-module program with same-named types
      runs in CI until #931 lets the selfhost compiler itself build through
      the module graph.
