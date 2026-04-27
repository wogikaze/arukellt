---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
ID: 377
Track: repo-hygiene
Depends on: —
Orchestration class: implementation-ready
---
# Repo Hygiene: scaffold / internal コンポーネントの露出 tier を定める

## Acceptance

- [x] コンポーネントの露出 tier (product / internal / scaffold / experimental) が定義される
- [x] `README.md` が product tier のみを主要セクションで紹介する
- [x] internal / scaffold tier のコンポーネントに README 内注記が付与される
- [x] `docs/current-state.md` が tier に準じた表記を使用する

## Resolution

- Created `docs/directory-ownership.md` with 4-tier classification: product / generated / internal / archive
- ark-lsp classified as internal (scaffold), ark-llvm as internal (LLVM 18 dependency)
- "Excluded from Default Build" section documents how to include non-default crates
- Tier definitions are consistent with docs/current-state.md status markers