---
Status: done
Created: 2026-04-15
Updated: 2026-05-16
ID: 508
Depends on: 593
Track: corehir
Orchestration class: blocked-by-upstream
Blocks: completion of issue 285 acceptance item "all fixtures pass legacy-less"
Orchestration upstream: #529
Priority: 4
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
Operational lane: legacy removal / selfhost transition. Keep separate from #125/#126 trusted-base compiler default-path correction and from #099 selfhost frontend design.
---

└─ lower_hir_to_mir          ← returns MirModule: ":new() (stub, always empty)"
let mut mir = MirModule: ":new();"
set_mir_provenance(&mut mir, MirProvenance: ":CoreHir);"
- [ ] All T1 + T3 fixtures pass with `MirSelection: ":CoreHir` (no legacy fallback)"
- 本 issue (#508) の `Depends on:` を `285` から `529` に張り替え、循環は解消。

# Legacy path removal is blocked by CoreHIR lowerer stub

## Summary

Issue #285 (legacy lowering path deprecation) cannot be fully completed because
`lower_hir_to_mir` — the entry point for the real CoreHIR lowerer — is currently
a placeholder that returns an empty `MirModule`. Every compilation path therefore
falls back to the legacy `lower_to_mir` (in `crates/ark-mir/src/lower/func.rs`).

Removing `lower_to_mir` would break **all** compilation fixtures (> 10 fixtures),
triggering the STOP_IF condition in issue #285.

## Evidence

The active call chain for every compilation as of 2026-04-15:

```
lower_check_output_to_mir
  └─ lower_corehir_with_fallback
       └─ lower_hir_to_mir          ← returns MirModule::new() (stub, always empty)
       └─ lower_corehir_via_legacy  ← always taken (deprecated via #285 work)
            └─ lower_hir_fallback
                 └─ lower_to_mir    ← the only real lowering implementation
```

`lower_hir_to_mir` in `crates/ark-mir/src/lower/facade.rs` builds structural statistics
from the CoreHIR program (item count, body count, etc.) but always returns empty MIR:

```rust
let mut mir = MirModule::new();
set_mir_provenance(&mut mir, MirProvenance::CoreHir);
push_optimization_trace(&mut mir, format!("corehir-snapshot ..."));
let _ = sink;
Ok(mir)
```

No function bodies are emitted. The Wasm backend requires at least the entry function
in the `functions` vec; an empty module always trips `ensure_runtime_entry`.

## Unblock Condition

This issue is unblocked when `lower_hir_to_mir` produces real `MirFunction` entries
for all Arukellt source constructs the fixture suite exercises. That is the CoreHIR
lowerer implementation work (probably a large chunk of the corehir track).

## Acceptance

- [ ] `lower_hir_to_mir` produces real MIR functions (entry + stdlib) for all fixture programs
- [ ] `cargo test` passes with `lower_to_mir` removed from the call path
- [ ] All T1 + T3 fixtures pass with `MirSelection::CoreHir` (no legacy fallback)
- [ ] `lower_to_mir`, `lower_legacy_only`, and related deprecated functions can be deleted
- [ ] Issue #285 acceptance items previously blocked are re-checked and closed

## Resolution path — 2026-04-22 (ADR-028)

[ADR-028](../../docs/adr/ADR-028-corehir-lowering-resolution.md) で
issue \#285 ⇄ \#508 の循環ブロッカーを設計判断で解消した。

要点:

- Rust 側 `lower_hir_to_mir` は実装しない (退役予定コードへの新規投資を回避)。
- Rust legacy lowering の撤去は #529 100% selfhost transition のクレート
  退役ステップに統合される。
- 本 issue (#508) の `Depends on:` を `285` から `529` に張り替え、循環は解消。
- `lower_hir_to_mir` は #529 退役完了まで現状の no-op スタブとして凍結。
  ADR-028 "Contract for `lower_hir_to_mir` (frozen until retirement)" を参照。

クローズ条件は ADR-028 "Done criteria" セクションに移譲する。

## Responsibility split — 2026-04-22

\#508 is a legacy-removal blocker record under #529. It is not the same lane as
\#125/#126, which correct the trusted-base compiler default path, and it is not
the selfhost frontend parser/design lane owned by #099. Do not group these as one
generic "compiler blockage" when choosing dispatch order.

## References

- `crates/ark-mir/src/lower/func.rs` — `lower_to_mir` (legacy implementation)
- `crates/ark-mir/src/lower/facade.rs` — `lower_hir_to_mir` (stub), `lower_corehir_with_fallback`
- `docs/compiler/legacy-path-status.md` — full pipeline state at time of deprecation
- Issue #285 (`issues/open/285-legacy-path-deprecation.md`)

---

## Closure Assessment — 2026-05-16

### Dependency status

- **#593 (selfhost Phase 1 multi-file fixpoint): DONE** (closed 2026-04-28, merged via `e0f419f3`)

### Codebase evidence

The entire Rust `crates/ark-mir/` crate — containing both the legacy `lower_to_mir` implementation
and the `lower_hir_to_mir` stub — has been **deleted** (commit `398c2d74`, closes #561,
dated after #529 Phase 5).

Current state of the compilation pipeline:

- The selfhost compiler (`src/compiler/mir_lower.ark`) provides the real `lower_to_mir`
  implementation (~51 functions, substantial lowering logic).
- The driver (`src/compiler/driver.ark`) calls `mir_lower::lower_to_mir` directly.
- No Rust-side `MirModule`, `MirFunction`, `MirProvenance`, `MirSelection`, `CoreHir`,
  `ensure_runtime_entry`, `lower_hir_to_mir`, `lower_corehir_with_fallback`,
  `lower_legacy_only`, or `lower_hir_fallback` symbols exist anywhere in `crates/`.

### Acceptance checklist (re-evaluated)

| Criterion | Status |
|-----------|--------|
| `lower_hir_to_mir` produces real MIR functions | **OBSOLETE** — per ADR-028, `lower_hir_to_mir` was never implemented in Rust. Instead the selfhost `lower_to_mir` provides the real lowering. |
| `cargo test` passes with `lower_to_mir` removed | **ALREADY TRUE** — the entire `crates/ark-mir/` crate was removed. |
| All T1 + T3 fixtures pass with `MirSelection::CoreHir` | **OBSOLETE** — `MirSelection` enum no longer exists. The selfhost lowerer is the only path. |
| `lower_to_mir` and related deprecated functions deleted | **ALREADY DELETED** — removed with `crates/ark-mir/`. |
| Issue #285 acceptance items re-checked and closed | **ALREADY DONE** — #285 was closed 2026-04-26. |

### Verification

`python scripts/manager.py verify quick` exits with **20/22 pass, 2 fail**.
The 2 failures are pre-existing documentation issues unrelated to legacy path removal:
1. Doc example parse/resolve errors in `docs/design/lang-uplift-gap-ledger.md`
2. Broken internal links

No regression or legacy-path-related failures.

### Conclusion

The legacy MIR-only path that #508 was created to track has been **fully removed**.
The ADR-028 resolution was executed: instead of implementing `lower_hir_to_mir` in
Rust, the selfhost compiler (`src/compiler/mir_lower.ark`) replaced the entire
`crates/ark-mir/` crate. The issue's dependency (#593) is done, and the blocker
condition has been resolved.

**Close note — 2026-05-16:** Issue superseded by #529 Phase 5 (crate retirement).
The `crates/ark-mir/` crate was deleted as #561. The selfhost compiler provides
the only MIR lowering implementation. No further action required on this issue.
Move to `issues/done/`.
