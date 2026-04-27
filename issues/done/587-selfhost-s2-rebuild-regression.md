---
Status: done
Created: 2026-04-23
Updated: 2026-04-23
ID: 587
Track: selfhost-frontend
Depends on: —
Orchestration class: investigation-required
Orchestration upstream: —
---

# 587 — Selfhost s2 rebuild regression: type errors on every fixture from current `src/compiler/main.ark`
**Blocks**: 583-followup, future bootstrap rotation
**Blocks v5**: yes (silently — current verification passes only via stale cache)
**Source**: Flagged by `impl-312-slice-d` final report 2026-04-23.

## Summary

Rebuilding `.build/selfhost/arukellt-s2.wasm` from the **current** `src/compiler/main.ark` via the pinned `bootstrap/arukellt-selfhost.wasm` produces an `arukellt-s2.wasm` that emits massive type errors when invoked on any fixture.

`python scripts/manager.py selfhost {fixpoint,fixture-parity,diag-parity}` currently pass **only because they reuse a stale cached `arukellt-s2.wasm` / `arukellt-s3.wasm` from before this regression was introduced**. Once the cache is invalidated (any change that triggers a rebuild from the pinned bootstrap), the gates will start failing across the board.

This regression is **pre-existing on master as of 2026-04-23**; it predates #312 slice-d and is unrelated to that slice's reachability pruning. It is therefore landed silently across some recent selfhost-frontend slice.

## Reproduction

```bash
# From a clean checkout on current master (commit 38a6e250 or later):
rm -rf .build/selfhost/arukellt-s2.wasm .build/selfhost/arukellt-s3.wasm
python scripts/manager.py selfhost fixpoint   # expect failures during s2 rebuild
# OR force the rebuild path explicitly:
wasmtime run --dir=/::/  bootstrap/arukellt-selfhost.wasm -- compile src/compiler/main.ark -o /tmp/s2.wasm
wasmtime run --dir=/::/  /tmp/s2.wasm -- check tests/fixtures/<any>.ark
# Observe massive type errors emitted by the freshly built s2.
```

## Investigation

Bisect candidate range: every selfhost-frontend commit landed since the last successfully-rebuilt `arukellt-s2.wasm` cache snapshot. Likely candidates:

- `#312` slices a/b/c (`1dfa4b3e`, `57f4e617`, `cba27c9e`) — generic monomorphization changes
- `#565` (`14a2e3ff`) — lexer recovery
- `#566` (`ccb62f68`) — parser partial AST recovery
- `#567` (`41e6f32b`) — incremental diagnostic accumulation in resolver/typechecker

The regression most plausibly sits in #312 slice-c (MIR monomorphization) or #567 (typechecker accumulation), since both rewire major parts of the typechecker / lowering pipeline that the bootstrap exercises when re-compiling itself.

## Acceptance

- [x] Bisect identifies the offending commit
- [x] Root cause documented
- [x] Fix lands; rebuilding s2 from current `src/compiler/main.ark` via the pinned bootstrap produces a wasm that passes `selfhost fixture-parity` / `diag-parity` without relying on the cached s2/s3
- [x] CI cache invalidated (or the gate switched to always-rebuild); subsequent `python scripts/manager.py verify` rc=0 on a cold cache (modulo 4 pre-existing failures)
- [x] No bootstrap rotation required (pinned wasm is correct; the bug was a stale stub-struct in `src/compiler/mir.ark`)

## Resolution

**Bisect result.** Offending commit: `cba27c9e` — `feat(selfhost): #312 slice-c — MIR monomorphization (specialized fns + call rewrite)`. Confirmed via cold-rebuild bisect across the candidate range; `cba27c9e^` produces a working `arukellt-s2.wasm`, `cba27c9e` does not.

**Root cause.** `src/compiler/mir.ark` carries its own *stub* declarations of `MonoInstance` and `TypeCheckResult` (lines 1133–1143 in pre-fix HEAD) so that `lower_to_mir` can take a `TypeCheckResult` parameter without needing the full typechecker source visible to its module. Slice-c added two new fields to the canonical structs in `src/compiler/typechecker.ark`:

