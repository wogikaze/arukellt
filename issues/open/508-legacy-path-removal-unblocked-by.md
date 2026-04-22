# Legacy path removal is blocked by CoreHIR lowerer stub

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-22
**ID**: 508
**Depends on**: 593
**Blocks**: completion of issue 285 acceptance item "all fixtures pass legacy-less"
**Track**: corehir
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #529
**Priority**: 4

**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.

**Operational lane**: legacy removal / selfhost transition. Keep separate from #125/#126 trusted-base compiler default-path correction and from #099 selfhost frontend design.

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
