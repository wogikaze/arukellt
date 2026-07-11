---
Status: done
Created: 2026-07-11
Updated: 2026-07-11
ID: 764
Track: docs-audit
Depends on: 761, 762, 763
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: Docs audit 2026-07-11 (P1-1..P1-4, P1-8 partial)
Blocks: 765
---

# 764 — Docs P1: lifecycle boundary, ownership, orphan current docs

## Summary

日付付き監査・計画が `process/` current 一覧に混在。重要ルート文書が
リンク孤立。`directory-ownership.md` が `docs/data/` を generated と誤分類。
release / benchmark / ownership の重複正本が残る。

## Acceptance

- [x] 日付付き監査・archived plan を `docs/history/`（または `docs/archive/reports/`）へ移動し、gate パスを更新
- [x] orphan current/reference 文書（cli-reference, release-*, test-strategy, retention-policy, ark-toml, debug-support, capability-surface, target-contract-summary）を README / sidebar から少なくとも 1 inbound link
- [x] `directory-ownership.md` で `project-state.toml` 等の入力正本と生成物を分離記載
- [x] process README に report/plan を current と同列掲載しない
- [x] docs-related verify gates for this slice pass (link integrity for moved reports; generated banner)

Front-matter `lifecycle` / `valid_as_of` 必須化は #765 へ移管。

## References

- Docs audit 2026-07-11 §P1-1..P1-4
- `docs/retention-policy.md`
- `docs/directory-ownership.md`

## Completion

Completed 2026-07-11 as part of docs audit remediation (Stage 1 + quick Stage 2).
Front-matter lifecycle enforcement remains on #765.
Pre-existing verify failures (host-linker / bootstrap wasm / gate-648) are out of scope.
