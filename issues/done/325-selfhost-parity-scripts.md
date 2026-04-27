---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 325
Track: selfhost-verification
Depends on: 324
Orchestration class: implementation-ready
---
# check-selfhost-parity.sh を作成する
**Blocks v1 exit**: no
**Priority**: 18

## Summary

`docs/compiler/bootstrap.md` が 4 箇所で参照するが存在しない `scripts/check/check-selfhost-parity.sh` を作成する。fixture / cli / diagnostic の 3 モードで Rust 版と selfhost 版の出力を比較する。

## Current state

- `docs/compiler/bootstrap.md:256-258`: `check-selfhost-parity.sh --fixture`, `--cli`, `--diag` を参照
- これら 3 variant とも script が存在しない
- `scripts/run/compare-outputs.sh` は phase 比較ツールであり、parity check ではない
- parity の比較基準が定義されていない

## Acceptance

- [x] `check-selfhost-parity.sh --fixture` が代表 fixture の run 結果 (stdout) を比較する
- [x] `check-selfhost-parity.sh --cli` が compile / check のフラグ挙動を比較する
- [x] `check-selfhost-parity.sh --diag` がエラーメッセージの severity / span / error code を比較する
- [x] 比較基準: run 結果一致 (fixture)、フラグ応答一致 (cli)、severity+span 一致 (diag) — 文言完全一致は求めない
- [x] CI から呼び出し可能

## References

- `docs/compiler/bootstrap.md:256-258` — 存在しない script への参照
- `scripts/run/compare-outputs.sh` — 既存 phase 比較ツール