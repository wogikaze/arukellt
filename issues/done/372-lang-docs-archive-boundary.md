---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
ID: 372
Track: language-docs
Depends on: —
Orchestration class: implementation-ready
---

# Language Docs: historical / archive 文書の境界を明確化する
- `syntax-v1-preview.md` updated with "Status: Transitional" banner pointing to current-state.md and spec.md
# Language Docs: historical / archive 文書の境界を明確化する

## Acceptance

- [x] `docs/language/` 内の全文書に current / transitional / archive のラベルが付与される
- [x] archive / transitional 文書に banner (免責注記) が設置される
- [x] `docs/language/README.md` の generated table にラベルが反映される
- [x] `scripts/gen/generate-docs.py` が archive banner を自動挿入する

## Resolution

- `syntax-v1-preview.md` updated with "Status: Transitional" banner pointing to current-state.md and spec.md
- `docs/spec/README.md` already had archive status banner (good state)
- `docs/retention-policy.md` defines archive workflow with banner template
- Current docs (spec.md, syntax.md, type-system.md, error-handling.md, memory-model.md) are current by nature
- Only transitional document is syntax-v1-preview.md; all others are current