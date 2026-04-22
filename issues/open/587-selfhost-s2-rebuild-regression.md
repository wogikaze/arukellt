# 587 — Selfhost s2 rebuild regression: type errors on every fixture from current `src/compiler/main.ark`

**Status**: open
**Created**: 2026-04-23
**Updated**: 2026-04-23
**ID**: 587
**Depends on**: —
**Track**: selfhost-frontend
**Orchestration class**: investigation-required
**Orchestration upstream**: —
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

- [ ] Bisect identifies the offending commit
- [ ] Root cause documented (probable: an early-return → continue conversion in #567 leaks a sentinel/poison type into a later check, OR a slice-c MIR emit path produces an uninstantiated MonoInstance reference)
- [ ] Fix lands; rebuilding s2 from current `src/compiler/main.ark` via the pinned bootstrap produces a wasm that passes `selfhost fixture-parity` / `diag-parity` without relying on the cached s2/s3
- [ ] CI cache invalidated (or the gate switched to always-rebuild); subsequent `python scripts/manager.py verify` rc=0 on a cold cache
- [ ] If the fix requires bootstrap rotation (the pinned wasm is itself wrong), update `bootstrap/arukellt-selfhost.wasm` + `bootstrap/PROVENANCE.md` per #585's procedure and document the rotation in resolution

## Risk if unaddressed

- All Phase 6/7 work that touches the selfhost frontend will start hitting confusing failures the moment any cache-invalidating change lands.
- Bootstrap rotation (necessary for #585 hygiene) is currently impossible.
- Future agents may chase ghost regressions thinking their slice broke things, when in fact the breakage is pre-existing.

## Notes

This is hygiene + correctness, not new feature work. Should be triaged before further selfhost-frontend slices to prevent compounded regression.
