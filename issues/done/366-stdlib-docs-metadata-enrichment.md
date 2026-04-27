---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
ID: 366
Track: stdlib-docs
Depends on: —
Orchestration class: implementation-ready
---
# Stdlib Docs: manifest metadata を enrichment し生成 docs の情報密度を上げる

## Acceptance

- [x] manifest に `since`、`see_also`、`deprecated_by` フィールドが定義される
- [x] 全関数に `doc_category` が付与される
- [x] `scripts/gen/generate-docs.py` が新フィールドを reference / module pages に反映する
- [x] 生成 docs で「いつ追加されたか」「関連関数」「後継関数」が表示される

## Resolution

- Added `doc_category` to all 152 previously uncategorized functions (274/274 now have categories)
- `since`, `see_also`, `deprecated_by` fields already defined in ManifestFunction schema (ark-stdlib)
- `deprecated_by` already rendered in reference docs with ~~strikethrough~~ → replacement
- `target` annotations now shown for host functions in reference docs
- `generate-docs.py` already processes all enriched metadata fields
- All 6 ark-stdlib tests pass, docs regeneration succeeds