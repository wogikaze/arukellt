---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 327
Track: selfhost-verification
Depends on: —
Orchestration class: implementation-ready
---
# Bootstrap docs の重複を解消し truth を一本化する
**Blocks v1 exit**: no
**Priority**: 17

## Summary

bootstrap.md の completion criteria が lines 83-100 と lines 246-316 で重複している。current-state.md に Self-Hosting Bootstrap Status セクションが存在しない (bootstrap.md line 267 がリンクしているのに)。存在しない script への参照が 4 箇所ある。これらを修正して truth を一本化する。

## Current state

- `docs/compiler/bootstrap.md`: 2 箇所に completion criteria table が重複
- `docs/current-state.md`: bootstrap status セクションなし (bootstrap.md からのリンクが 404)
- `docs/compiler/bootstrap.md:256-258`: `check-selfhost-parity.sh` への参照 (script 不在)
- `docs/test-strategy.md`: bootstrap は informational / scaffold 扱い

## Acceptance

- [x] bootstrap.md の completion criteria が 1 箇所に統合される
- [x] current-state.md に bootstrap status セクションが追加される
- [x] 存在しない script への参照が「未作成 (see #325)」として明記される、または #325 完了後に実体化される
- [x] verify-bootstrap.sh の machine-readable output と docs の記述が一致する

## References

- `docs/compiler/bootstrap.md:83-100` — completion criteria (1 箇所目)
- `docs/compiler/bootstrap.md:246-316` — completion criteria (2 箇所目)
- `docs/current-state.md` — bootstrap status セクション不在
- `docs/test-strategy.md` — bootstrap 扱い