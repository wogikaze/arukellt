---
Status: done
Created: 2026-04-03
Updated: 2026-06-14
ID: 480
Track: docs
Depends on: 479
Orchestration class: implementation-ready
Blocks v1 exit: no
Upstream: "#479 (LSP 実装) — 完了後に着手"
---

# Extension README: 設定一覧テーブル追加

## Summary

`extensions/arukellt-all-in-one/README.md` に `## Extension Settings` セクションを設け、
`package.json` の全 10 設定を型・デフォルト・説明付きで一覧化する。
`docs/cli-startup-contract.md` の Extension settings 表も同じ契約に同期する。

## Close Evidence

- `extensions/arukellt-all-in-one/README.md` — `## Extension Settings` (line 29): unified table for all 10 `contributes.configuration.properties` keys; LSP forwarding note cites #479 and matches `extension.js` initializationOptions keys
- `docs/cli-startup-contract.md` — Extension settings table updated: `arukellt.target` default `null` (was stale `"wasm32-wasi-p1"`), five LSP behaviour settings + `playgroundUrl` added
- Key/default parity: all README table keys and defaults match `extensions/arukellt-all-in-one/package.json`
- `grep "Extension Settings" extensions/arukellt-all-in-one/README.md` → 1 hit
- `grep "check.onSave" extensions/arukellt-all-in-one/README.md` → 1 hit
- Upstream #479 closed (`issues/done/479-lsp-config-struct-and-handler-behavior.md`)
- `python scripts/manager.py verify quick` — 163/163 pass
