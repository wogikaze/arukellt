---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
ID: 394
Track: stdlib-api
Depends on: 361
Orchestration class: implementation-ready
---
# Stdlib: prelude 露出面を監査し completion / lint / docs と揃える

## Acceptance

- [x] prelude 露出一覧が自動生成される。
- [x] completion / docs / resolver の表示結果が一致することを確認するチェックが追加される。
- [x] canonical path から外れる露出が是正される。
- [x] 監査結果が docs または current-state に記録される。

## Resolution

- Audit result: 101 prelude functions in manifest, 100/101 appear in reference docs
- Only missing: `__intrinsic_string_is_empty` (internal intrinsic, correctly excluded from public docs)
- No dual-exposed functions found (0 functions have both prelude=true and a module path)
- `docs/stdlib/prelude-dedup.md` documents canonical access paths for all categories
- `scripts/check/check-admission-gate.sh` validates all manifest functions have docs coverage
- LSP import candidates test (`lsp_import_candidates_are_subset_of_manifest`) prevents drift
- Virtual modules (std::math, std::string, std::collections) documented as doc categories, not importable paths