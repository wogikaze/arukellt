# ADR 台帳の規則追従 — research/plans 分離と supersession 整合

Status: open
Priority: P1
Created: 2026-07-11

## Summary

ADR-000 の規則に既存 ADR 群を追従させる残作業を追跡する。
猶予は **2026年7月末**（ADR-000 帰結）。恒久的な自己正当化には使わない。

## DONE_WHEN

1. [x] 調査・epic・移行フェーズが ADR 本文に残っていない（research / plans / RFC へ移済み）
2. [ ] 現役 ADR の判断文脈で T1–T5 を使っていない（歴史括弧のみ可）
3. [x] `docs/current-state.md` のターゲット表記が ADR-007/013 canonical 名と一致
4. [x] ADR-002 等の生きたランタイム版・未実装スナップショットが current-state 側へ整理済み
5. [ ] `python3 scripts/check/check-adrs.py` と `python3 scripts/manager.py verify quick` が緑

## Notes

2026-07-11 着手済み: ADR-043→research、035/040/042→plans、ADR-004/005 supersession、
ADR-006 Layer3 弱体化、ADR-007/013 一本化、check-adrs 強化。

2026-07-11 追記2: WASI P1 environ_get 誤認訂正、Weak/finalizer を ADR-002/043 で未採択に統一、
jco GC ブロッカー記述を調査結果に合わせて更新、ADR-008 CM 安定化表現を訂正、
ADR-006 raw GC layout を non-stable に縮小。

2026-07-11 追記3: ADR-007 を決定のみへスリム化（現行実行面は
`docs/platform/target-runtime-and-surfaces.md`）。native 出力表の矛盾を解消。
ADR-042 に intrinsic negative examples。ADR-006 層番号廃止。ADR-002 計測を research へ。
ADR-017 を理想 frontend 契約へ。形式残件（004 ファイル名、決定日、T3 表現、035 タイトル）。

2026-07-11 追記4: current-state / project-state を ADR と同期（freestanding・component・jco）。
research で Chrome jco E2E を未検証と明記。ADR-008 scratch→plan、ADR-035 を layout 方針へ縮小。
check-docs に current-state ADR sync 検査を追加。

2026-07-11 追記5: ADR-021/040 を短い ADR + RFC（001/002）へ分割。
ADR-017 を v1 のみに縮小し、v2 を ADR-032（ACCEPTED）へ再分離。
