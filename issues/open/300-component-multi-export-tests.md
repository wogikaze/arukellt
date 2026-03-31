# multi-export world と他言語ホストの相互運用テストを拡充する

**Status**: open
**Created**: 2026-03-31
**ID**: 300
**Depends on**: —
**Track**: main
**Priority**: 20

## Summary

現在の component interop テストは単一関数 export が中心。複数関数 export world と、wasmtime 以外のホスト (Python wasmtime bindings 等) でのテストがない。

## Current state

- `tests/component-interop/jco/`: calculator, bool-logic, primitives-float, record-point, enum-colors
- すべて wasmtime CLI による検証
- 複数関数を同一 world で export するテストがない

## Acceptance

- [ ] 3+ 関数を export する world のテストが追加される
- [ ] 異なる型の関数を混在 export するテストが追加される
- [ ] wasmtime CLI でのテストが CI に組み込まれる
- [ ] (optional) Python wasmtime bindings での呼び出しテスト

## References

- `tests/component-interop/`
- `crates/ark-wasm/src/component/wrap.rs`
