# GC-native full verification, cleanup, and ADR update

**Status**: done
**Created**: 2026-03-27
**Updated**: 2026-03-27
**ID**: 027
**Depends on**: 026
**Track**: gc-native
**Blocks v1 exit**: no

## Summary

Final integration phase. All fixture tests (346+) must pass with the
GC-native emitter. Remove all dead bridge-mode code paths from t3_wasm_gc.rs.
Update ADR-002 to note GC-native implementation is complete. Run full
verify-harness.sh and document the results.

## Acceptance Criteria

### Full fixture coverage

- [ ] All `t3-compile:*` fixtures compile (every category).
- [ ] All `run:*` fixtures produce correct output (every category).
- [ ] `scripts/verify-harness.sh` exits with status 0 (all 16 checks pass).
- [ ] `scripts/verify-harness.sh --quick` passes.

### Code cleanup

- [ ] Remove all linear-memory allocation code (bump allocator functions,
      alloc helpers) from t3_wasm_gc.rs.
- [ ] Remove `heap_ptr` global references (already done in 019, verify clean).
- [ ] Remove `call_indirect` / table-related code (already done in 025, verify clean).
- [ ] Remove any `// BRIDGE:` or `// TODO: GC-native` markers.
- [ ] Ensure no dead code warnings (clippy clean).

### Documentation

- [ ] Update `docs/adr/ADR-002-memory-model.md`: add "Implementation Status"
      section noting GC-native codegen is complete, listing key design
      decisions (subtype enums, bare string array, call_ref, I/O bridge).
- [ ] Update README.md if it mentions memory model or compilation strategy.

### Binary comparison

- [ ] Compare output .wasm binary sizes (GC-native vs bridge) for a few
      representative programs. Document in commit message or ADR.
- [ ] Verify no unnecessary sections (no table, no elem, no global for heap_ptr).

### Regression

- [ ] Run `cargo test --workspace --exclude ark-llvm --exclude ark-lsp` —
      all unit tests pass.
- [ ] Run `cargo clippy --workspace --exclude ark-llvm --exclude ark-lsp` —
      no new warnings.

## Key Files

- `crates/ark-wasm/src/emit/t3_wasm_gc.rs` — cleanup target
- `docs/adr/ADR-002-memory-model.md` — update
- `scripts/verify-harness.sh` — verification

## Notes

- This issue should be the very last one completed. All other GC-native
  issues must be done first.
- If any fixtures fail at this point, debug and fix here (don't create new issues).
- Consider running wasm-opt on a sample output to verify the GC module
  validates with external tools.