- `MonoInstance` gained `type_args: Vec<TypeInfo>`
- `TypeCheckResult` gained `mono_call_sites: Vec<MonoCallSite>` (and a brand-new `MonoCallSite` struct)

…but did **not** propagate those additions to the `mir.ark` stubs. The pinned bootstrap’s `ctx_field_index` resolves struct fields by *first matching struct-name in declaration order* (and returns `0` when the field is absent from that match), so when the bootstrap re-parsed current HEAD it hit the stale stubs first and resolved `check_result.mono_call_sites` to **field index 0** (`error_count`). The freshly built `arukellt-s2.wasm` therefore read an `i32` (interpreted as a `Vec` pointer) at the head of `TypeCheckResult` and trapped on the first dereference — and inside `typecheck_module` the symmetric write `result.mono_call_sites = env.mono_call_sites` corrupted memory beyond the (still-wrong-size) `TypeCheckResult` allocation, producing the nondeterministic “35,000 type errors” signature on every fixture.

**Fix commit.** `<this commit>` (feat/587-s2-bisect): two struct stub updates in `src/compiler/mir.ark` — add `type_args: Vec<TypeInfo>` to the stub `MonoInstance`, add the missing `struct MonoCallSite { span_start: i32, mangled_name: String }`, and add `mono_call_sites: Vec<MonoCallSite>` to the stub `TypeCheckResult`. No initializers or runtime code change in `mir.ark`; the stubs are field-layout declarations only. Once the stubs match the canonical typechecker definitions, the bootstrap’s field-offset lookup returns the correct indices and both `check_result.mono_call_sites` (read in `lower_to_mir`) and `result.mono_call_sites` (written in `typecheck_module`) compile to the right offsets.

**Cold-rebuild verification numbers (post-fix, fresh `.build/selfhost/`):**

```
$ rm -rf .build/selfhost/arukellt-s{2,3}.wasm
$ python scripts/manager.py selfhost fixture-parity   # PASS
$ python scripts/manager.py selfhost diag-parity      # PASS
$ python scripts/manager.py selfhost fixpoint         # SKIP (pre-existing — see #585)
$ python scripts/manager.py verify quick              # 15 pass / 4 fail (same 4 pre-existing
                                                      #   failures: fixture manifest, done/
                                                      #   checkbox audit, doc example check,
                                                      #   broken internal links)
$ wasmtime run --dir . bootstrap/arukellt-selfhost.wasm -- compile src/compiler/main.ark \
    --target wasm32-wasi-p1 -o /tmp/s2.wasm
  → ok (compilation succeeded, phase 6)
$ wasmtime run --dir . /tmp/s2.wasm -- check tests/fixtures/hello/hello.ark
  → ok (compilation succeeded, phase 4)
$ wasmtime run --dir . .build/selfhost/arukellt-s2.wasm -- compile tests/fixtures/hello/hello.ark \
    --target wasm32-wasi-p1 -o /tmp/hello.wasm
  → wrote 491 bytes
```

**Bootstrap rotation.** Not required. The pinned `bootstrap/arukellt-selfhost.wasm` is correct; the regression was strictly in current HEAD source where slice-c failed to keep the `mir.ark` stubs aligned with the canonical typechecker structs. `bootstrap/PROVENANCE.md` is unchanged.

**Follow-up note for future selfhost slices.** When adding fields to typechecker structs that are also declared (as stubs) in `mir.ark`, the stubs must be kept field-layout-aligned with the canonical declarations, otherwise the pinned bootstrap will silently miscompile field offsets the next time selfhost is rebuilt cold.

## Risk if unaddressed

- All Phase 6/7 work that touches the selfhost frontend will start hitting confusing failures the moment any cache-invalidating change lands.
- Bootstrap rotation (necessary for #585 hygiene) is currently impossible.
- Future agents may chase ghost regressions thinking their slice broke things, when in fact the breakage is pre-existing.

## Notes

This is hygiene + correctness, not new feature work. Should be triaged before further selfhost-frontend slices to prevent compounded regression.
