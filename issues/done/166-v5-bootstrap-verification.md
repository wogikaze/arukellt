---
Status: done
Updated: 2026-03-30
ID: 166
Track: main
Depends on: 181, 182
Orchestration class: implementation-ready
---
# v5 Bootstrap verification epic
**Blocks v1 exit**: no

**Status note**: Parent issue for bootstrap verification. End-user documentation is tracked separately in #169.

## Summary

ブートストラップ検証は、fixpoint を確認する比較スクリプトと、verify-harness / CI への接続を分けて扱う必要がある。手順書は検証コードとは別の成果物なので #169 に分離する。

## Acceptance

- [x] #181, #182 が完了している
- [x] bootstrap verification の実装責務と docs 責務が分離されている
- [x] selfhost fixpoint 検証が issue queue 上で追跡できる

## References

- `issues/open/165-v5-phase3-wasm-emitter.md`
- `issues/open/169-v5-bootstrap-doc.md`
- `scripts/run/verify-harness.sh`