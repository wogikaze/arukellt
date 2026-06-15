---
Status: done
Created: 2026-03-28
Updated: 2026-06-15
ID: 44
Track: stdlib
Depends on: 039, 044, 053
Orchestration class: implementation-ready
Blocks v3 exit: "no (Experimental)"
Status note: Closed 2026-06-15 — std::wit types/world/parser/printer + std::component handle/canonical ABI + 7 fixtures.
---

# std::wit + std::component: WIT 型、resource handle、canonical ABI

## Acceptance

- [x] WitType compound variants (`std/wit/types.ark`)
- [x] World / Interface / WitFunc builders (`std/wit/world.ark`)
- [x] WIT print subset (`std/wit/printer.ark`)
- [x] WIT parse subset (`std/wit/parser.ark`)
- [x] HandleTable i32 + `HandleTableI32` (`std/component/handle.ark`)
- [x] Canonical ABI string/list lower/lift (`std/component/canonical.ark`, ADR-008 scratch limits)
- [x] Fixtures: `stdlib_wit/*` (5) + `stdlib_component/*` (4)

## References

- `std/wit/`, `std/component/`
- `tests/fixtures/stdlib_wit/`, `tests/fixtures/stdlib_component/`
