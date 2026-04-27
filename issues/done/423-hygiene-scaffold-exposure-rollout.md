---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
ID: 423
Track: repo-hygiene
Depends on: 377
Orchestration class: implementation-ready
---

# Repo Hygiene: scaffold / internal コンポーネントの露出 tier を README と docs に反映する
- Component tiers documented: product, generated, internal, archive
# Repo Hygiene: scaffold / internal コンポーネントの露出 tier を README と docs に反映する

## Acceptance

- [x] README の component 紹介が tier に沿って更新される。
- [x] current-state の表記も tier に揃う。
- [x] internal/scaffold に説明注記が付く。
- [x] 少なくとも主要露出箇所が一通り更新される。

## Resolution

- `docs/directory-ownership.md` provides the canonical tier reference
- ark-lsp and ark-llvm listed as "internal" with notes on why excluded from default build
- current-state.md's bootstrap section updated to reflect verified fixpoint status
- Component tiers documented: product, generated, internal, archive