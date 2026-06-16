---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 680
Track: docs-audit
Depends on: 679
Orchestration class: audit-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: Target tier honesty audit framework 2026-06-17
Child tracks: 668, 646, 649, 474
---

# 680 — Target tier honesty audit (stable / smoke / scaffold / none)

## Summary

`docs/target-contract.md` は tier 語彙（guaranteed / smoke / scaffold / none）を定義し、
T2/T4/T5 を scaffold または not-started としている。README や Quickstart、
playground、CLI `--help` が **実用 target** のように読める箇所と、fixture /
`run_supported=false` 実態のズレを監査する。

## Audit checklist (section 2)

| チェック | 現状 (2026-06-17) | 起票/追跡 |
|----------|-------------------|-----------|
| stable / smoke / scaffold が混在していない | T3 component = smoke；P2 native docs 分裂 | 本 issue |
| `stable` 表記が smoke tier feature を含まない | README「Canonical target: wasm32-wasi-p2」は component smoke と同居 | 本 issue |
| component-compile が wasm-tools 不在で skip | `target-contract` L158–162 明記 | **#682** |
| T2 scaffold が runtime 例を示していない | `t2_scaffold.ark` compile-only | OK（要 gate） |
| T4 scaffold が run 例を示していない | `native_scaffold.ark` compile-only | OK（要 gate） |
| T5 not-started が CLI/API に露出していない | `#646` open scaffold | **#646** |
| target table generated block ↔ current-state | fixture count 773 vs manifest 1117+ — target-contract 古い | 本 issue |
| `run_supported=false` target に run 例なし | 要 CLI/playground 横断 | **#685** |

## Acceptance

- [ ] `docs/target-contract.md` の P2 native 記述を `current-state` / #074 close evidence と整合
      （「deferred v5+」削除または tier 再分類）
- [ ] README / Quickstart / CLI help に tier ラベル（smoke/scaffold）をユーザー向けに明示
- [ ] `target-contract` fixture count ブロックを manifest から再生成する gate
- [ ] T2/T4/T5 について「compile-only / not runnable」の UX 文言監査完了
- [ ] Gate `scripts/check/gate-680-target-tier-honesty.py`
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `docs/target-contract.md`
- `issues/open/646-t5-wasm32-wasi-p3-target-scaffold.md`
- `issues/open/649-t4-native-full-lowering.md`
- `issues/done/074-wasi-p2-native-component.md`
