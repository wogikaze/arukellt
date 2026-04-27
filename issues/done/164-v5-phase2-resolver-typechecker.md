---
Status: done
Updated: 2026-03-30
ID: 164
Track: main
Depends on: 177, 178
Orchestration class: implementation-ready
Blocks v1 exit: False
Status note: Parent issue for semantic analysis. MIR lowering is tracked under
# v5 Phase 2: Resolver + TypeChecker epic
---
# v5 Phase 2: Resolver + TypeChecker epic


## Summary

旧 issue では Phase 2 に resolver / typechecker / HIR / MIR lowering / optimization が混在していた。semantic analysis と backend preparation を分離し、この issue は名前解決・型検査・typed HIR 完了の親 issue にする。

## Acceptance

- [x] #177, #178 が完了している
- [x] semantic analysis の責務が resolver/import binding と type inference / typed HIR に分離されている
- [x] HIR→MIR lowering は #165 側で追跡されている

## References

- `issues/open/163-v5-phase1-driver-cli.md`
- `issues/done/168-v5-ir-spec-doc.md`
- `crates/ark-resolve/src/`
- `crates/ark-mir/src/lower/`