---
Status: done
Created: 2026-03-31
Updated: 2026-06-28
ID: 300
Track: component-model
Depends on: 297, 298
Orchestration class: implementation-ready
---
# multi-export world と他言語ホストの相互運用テストを拡充する
**Blocks v1 exit**: no
**Priority**: 20

## Summary

現在の component interop テストは単一関数 export が中心。複数関数 export world と、wasmtime 以外のホスト (Python wasmtime bindings 等) でのテストがない。

## Acceptance

- [x] 3+ 関数を export する world のテストが追加される
- [x] 異なる型の関数を混在 export するテストが追加される
- [x] wasmtime CLI でのテストが CI に組み込まれる
- [x] (optional) Python wasmtime bindings での呼び出しテスト — deferred; scalar types verified via wasmtime CLI

## Implementation

- Added `tests/component-interop/jco/multi-type-exports/` with 5 exported functions
  across i32, i64, f64, bool types in a single component world
- 9 wasmtime invocations all pass
- Auto-discovered by CI via `scripts/run/verify-harness.sh --component`
- Python bindings test deferred (optional acceptance criterion)