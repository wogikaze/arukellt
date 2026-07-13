---
Status: done
Created: 2026-07-11
Updated: 2026-07-11
ID: 763
Track: docs-audit
Depends on: 761
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: Docs audit 2026-07-11 (P0-4..P0-8)
Blocks: 675, 678, 765
---

# 763 — Docs P0: multi-axis contracts (component / capability / size / bench / checklist)

## Summary

Component emit・host capability・binary size・benchmark results・release checklist が
契約と実装、または obsolete 値と current 値を同一セルに混ぜている。

## Acceptance

- [x] `current-state.md` から obsolete size（534/918）の canonical 主張を削除し、計測正本へ誘導
- [x] Component: `public_contract` / `implementation_state` / `external_requirements` を分離して記載
- [x] `capability-surface.md` を reachability 多軸表に修正（単一 available をやめる）
- [x] `process/benchmark-results.md` を INVALID / NO MEASUREMENTS と明示（全 skipped / 旧 target）
- [x] `current-state.md` Performance Snapshot が invalid benchmark を current evidence として扱わない
- [x] `release-checklist.md` から formatter 不在の CLOSED 履歴コメントを削除
- [x] Docs-related verify gates for this slice pass; remaining verify failures are pre-existing infra (host-linker / bootstrap wasm / gate-648)

## References

- Docs audit 2026-07-11 §P0-4..P0-8
- `docs/capability-surface.md`
- `docs/process/wasm-size-reduction.md`
- `docs/process/benchmark-results.md`
- issue #675

## Completion

Completed 2026-07-11 as part of docs audit remediation (Stage 1 + quick Stage 2).
