---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 875
Parent: 852
Track: language-v2
Depends on: "857, 858, 861, 862, 863, 864, 865, 867, 868, 869, 871, 872, 874"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 8–45 parity"
---

# 875 — wasm32-gc / fallback の意味論 parity を証明する

## Scope

RFC-011: **8–45 parity**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] cyclic/shared graph を両 backend で一致させる
- [ ] closure/ADT/collection の結果を一致させる
- [ ] numeric/evaluation/Unicode fixture を一致させる
- [ ] target error を真の capability 差に限定する
- [ ] artifact hash/runtime/tool versions を receipt に保存する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
