---
Status: done
Created: 2026-06-17
Updated: 2026-08-14
Closed: 2026-08-14
ID: 670
Track: language-design
Parent: 124
Depends on: 653
Priority: 2
---

# 670 — WIT import resolver hardening (duplicates, spans, collisions)

## Close summary

WIT import aliases and packages are now registered through span-aware resolver APIs. Duplicate aliases/packages and collisions with existing module/local namespaces use the declaration span rather than fixture-only string errors. Stable generated names use the shared `std::wit::names` helpers.

`--dump-phases backend-plan` now prints WIT import lowering summaries with package/interface and callable/type counts after MIR verification, making resolver lowering observable from the normal compiler pipeline.

## Acceptance closure

- [x] Duplicate alias/package registration is detected with source spans.
- [x] WIT alias collisions reuse normal resolver duplicate-symbol diagnostics.
- [x] WIT generated/display names use shared stable kebab/snake/Pascal transformations.
- [x] `backend-plan` includes WIT lowering information.
- [x] Dedicated duplicate-alias/package fixtures and close gate are present.

Unknown function/type references continue through the normal qualified resolver/typechecker diagnostic path, so their source locations are owned by the same span-aware symbol lookup rather than a separate WIT-only error channel.

## Verification

- `scripts/check/gate-670-wit-import-resolver-hardening.py`
- `scripts/check/gate-component-wit-productization.py`
