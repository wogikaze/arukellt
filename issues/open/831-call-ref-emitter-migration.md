---
Status: open
Created: 2026-07-25
Updated: 2026-07-25
ID: 831
Track: wasm-feature
Depends on: 722
Orchestration class: implementation-ready
Orchestration upstream: 722
Blocks v{N}: none
Priority: 2
Source: ADR-033 Phase A/C GO from #722 — production call_ref emitter
---

# call_ref emitter 移行（typed funcref）

## Summary

#722 の計測で Class A/B の `call_ref` が ≥5% ゲートを超えた。本 issue は本番 emitter を
`call_indirect`（i32 table index）から typed funcref + `ref.func` + `call_ref` へ切り替える。

## Target ABI

| 言語形 | Wasm | 呼び出し |
|--------|------|----------|
| `fn(T) -> R` 引数/変数 | non-null `(ref $sig)` | `call_ref $sig` |
| 関数名を値として渡す | `ref.func $f`（既存 table elem で declare 相当） | — |
| Class C / 未対応 | `call_indirect` + table（残置） | fallback |
| `Option<fn>` | GC enum のまま（ADR-035） | match → funcref → `call_ref`（`br_on_null` は後続） |

## Primary paths

- `src/compiler/corehir/param_shape_value.ark` / `param_shape_local.ark` / `type_ann_param_name.ark`
- `src/compiler/mir/opcodes.ark` / `inst_ref_func.ark` / `inst_call_ref.ark`（新規）
- `src/compiler/mir/lower/call_indirect_emit.ark` / `core_names.ark`
- `src/compiler/wasm/opcodes.ark` / `inst_dispatch_const.ark` / `call_ref.ark`（新規）
- `src/compiler/wasm/sections_types_sigs_detail.ark` / `sections_types_emit.ark` / `code_locals*.ark`
- `tests/fixtures/functions/higher_order.ark`（`call_ref` / `ref.func` 出現を確認）
- `Option<fn>` 最小回帰 fixture（enum 経路）

## Non-goals

- `Option` 全体の `None=null`（ADR-035 禁止）/ `br_on_null` Phase B 実装
- table/elem 全廃、`return_call_ref`（#492）
- Class C の削除

## Acceptance

- [ ] `fn` パラメータの MIR VT が typed funcref（`VT_FUNCREF`）になり、型セクションで `(ref $sig)` になる
- [ ] `MIR_REF_FUNC` が `ref.func` を emit する（i32.const table index ではない）
- [ ] Class A/B の funcref 呼び出しが `MIR_CALL_REF` / `call_ref` になる
- [ ] `higher_order.ark` の出力に `call_ref` と `ref.func` があり、実行結果が従来どおり
- [ ] `Option<fn>` の `Some(g) => g(...)` が enum のまま呼び出し可能（最小 fixture）
- [ ] `python3 scripts/manager.py verify lane --gate t3`
- [ ] `python3 scripts/manager.py verify quick`
- [ ] `python3 scripts/manager.py selfhost fixpoint`
- [ ] `docs/current-state.md` Closures 行を更新

## Related

- ADR-033, ADR-035, ADR-043
- #722（計測完了・本 issue の upstream）
