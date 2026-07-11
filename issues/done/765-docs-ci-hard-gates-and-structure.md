---
Status: done
Created: 2026-07-11
Updated: 2026-07-11
ID: 765
Track: docs-audit
Depends on: 761, 762, 763, 764
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: Docs audit 2026-07-11 (§9 CI gates, Stage 2–3)
Blocks: none
---

# 765 — Docs CI hard gates and structural remediation

## Summary

第1段階で誤誘導を止めた後、再発防止の hard gate と構造整理を行う。
数値整合・旧語彙禁止・orphan 禁止・section registry 一致・skip budget・
classification 完全性・overview freshness を CI 化する。

## Acceptance

### CI gates

- [x] Current numeric consistency gate（全生成ビュー一致）
- [x] Deprecated target vocabulary gate（current/reference で旧名 / T0–T5 禁止、allowlist 付き）
- [x] Generated ownership manifest（入力 vs 生成の誤分類禁止）
- [x] Language classification completeness（未分類で generate --check 失敗）
- [x] Current benchmark validity（全 skipped / 旧 commit / deprecated target を current 公開禁止）
- [x] Overview freshness（生成化または archive-only 強制）
- [x] Capability dimensions（単一 `available` bool 禁止）
- [x] Orphan current docs gate
- [x] Section registry parity（README / sidebar / Docsify `rootRoutes`）
- [x] skip-doc-check 構造化 + normative skip budget

### Structure (may land incrementally)

- [x] release-checklist + release-criteria の単一 source（`docs/release/` 入口）
- [x] benchmark governance / runbook 分離（`docs/benchmarks/runbook.md`）
- [x] `target-contract-summary.md` を TOML からの生成のみに固定（手書き正本廃止）
- [x] ownership-map 共通 schema 抽出（`docs/governance/document-ownership.md`）
- [x] 巨大 manual doc の責務分割計画（`docs/plans/docs-manual-split-plan.md`）

## Implementation

- Gate: `scripts/check/gate-765-docs-ci-hard-gates.py`
- Config: `docs/data/docs-gate-config.toml`
- Wired into `python3 scripts/manager.py verify quick`
- `generate-docs.py` fails closed on unclassified `docs/language/*.md`

## Completion

Completed 2026-07-11. Front-matter `lifecycle` / `valid_as_of` mass rollout remains optional follow-up;
gate + section registry cover the hard re-occurrence cases from the audit.
