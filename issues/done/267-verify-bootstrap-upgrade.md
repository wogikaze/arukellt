---
Status: completed
Created: 2026-04-19
ID: 267
Track: main
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: —
---

# verify-bootstrap.sh の昇格
**Blocks v3**: yes

## Summary

`scripts/run/verify-bootstrap.sh` は Stage 0 で各 selfhost source を個別に compile し、`main.wasm` が生成されない場合は Stage 1/2 を skip する構造であり、fixpoint が継続検証されていない。このissueでは skip 条件を除去し、全 stage を逐次実行・失敗時に詳細ログを出力する構造に改修する。

## Why this matters

* `verify-bootstrap.sh` は skip 前提の scaffold ではなく達成判定の本体になる必要がある
* fixpoint が継続検証されていない
* 失敗時に詳細ログが出力される必要がある

## Acceptance

* [x] `scripts/run/verify-bootstrap.sh` から skip 条件が除去されている
* [x] 全 stage (Stage 0/1/2) が逐次実行される
* [x] 失敗時に詳細ログが出力される
* [x] Stage 2 fixpoint 達成時に exit 0 で終了する
* [x] Stage 2 fixpoint 未達成時に exit non-zero で終了する

## Scope

### skip 条件の除去

* `main.wasm` が生成されない場合でも Stage 1/2 を実行する
* 各 stage の失敗を明確に検出する

### 詳細ログの出力

* 各 stage の実行ログを出力
* 失敗時にエラー原因を特定できるログを出力

### fixpoint 検証

* Stage 1 と Stage 2 の出力を比較して fixpoint を検証
* fixpoint 未達成時に明確なエラーメッセージを出力

## References

* `scripts/run/verify-bootstrap.sh`
* `docs/compiler/bootstrap.md`
* `issues/open/253-selfhost-completion-criteria.md`
* `issues/open/268-selfhost-parity-ci-verification.md`
