---
Status: open
Created: 2026-08-31
Updated: 2026-08-31
ID: 850
Parent: 827
Track: selfhost-infra
Depends on: "827"
Related: "#827, #823, #730, #824, #834, docs/research/selfhost-phase-arena-ownership.md, docs/research/selfhost-compile-latency-root-cause.md"
Orchestration class: architecture-implementation
Orchestration upstream: 827
Blocks v4 exit: False
Priority: 1
Source: "#827 design-only close required a new issue before product arena code"
---

# 850 — Compiler phase-arena implementation (wasm32-gc host overlay)

## Summary

Implement the compiler-bootstrap phase arena decided in
[`docs/research/selfhost-phase-arena-ownership.md`](../../docs/research/selfhost-phase-arena-ownership.md).
`#827` is **design-only** and stays closed. This issue is the product-code tracker.

ADR-002 (user-program Wasm GC) does not change. This is bump / handle storage
inside `src/compiler/**` so the **gc-typed selfhost compiler** does not keep a
~1.5–1.8GB Copying-GC live set while compiling itself.

## Why this is the remaining path

Official goal: `BOOTSTRAP_EMIT_* = wasm32-gc` + `wasi-p2`, `sha256(s2)==sha256(s3)`,
cacheless flattened overlay of `src/compiler/main.ark` on that gc host **≤10s**.

Measured on HEAD `886581d1d` (do not treat wasm32 ≤10s as this goal):

| Host | Overlay | Notes |
|---|---:|---|
| wasm32 + wasi-p1 | ~10s | superseded as the *goal*; still the fast emit host |
| wasm32-gc + wasi-p2 (Copying) | **242s quiet / 255s loaded** | s2=s3, RSS ~1.75GB |
| wasm32-gc Null collector | ~23–26s then trap | mutator floor; discard does not help Null |
| wasm32-gc DeferredRC | ~279s | worse |

Closed families (do not retry): AST empty-fallback (`tick 35`, AST is not the 2GB),
lazy per-fn HIR (`tick 36`), discard/pack MIR (`ticks 37–41`), generic/skip/intern
micro-opts (`ticks 43–62` except landed 42/44/49), thin-shell discard (`tick 63`:
**97s trap, RSS still 1.52GB**). `#824` early-body-only-reachable is wontfix.

10s needs **both** live set **≪ 1GB** (function shells / LowerCtx / TypeTable, not
just insts) **and** less mutator work than today's lower. i32 handles + phase reset
are the structural way to get there without flipping `BOOTSTRAP_EMIT_*` early.

## Design constraints (from the #827 memo)

1. Reset only at phase boundaries: parse→typecheck, typecheck→lower, lower→emit.
2. No raw references across resettable arenas; cross-phase values are durable
   handles or copies into a non-reset durable bump.
3. Final Wasm bytes and durable tables (types, signatures, export names) must not
   live in a resettable phase arena.

## Acceptance

- [ ] Product arena / handle table exists in `src/compiler/**` and is used on a
      hot lower or emit path (not a stub, not stdlib `Arena<T>` demo-only)
- [ ] Reset points match the memo; no cross-arena raw refs
- [ ] Cacheless gc-host overlay of flattened `src/compiler/main.ark` stays
      `sha256(s2)==sha256(s3)` (or the *new* compiler is itself a fixpoint)
- [ ] Receipt: wall + RSS vs HEAD 242s quiet / 255s loaded / ~1.75GB
- [ ] Do **not** set `BOOTSTRAP_EMIT_*` to wasm32-gc until overlay is ≤10s
- [ ] `python3 scripts/manager.py verify lane` on the implementation slice
- [ ] No `--to-memory64` widen of memory32 wasm32-gc (`#834`)

## First slice (suggested)

Lower-phase durable handles for MIR temporaries (insts / locals / block
instruction vecs) so Copying GC scans i32 tables instead of record graphs.
Do not add fields to `MirFunction`. Side tables belong on `LowerCtx` or a
parallel session-owned bump.

**Do not reconstruct a fat `MirInst` on every `MirBlock_inst_at`.** Tick 64
(SoA + reconstruct-on-read) and tick 65 (in-place resolve, no before/after
snapshot) both cut RSS **1.75GB → 1.06GB** and both **timeout 320s**.
Fixing resolve alone is not enough: propagate + wasm emit still rematerialize.
Next slice must make those walks read SoA columns in place.

## Receipts

| Slice | Overlay | s2=s3 | RSS | Notes |
|---|---:|---|---:|---|
| HEAD `fee7d7588` | 242s quiet / 255s loaded | yes | ~1.75GB | baseline |
| tick 64 SoA + reconstruct-on-read | timeout 320s | — | 1.06GB | hello 2164; reverted |
| tick 65 SoA + in-place resolve | timeout 320s | — | 1.06GB | hello 2164; emit/propagate still reconstruct; reverted |

## Non-goals

- Reopening `#827` or `#097` (rejected Rust `bumpalo` rewrite)
- Language-level GC heap / ADR-002 change
- Coupling into `#824` early body lowering
- Official fixpoint on native-cpp
- Treating wasm-opt or AST cache as the 10s path
