---
Status: done
Created: 2026-03-29
Updated: 2026-03-30
ID: 178
Track: main
Depends on: 177
Orchestration class: implementation-ready
Blocks v1 exit: False
# v5 TypeChecker: inference and typed HIR
---
# v5 TypeChecker: inference and typed HIR

## Summary

型推論・ユニフィケーション・typed HIR 生成をまとめて追跡する。Phase 2 の完了条件を "resolver がある" ではなく、typed HIR が出ることに寄せる。

## Acceptance

- [x] type inference / unification の責務が明確
- [x] 型エラー diagnostics を追跡できる
- [x] backend へ渡す typed HIR surface が定義されている

## References

- `issues/open/177-v5-resolver-name-binding-and-imports.md`
- `issues/done/168-v5-ir-spec-doc.md`
- `crates/ark-mir/src/lower/`