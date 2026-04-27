---
Status: done
Track: main
Orchestration class: implementation-ready
Depends on: なし
Version: "v5 (selfhost prep)"
Priority: P0
Reason: Existing checklist mixed "anticipated" dependencies with actual usage and showed critical missing items.
---

# 160: セルフホスト必要 stdlib チェックリストの確認
- Host module APIs used by selfhost compiler: 6
- Prelude exports used by selfhost compiler: 16
- Builtins used by selfhost compiler: 6
- Missing/stub required APIs: 0
# 160: セルフホスト必要 stdlib チェックリストの確認

## Reopened by audit — 2026-04-13


## Resolved — 2026-04-14

This issue was re-verified against actual selfhost compiler usage in `src/compiler/*.ark`.

Authoritative checklist:
- `docs/process/selfhosting-stdlib-checklist.md`

## Verification Result (usage-based)

- Host module APIs used by selfhost compiler: 6
- Prelude exports used by selfhost compiler: 16
- Builtins used by selfhost compiler: 6
- Total required by current `src/compiler/*.ark`: 28
- Missing/stub required APIs: 0

## Gap analysis outcome

No blocking stdlib gaps were found for the current selfhost compiler code path.

## Close gate

- [x] Selfhost stdlib dependency list documented
- [x] Each dependency marked available/missing (all currently required are available)
- [x] Gap analysis recorded
- [x] Issue moved to `issues/done/`