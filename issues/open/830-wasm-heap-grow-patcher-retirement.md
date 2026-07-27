---
Status: open
Created: 2026-07-25
Updated: 2026-07-25
ID: 830
Track: selfhost-infra
Parent: 727
Depends on: "730"
Related: "#727, #686, #813"
Orchestration class: architecture-implementation
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: Split from #727 Related section — patcher retirement is not part of HTTP/sockets bridge acceptance
---

# 830 — Retire `wasm-heap-grow-patcher` (walrus) from selfhost bootstrap

## Summary

`scripts/bootstrap/wasm-heap-grow-patcher` (depends on `walrus`) post-processes
the pinned bootstrap wasm before stage-2 compilation. It should be retired so
the selfhost pipeline does not need this external Rust dependency.

This work was originally described under `#727` as "Related", but it is **not**
part of `#727` acceptance (HTTP/sockets `arukellt_host` retirement). It is
tracked here as a child issue and coordinated with `#730` (bootstrap memory /
pinned refresh).

## What the patcher does

1. **Memory expansion**: bumps `initial` to 65536 pages (4 GiB) and removes
   `maximum`.
2. **Vec_new overflow guard**: replaces bump allocator prologue with a
   u32-wraparound-aware version.
3. **Export deduplication**: removes duplicate export names (first-wins).

A related binary patch (`_patch_bootstrap_disable_selfhost_mir_prune` in
`scripts/selfhost/checks.py`) flips `prune=1` → `prune=0` in the pinned wasm.

## Root causes to fix before deletion

1. Pinned wasm stale memory section (prefer refresh from current source /
   Memory64 path already advancing under `#730`).
2. `Vec_new` missing u32 wraparound detection in
   `src/compiler/wasm/intrinsic_vec_new_layout.ark`.
3. Duplicate export names — verify `sections_exports.ark` covers all cases.
4. MIR prune stripping emitter functions — ensure pinned / lower path uses
   no-prune for selfhost (`src/compiler/mir/lower/entry.ark`).

## Acceptance

- [ ] Compiler-side root causes above are fixed or superseded by `#730`
      pinned/Memory64 path such that patcher steps are no-ops
- [ ] `scripts/bootstrap/wasm-heap-grow-patcher/` deleted and removed from
      workspace `Cargo.toml` members
- [ ] Patcher / prune binary-patch call sites removed from
      `scripts/selfhost/checks.py` (and related runners)
- [ ] CI no longer builds the patcher (`.github/workflows/ci.yml`)
- [ ] `python3 scripts/manager.py verify quick` passes without the patcher

## Non-goals

- HTTP/sockets / `arukellt_host` migration (`#727`)
- Performance tuning of `memory.grow` after removal

## References

- `issues/done/727-arukellt-host-bridge-retirement.md` (original Related section)
- `issues/open/730-bootstrap-wasm-4gb-memory-limit.md`
- `docs/plans/arukellt-host-bridge-retirement.md` §3.2
- `scripts/bootstrap/wasm-heap-grow-patcher/src/main.rs`
- `src/compiler/wasm/intrinsic_vec_new_layout.ark`
- `src/compiler/wasm/sections_memory.ark`
- `src/compiler/mir/lower/entry.ark`
