---
Status: open
Created: 2026-10-11
Updated: 2026-10-25
ID: 917
Track: stdlib-api
Depends on: "709"
Orchestration class: incremental
Orchestration upstream: "#709 policy, #703 monomorphic cutover, #718 inventory"
Blocks v{N}: none
Priority: 1
Source: ADR-046 eradication execution umbrella split from #709 inventory
---

# 917 — Free-function eradication execution (must-delete / associated-or-method waves)

## Summary

Issue #709 produced the policy, classification table, and generated
scorecard for ADR-046 free-function eradication
(`docs/data/stdlib-api-eradication.toml`, `docs/stdlib/eradication.md`).
This issue owns the **execution**: actually deleting, deprecating, or
replacing every public free / `prelude_wrapper` / monomorphic helper
classified there, in dependency-safe waves.

## Current state

- `docs/data/stdlib-api-eradication.toml` now classifies both
  `std/manifest.toml` `[[functions]]` entries and top-level `pub fn`
  declarations under `std/**/*.ark`: 812 manifest functions + 447
  unregistered declarations = **1258** symbols (`must delete` 106,
  `associated-or-method` 1152, `intrinsic-only` 1).
- `scripts/check/check-stdlib-eradication.py` enforces the dual surface:
  no unclassified manifest entry or std declaration, no manifest entry
  without a source declaration, no dead rule, every public class has an owner.
- The generated scorecard counts remaining user-reachable free symbols;
  **zero** is the goal.
- Migration precedent: #718 (done) delivered the Tier-1 wave — method /
  associated replacements emitted before frees retired.
- Deletions are gated by ADR-014 (manifest 0.1.0, `remove_in 1.0.0`) and by
  the surface replacements owned by #703/#697/#708/#693/#710/#702.

## Required work

- [x] Phase 0 — measure top-level `pub fn` under `std/**/*.ark`, make the gate
      dual-surface (manifest + source declarations), and generate the real
      1258-symbol scorecard.
- [ ] Wave A — `must delete` entries whose replacement already exists
      (deprecated prelude wrappers, vec intrinsic-compat frees,
      monomorphic numeric conversions, `HashMap_*` builtins once
      `impl HashMap<K,V>` methods cover them). Apply ADR-014
      deprecation where the symbol is still reachable.
- [ ] Wave B — `associated-or-method` entries: land the inherent /
      trait / associated replacement first (per-owner issues #693,
      #702, #703, #708, #710, and the future Test / IO / collection
      types), then retire the free symbol.
- [ ] Keep the scorecard at zero-unclassified while waves land: update
      the TOML rules as symbols retire so the checker stays green.
- [ ] Update user-facing docs/examples in the same PR that retires a
      symbol family (docs-sync gate).

## Acceptance

- [ ] Generated scorecard shows 0 public free functions, 0
      `prelude_wrapper` entries, 0 monomorphic helpers.
- [ ] `python3 scripts/check/check-stdlib-eradication.py` passes.
- [ ] `std/manifest.toml` contains no `kind = "prelude_wrapper"`.
- [ ] Each retired family points at its method/associated replacement in
      release notes or migration docs.

## References

- ADR-046 (free-function eradication), ADR-014 (stability), ADR-044
- #709 (policy + inventory), #718 (Tier-1 wave, done), #703, #693,
  #702, #708, #719
- `docs/data/stdlib-api-eradication.toml`,
  `scripts/check/check-stdlib-eradication.py`,
  `std/manifest.toml`

