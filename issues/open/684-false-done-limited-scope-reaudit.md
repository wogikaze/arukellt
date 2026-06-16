---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 684
Track: docs-audit
Depends on: none
Orchestration class: audit-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: False-done / limited-scope re-audit framework 2026-06-17
Child tracks: 678
---

# 684 — False-done and limited-scope done issue re-audit program

## Summary

`docs/process/false-done-prevention.md` は FD-01–FD-10 を定義し、
`check-false-done-close-gates.py` は **done へ戻った issue** に gate を適用する。
一方、section 3 / 10 の「done だが limited」「parent gate 未達 child done」
パターンの **横断再監査** は手動 audit メモ依存。

本 issue は機械化された **done issue 健康診断** と reopen 候補リストの維持を担う。

## Audit checklist (section 3 + 8 + 10)

| FD / 兆候 | 機械化 | 現状 |
|-----------|--------|------|
| docs-only close (FD-03) | 部分 (`check-false-done-hygiene`) | 要拡張 |
| parse/stub = product (FD-04) | playground/LSP 個別 gate | **#685** |
| partial/limited/remains open (FD-09) | 文言 grep のみ | 本 issue |
| compile-only runtime close | gate 個別 | 本 issue |
| guard-only callable close | #034 再監査済み done | 要回帰 |
| skipped verification (FD-10) | component skip 率未監視 | **#682** |
| deleted path evidence (FD-05) | 手動 | 本 issue |
| parent gate 未達 child done (FD-06) | 部分 | 本 issue |
| bootstrap stub = prod (FD-07) | 文言のみ | **#668** |
| release checklist ↔ component/host | 未リンク | **#678** |

## Acceptance

- [ ] `scripts/check/check-done-issue-health.py` — done issue 本文から
      `remains open` / `partial` / `deferred` / `compile-only` / `guard-only` を抽出し
      gate evidence リンクの有無を検証
- [ ] 監査レポート `docs/process/done-issue-health-audit-YYYY-MM-DD.md` を四半期更新
- [ ] `check-false-done-close-gates.py` の TRACKED 一覧と done issue acceptance の差分ゼロ
- [ ] issue done 復帰ルール（fixture + manifest + verify）を `AGENTS.md` と一致
- [ ] Gate `scripts/check/gate-684-false-done-reaudit.py`
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `docs/process/false-done-prevention.md`
- `docs/process/false-done-audit-2026-06-12.md`
- `scripts/check/check-false-done-close-gates.py`
- `scripts/check/check-false-done-hygiene.py`
- `issues/open/678-verification-gates-docs-release.md`
