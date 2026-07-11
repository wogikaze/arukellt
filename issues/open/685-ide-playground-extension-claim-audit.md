---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 685
Track: docs-audit
Depends on: 679
Orchestration class: audit-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: IDE / Playground / Extension claim audit framework 2026-06-17
---

# 685 — IDE / Playground / Extension product-claim vs compiler gate audit

## Summary

false-done audit では Playground typecheck、LSP、extension が reopen lane として
挙がっている。VS Code extension と LSP は **advertised capabilities**（completion,
hover, codeAction）を持つが、WIT import や component 等の **compiler-backed gate**
より先に product claim している箇所を監査する。

## Audit checklist (section 7)

| チェック | 現状 (2026-06-17) | 起票/追跡 |
|----------|-------------------|-----------|
| playground typecheck が parse shim のみ | 要 playground gate 確認 | 本 issue |
| playground wasm export が削除 crate 依存 | FD-05 対象 | **#684** |
| LSP completion/hover が WIT import 未対応 | #669 open | **#669** |
| extension が起動だけで言語機能を証明していない | #569/#191 gate あり | 要マトリクス |
| debug adapter E2E が薄い | release-checklist あり | **#678** |
| browser playground と T2 freestanding 混同 | 要 docs 監査 | **#680** |
| IDE feature が stdlib manifest 更新に追従 | #334–#340 baseline | 本 issue |

## Acceptance

- [ ] Capability matrix: advertised IDE feature × compiler fixture gate × stdlib manifest
- [ ] Playground が claim する機能一覧を `docs/current-state.md` に明示
- [ ] WIT/component 未対応 IDE 機能に「experimental / not for WIT imports」ラベル
- [ ] Extension README の claim を gate 一覧とリンク
- [ ] Gate `scripts/check/gate-685-ide-claim-audit.py`（matrix TOML + verify）
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `extensions/arukellt-all-in-one/README.md`
- `issues/open/669-wit-import-ide-formatter.md`
- `docs/history/reports/false-done-audit-2026-06-12.md`
