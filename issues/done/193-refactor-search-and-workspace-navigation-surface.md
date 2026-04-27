---
Status: done
Created: 2026-03-29
Updated: 2026-03-29
ID: 193
Track: parallel
Depends on: none
Orchestration class: implementation-ready
---
# Refactor / search / workspace navigation surface
**Blocks v1 exit**: no

## Summary

rename、workspace symbols、multi-keyword fuzzy search、structural search/replace、edit graph、inline TODO graph をまとめて、workspace 全体を横断する refactor/search/navigation surface として追う。

## Acceptance

- [x] rename / workspace symbols / structural search の責務が追跡できる
- [x] edit graph や workspace navigation の責務が整理されている
- [x] TODO / symbol / reference 横断の導線を issue queue 上で追跡できる

## References

- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `crates/ark-lsp/src/lib.rs`
- `crates/ark-lsp/src/server.rs`