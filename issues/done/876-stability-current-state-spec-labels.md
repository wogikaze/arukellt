---
Status: done
Created: 2026-09-19
Updated: 2026-09-23
ID: 876
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 42, 43"
---

# 876 — stability / current-state / redesign の分類を整合する

## Scope

RFC-011: **42, 43**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] stable/provisional/experimental/unimplemented を一意に定義する
- [ ] current-state と future proposal を分離する
- [ ] ADR-014/ADR-018 と spec labels を同期する
- [ ] 未実装 proposal を stable 表示しない
- [ ] docs consistency で drift を検出する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する

## Completion receipt — 2026-09-23

Implementation commit: `404ab8252`

- Acceptance mapping with repo evidence:

  | Acceptance | Evidence |
  |---|---|
  | 4 ラベルを一意に定義 | ADR-014 (ACCEPTED) が正本。maturity-matrix.md の Stability Labels 表は ADR-014 定義文に揃え canonical link を明記、classifications TOML ヘッダも ADR-014 を正本として指す (`404ab8252`) |
  | current-state と future proposal の分離 | `docs/current-state.md` は "Accepted ADR contract gaps" と "Proposed migration gaps（normative ではない）" を分離し、v2 項目は "Language v2 semantic milestones" として issue/ADR 単位に区切って現状を記述（構造は既存。本 issue では分離が保たれていることを確認） |
  | ADR-014/ADR-018 と spec labels の同期 | spec.md の 2.9 / 6.4 / 6.5 に provisional marker を追加し TOML と一致。spec 再番号付けで取り残されていた TOML 2.8 を Let-Generalization (TypeScheme) に修正し 2.9 Traits and Impl Blocks (v1) を新設。maturity-matrix.md を再生成 (`404ab8252`) |
  | 未実装 proposal を stable 表示しない | matrix の unimplemented 行は 0 で stable 表記なし。current-state.md は wasi-p3 / SIMD 等を unimplemented / experimental と正直に記述。以降の marker / TOML drift は新規チェックが検出する |
  | docs consistency で drift 検出 | check_spec_stability_marker_sync() を check-docs-consistency.py の main() に配線。label drift・name drift（節番号の振り直しで別機能へラベルが付く事故）・spec 節欠落・marker 行欠落の 4 種を検出 |

- Positive / negative evidence: `scripts/tests/test_spec_stability_sync.py`
  (13 tests) — 見出し parse、祖先への label 継承、name 正規化、
  synced tree PASS、label/name/欠落の各 drift を rc=1 で検出。
  手動 mutation でも TOML stable 反転と spec marker 除去の両方向で
  検出を確認済み。
- Verification receipts:

  ```text
  python3 scripts/tests/test_spec_stability_sync.py   PASS (13 tests)
  python3 scripts/check/check-docs-consistency.py     docs consistency OK (0 issues)
  python3 scripts/gen/generate-docs.py                generated docs are up to date
  python3 scripts/manager.py verify lane              PASS
  ```

- Normative references: ADR-014 (label 定義の正本), ADR-018 (docs 分類),
  spec.md markers, language-doc-classifications.toml features,
  generated maturity-matrix.md。
