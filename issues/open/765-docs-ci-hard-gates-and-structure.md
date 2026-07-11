---
Status: open
Created: 2026-07-11
Updated: 2026-07-11
ID: 765
Track: docs-audit
Depends on: 761, 762, 763, 764
Orchestration class: design-ready
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

- [ ] Current numeric consistency gate（全生成ビュー一致）
- [ ] Deprecated target vocabulary gate（current/reference で旧名 / T0–T5 禁止、allowlist 付き）
- [ ] Generated ownership manifest（入力 vs 生成の誤分類禁止）
- [ ] Language classification completeness（未分類で generate --check 失敗）
- [ ] Current benchmark validity（全 skipped / 旧 commit / deprecated target を current 公開禁止）
- [ ] Overview freshness（生成化または archive-only 強制）
- [ ] Capability dimensions（単一 `available` bool 禁止）
- [ ] Orphan current docs gate
- [ ] Section registry parity（README / sidebar / Docsify `rootRoutes`）
- [ ] skip-doc-check 構造化 + normative skip budget

### Structure (may land incrementally)

- [ ] release-checklist + release-criteria の単一 source
- [ ] benchmark governance / runbook 分離
- [ ] `target-contract-summary.md` を TOML からの生成のみに固定（手書き正本廃止）
- [ ] ownership-map 共通 schema 抽出
- [ ] 巨大 manual doc の責務分割計画（spec / cookbook / audits）

## References

- Docs audit 2026-07-11 §9–§10
- issues #678, #680, #681, #683
