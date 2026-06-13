---
Status: done
Created: 2026-04-03
Updated: 2026-06-13
ID: 462
Track: vscode-ide
Depends on: 477, 478, 479
---

# Extension settings rationalization (parent)

## Summary

VS Code `package.json` 設定 → extension `initializationOptions` → selfhost LSP server の end-to-end 配線。#479 完了により親 issue をクローズ。README 設定表は #480 で追跡。

## Child issues

| ID | Status | Scope |
|----|--------|-------|
| #477 | done | `package.json` manifest（5 settings） |
| #478 | done | extension.js wiring |
| #479 | done | server `lsp_config.ark` |
| #480 | **open** | README 設定テーブル・ユーザ向け docs |

## Acceptance（rollup 9/10）

- [x] #477 manifest
- [x] #478 extension wiring
- [x] #479 server behavior
- [x] lifecycle + verify pass

**Remaining (tracked in open #480)**: README 設定テーブル — see `issues/open/480-*.md`.

## Verification

- `python3 scripts/check/check-lsp-lifecycle.py` — 11/11 pass
- `python scripts/manager.py verify quick` — 150/150 pass

## Audit resolution — 2026-06-13

**Rollup note**: 実装・プロトコル層は #477–#479 で完了。#480（README）は意図的に open。

**Evidence**: `issues/done/477-*.md`, `478-*.md`, `479-lsp-config-struct-and-handler-behavior.md`
