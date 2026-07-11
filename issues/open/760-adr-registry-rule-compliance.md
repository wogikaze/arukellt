# ADR 台帳の規則追従 — research/plans 分離と supersession 整合

Status: open
Priority: P1
Created: 2026-07-11

## Summary

ADR-000 の規則に既存 ADR 群を追従させる残作業を追跡する。
猶予は **2026年7月末**（ADR-000 帰結）。恒久的な自己正当化には使わない。

## DONE_WHEN

1. [x] 調査・epic・移行フェーズが ADR 本文に残っていない（research / plans / RFC へ移済み）
2. [x] 現役 ADR の判断文脈で T1–T5 を使っていない（歴史括弧のみ可）
3. [x] `docs/current-state.md` のターゲット表記が ADR-007/013 canonical 名と一致
4. [x] ADR-002 等の生きたランタイム版・未実装スナップショットが current-state 側へ整理済み
5. [ ] `python3 scripts/check/check-adrs.py` と `python3 scripts/manager.py verify quick` が緑

## Notes

2026-07-11: ADR修復 → normative伝播 → 提案ADRの前提修正（038/039）・037 capabilities・
current-state分割（history/）・鮮度メタ・リンク修正。

2026-07-11 (再評価対応): ADR-036 を ADR-014 準拠の削除方針へ修正。From/Into 直接
impl 禁止・TryFrom<T,E>・RFC-004。ADR-037 TypeKind。RFC-003 拡充。host_profiles
SSOT・ADR-042 core-ops.toml・primary 役割定義。

