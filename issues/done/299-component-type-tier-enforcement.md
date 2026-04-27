---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 299
Track: component-model
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 16
---

- 対応済み: "scalar (i32, i64, f32, f64, bool), unit enum, scalar record"
- 未対応: string, list, option, result, tuple, variant with payload, resource
- W0005 warning: non-exportable 関数に対して出力されるが、型レベルの拒否はない
- [x] 対応型 tier が文書化される (Tier 1: "scalar+enum+record, Tier 2: string+list+option+result, Tier 3: resource+stream)"
# Component Model の対応型 tier を宣言し、未対応型 export を compile error にする

## Summary

現在、canonical ABI 未対応の型を export しても W0005 warning 止まり。未対応型で component を生成すると実行時エラーになる。対応範囲を明示し、未対応型は compile error にすべき。

## Current state

- 対応済み: scalar (i32, i64, f32, f64, bool), unit enum, scalar record
- 未対応: string, list, option, result, tuple, variant with payload, resource
- W0005 warning: non-exportable 関数に対して出力されるが、型レベルの拒否はない

## Acceptance

- [x] 対応型 tier が文書化される (Tier 1: scalar+enum+record, Tier 2: string+list+option+result, Tier 3: resource+stream)
- [x] Tier 外の型を export しようとした場合に compile error (E レベル) が出る
- [x] error メッセージに「この型は component export で未対応です」と表示される
- [x] `docs/current-state.md` §V2 の carry-over limitations が更新される

## References

- `crates/ark-wasm/src/emit/t3/cabi_adapters.rs`
- `crates/ark-wasm/src/component/wit.rs`
- `docs/current-state.md`