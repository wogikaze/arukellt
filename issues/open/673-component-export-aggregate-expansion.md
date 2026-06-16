---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 673
Track: component-model
Parent: 648
Depends on: "648, 660 (done), 667"
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: P1 component export surface checklist audit 2026-06-17
---

# 673 — Component export aggregate expansion (Tier 2 blocked shapes)

## Summary

Issues #648/#660 closed umbrella gates with explicit boundaries. Many Tier 2 export shapes
remain `E0401` (`export_unsupported_*` fixtures): `Option<String>`, `Vec<String>`,
`Vec<u8>`, general record/enum/variant beyond Color/Point/Shape fixtures, and
3-element tuples.

This issue generalizes component export adapters beyond name-independent fixture
specializations. Coordinate with #667 so library routing uses generalized adapters,
not scalar-only bypass.

## Acceptance

Unlock or explicitly defer each row with fixture + `current-state.md` tier update:

- [ ] `Option<String>` and `Option<Vec<i32>>` export paths
- [ ] `Result<String, String>` and `Result<Vec<i32>, String>` export paths
- [ ] `Vec<String>`, `Vec<u8>`, `Vec<i64>`, `Vec<Option<i32>>` export paths
- [ ] `tuple<String, String>` and 3-element tuple exports
- [ ] General record / enum / variant adapters (remove Color/Point/Shape-only
      special casing in `contract_record_color_scan.ark` / variant scans)
- [ ] Mixed scalar + aggregate multi-export fixture (green path)
- [ ] Multi-export fixture with strings and lists (beyond string-only #660)
- [ ] Rejection tests for unsupported recursive export shapes remain `E0401`
- [ ] Canonical ABI memory allocation helper shared across new adapters
- [ ] Canonical ABI memory bounds checks on export lift/lower paths
- [ ] Extend `gate-648-component-export-general-abi.py` or add `gate-673`
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `docs/current-state.md` — Component export type tiers table
- `tests/fixtures/component/export_unsupported_*.ark`
- `issues/open/667-library-component-emit-routing-regression.md`
