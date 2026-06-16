---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 679
Track: docs-audit
Depends on: none
Orchestration class: audit-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: Docs-to-runtime contract audit framework 2026-06-17
Child tracks: 668, 675, 678
---

# 679 — Docs-to-runtime contract audit (README / current-state / manual docs)

## Summary

README は「Wasm-first」「Component/WIT target: `wasm32-wasi-p2`」と掲げ、読者を
`docs/current-state.md` に誘導している。一方で generated README status block、
`target-contract.md`、`capability-surface.md`、manual docs の主張が **同期していない**
箇所がある。

本 issue は **実装1件の修正** ではなく、「読者に約束している面」と「verify で
裏付けている面」の差分を棚卸しし、未消化 gap ごとに子 issue へ分割する
**監査 umbrella** である。

## Audit checklist (section 1 + 6)

| チェック | 現状 (2026-06-17) | 起票/追跡 |
|----------|-------------------|-----------|
| README `Wasm-first` が component/host/interop まで裏付け | README は status block のみ；component は smoke/skip 依存 | → **#682** |
| README `Component/WIT target` が CI で保証 | `target-contract`: component-compile は wasm-tools 不在で skip-on-CI | → **#682** |
| README status block が current-state と一致 | README `Updated: 2026-05-14`、fixture 数・verify 表記が古い可能性 | 本 issue |
| current-state ↔ target-contract 同期 | P2 native: current-state は gate_074 緑、target-contract は「deferred v5+」 | → **#680**, **#668** |
| current-state ↔ capability-surface 同期 | host reachability 記述が `call_host_network.ark` 実装と食い違う疑い | → **#675**, **#681** |
| generated stdlib docs ↔ manifest availability | HTTP/sockets pages 存在、not user-reachable 注記はあるが一覧で弱い | → **#681** |
| legacy/archived docs が誤解を生まない | 要横断 grep + inventory | 本 issue |
| `docs/process/false-done-prevention.md` と矛盾する landing claim | 要監査 | 本 issue |

## Acceptance

- [ ] 監査レポート `docs/process/docs-runtime-contract-audit-2026-06-17.md`（または同等）に
      上表全行の判定（OK / gap / deferred）と evidence path を記載
- [ ] 各 gap 行が **open child issue** または **fixed + gate** にマップ済み（未マップ行ゼロ）
- [ ] README generated status block の source of truth を1つに固定（`project-state.toml` か
      verify 出力かを ADR/issue で明記）
- [ ] `scripts/check/check-docs-consistency.py` に README ↔ current-state 必須フィールド
      比較を追加（P2 native tier、component smoke tier、host reachability サマリ）
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Close gate

`scripts/check/gate-679-docs-runtime-contract-audit.py` — 監査レポート存在、
gap→issue マップ完全性、README/current-state 機械比較 green。

## Out of scope

- 個別 feature 実装（#668–#678 が担当）
- stdlib API 追加

## References

- `README.md` (generated status block)
- `docs/current-state.md`
- `docs/target-contract.md`
- `docs/capability-surface.md`
- `docs/process/false-done-prevention.md` (FD-03, FD-04)
