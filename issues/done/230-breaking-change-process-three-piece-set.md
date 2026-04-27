---
Status: done
Created: 2026-03-30
Updated: 2026-03-30
ID: 230
Track: main
Depends on: 226
Orchestration class: implementation-ready
---
# 破壊的変更の issue/changelog/migration guide 3点セット手順を確立する
**Blocks v1 exit**: yes

## Summary

現状、破壊的変更がどのような手順で導入されるかが定められていない。
issue も changelog も migration guide も揃っていない変更が入ると、利用者コードは事前通知なく壊れる。
この issue では、破壊的変更に必須の 3 点セット（issue / changelog / migration guide）の手順を確立する。

## Acceptance

- [x] 破壊的変更の定義が文書化されている（何が「破壊的」とみなされるか）
- [x] 破壊的変更を導入するための必須手順（issue 起票 → changelog → migration guide）が ADR として定められている
- [x] 既存の変更履歴から破壊的変更に相当するものが遡及して changelog に記録されている
- [x] CHANGELOG.md または同等のファイルが存在し、バージョン別に管理されている

## Scope

### 破壊的変更の定義

- 「破壊的変更」とみなされるケースの列挙（構文変更・API 削除・動作変更・ABI 変更 など）
- 「非破壊的変更」との境界の明確化

### 手順の ADR 化

- 破壊的変更の導入フロー（告知 → 非推奨期間 → 削除）の定義
- issue テンプレート・changelog エントリ・migration guide テンプレートの作成

### 遡及記録

- 過去の主要な破壊的変更を CHANGELOG.md に記録
- migration guide が必要な変更の洗い出し

## References

- `docs/adr/`
- `CHANGELOG.md` (存在する場合)
- `issues/open/226-language-spec-stability-labels.md`

## Completion Note

Closed 2026-04-09. ADR-016 written defining breaking change three-piece set rule (issue + changelog + migration guide), deprecation periods, and enforcement.