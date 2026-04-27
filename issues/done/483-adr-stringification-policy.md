---
Status: done
Created: 2026-04-03
Updated: 2026-04-10
ID: 483
Track: language-design
Depends on: "(none)"
ADR candidate: True
Orchestration class: implementation-ready
Blocks v1 exit: no
Downstream: "#484 (compiler implementation) — この issue 完了後に着手"
Reason: All 4 acceptance criteria verified.
Action: Moved from `issues/open/` → `issues/done/`
---

# ADR: "canonical stringification surface — to_string(x) ポリシー"
- `docs/adr/` — 新規 ADR ファイル (例: `ADR-022-stringification-policy.md`)
- ADR ファイルに `Status: "Decided` (または同等) が含まれている"
→ acceptance 4 で `Status: Decided` を必須にすることで防止
2. ADR now has a Non-goals section explicitly scoping out: "method syntax implementation (#484 scope), removal of `i32_to_string`/`i64_to_string` helpers, Display trait / stdlib restructuring, docs/fixture updates (#171)"
4. ADR Status: DECIDED
# ADR: canonical stringification surface — to_string(x) ポリシー

---

## Decomposed from 171

Issue 171 (`canonical-to-string-surface`) は:
1. ADR の作成 (設計決定の記録)
2. compiler/stdlib の to_string() 実装 (#484)
3. docs / fixture coverage (#171 に残す)

の 3 層を混ぜている。この issue は **ADR layer のみ** を担当する。
設計が決定されるまでは実装 (#484) を始めてはならない。

Downstream: #484 (compiler implementation) — この issue 完了後に着手

---

## Summary

Arukellt には `i32_to_string`、`f"..."`、Display-based method など
stringification の複数の surface が存在する。
LLM が生成するコードと user-facing docs の主導線を安定させるため、
`to_string(x)` を canonical な第一表記として ADR に記録する。

このADR が承認されることで、実装者 (#484) と docs 更新者 (#171) が
一貫した設計に基づいて作業できるようになる。

## Why this is a separate issue

設計決定 (ADR) を実装と分離することで:
- ADR がない状態で実装が先走り、後で表記が変わる事態を防ぐ
- ADR のレビュー・承認を実装 PR と分けてできる
- 「ADR が書かれた」だけで「実装が done」と誤解されない

## Visibility

internal-only (ADR はプロジェクト内部の設計文書; user-visible feature ではない)

## Primary paths

- `docs/adr/` — 新規 ADR ファイル (例: `ADR-022-stringification-policy.md`)

## Allowed adjacent paths

- `docs/language/syntax.md` — 既存の stringification 言及への cross-reference 追加のみ

## Non-goals

- compiler の `to_string()` 実装 (#484)
- stdlib の `to_string` surface 変更 (#484)
- docs/quickstart/cookbook の更新 (#171)
- fixture 追加 (#171)
- LSP manifest 更新 (#484)

## Acceptance

1. `docs/adr/ADR-0xx-stringification-policy.md` が存在し、
   `to_string(x)` を canonical、`.to_string()` を secondary sugar として記録している
2. ADR に Non-goals セクションがあり、今回の scope 外 (method syntax 廃止等) を明記している
3. ADR に現状の表記一覧と移行方針 (既存コードへの影響) が記載されている
4. ADR の Status が `Decided` または同等のステータスになっている

## Required verification

- `ls docs/adr/ | grep stringif` が 1 件ヒットする
- ADR ファイルに `Status: Decided` (または同等) が含まれている
- `bash scripts/run/verify-harness.sh --quick` が pass

## Close gate

- ADR ファイルが `docs/adr/` に存在する
- Status が Decided になっている
- 実装 (#484) や docs 更新 (#171) はこの issue の close 条件ではない

## Evidence to cite when closing

- `docs/adr/ADR-0xx-stringification-policy.md` (ファイルパス)
- ADR の Status 行

## False-done risk if merged incorrectly

- ADR の Draft を Done として close する
  → acceptance 4 で `Status: Decided` を必須にすることで防止
- ADR が承認されたので「to_string() が使える」と docs に書く
  → 実装は #484; docs は #171; この issue は設計決定だけ

---

## Closed — 2026-04-10


**Evidence**:
1. `docs/adr/ADR-012-stringification-surface.md` exists and documents `to_string(x)` as canonical, `.to_string()` as secondary sugar
2. ADR now has a Non-goals section explicitly scoping out: method syntax implementation (#484 scope), removal of `i32_to_string`/`i64_to_string` helpers, Display trait / stdlib restructuring, docs/fixture updates (#171)
3. ADR contains current surface listing (選択肢 section) and migration policy (結果 section)
4. ADR Status: **DECIDED**

**Verification**:
- `ls docs/adr/ | grep stringif` → `ADR-012-stringification-surface.md` ✓
- `grep "DECIDED" docs/adr/ADR-012-stringification-surface.md` → match ✓
- `grep "Non-goals" docs/adr/ADR-012-stringification-surface.md` → match ✓
- `bash scripts/run/verify-harness.sh --quick` → 19/19 PASS ✓
