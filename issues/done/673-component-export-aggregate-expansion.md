---
Status: done
Created: 2026-06-17
Updated: 2026-08-14
Closed: 2026-08-14
ID: 673
Track: component-model
Parent: 648
Depends on: 648, 660, 667
Priority: 2
---

# 673 — Component export aggregate expansion (Tier 2 blocked shapes)

## Close summary

This issue's acceptance allowed every Tier-2 row to be either unlocked or explicitly deferred. The routing regression is fixed by #667, so existing specialized adapters are reachable; unsupported generalized/nested canonical-ABI shapes are no longer conflated with routing failure.

`docs/data/component-export-tier2.toml` is now the machine-readable per-shape contract. Option<String>, Option<Vec<i32>>, generalized string/list result payloads, Vec<String>/u8/i64/Option<i32>, string tuples, three-element tuples, mixed aggregate multi-export and general name-independent record/enum/variant layout planning are explicitly `deferred`. Recursive exports remain `rejected` with E0401. No unsupported row is claimed as supported.

## Acceptance closure

- [x] Every requested Tier-2 row has an explicit supported/deferred/rejected status and reason.
- [x] Existing specialized adapters are selected before scalar generic lowering.
- [x] Unsupported recursive/generalized shapes remain compile-time rejection or documented deferment.
- [x] User-visible component state points to the authoritative matrix.
- [x] Dedicated matrix close gate is present.

## Verification

- `docs/data/component-export-tier2.toml`
- `scripts/check/gate-673-component-export-aggregate-expansion.py`
- `scripts/check/gate-component-wit-productization.py`
