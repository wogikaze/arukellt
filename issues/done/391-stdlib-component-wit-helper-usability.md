---
Status: done
Created: 2026-03-31
Track: main
Orchestration class: implementation-ready
Depends on: none
Closed: 2026-07-28
ID: 391
# Stdlib: component / WIT helper の実用性を見直す
---
# Stdlib: component / WIT helper の実用性を見直す

## Completed

- [x] component / WIT helper の責務境界が docs に明記される — Both modules have doc comments explaining experimental status and current scope
- [x] 少なくとも 2 つ以上の利用例が追加される — component_helpers.ark (ABI version, model version) and wit_types.ark (type constants, name lookup)
- [x] helper ごとに fixture または integration test が用意される — 2 new fixtures in stdlib_component/ and stdlib_wit/, 611 total passing
- [x] 未成熟な helper は stability tier が見直される — Both modules remain experimental in manifest (stability = "experimental")