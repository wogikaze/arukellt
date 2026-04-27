---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
ID: 425
Track: repo-hygiene
Depends on: —
Orchestration class: implementation-ready
---
# Repo Hygiene: ディレクトリごとの ownership / maintenance map を作る

## Acceptance

- [x] directory ownership map が追加される。
- [x] 主要ディレクトリに役割と保守責任が記載される。
- [x] generated / archive / product / internal の区分が含まれる。
- [x] README または docs から辿れる。

## Resolution

- Created `docs/directory-ownership.md` with 4-tier classification (product/generated/internal/archive)
- 35+ directories mapped with tier, owner/generator, and description
- Generated files table with regeneration commands
- Default-build exclusions documented (ark-llvm, ark-lsp)