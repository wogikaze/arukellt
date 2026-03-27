# 公開 ABI 方針

> **Current-first**: 現在の実装確認は [../current-state.md](../current-state.md) を参照してください。

## 現行 reality

- T3 (`wasm32-wasi-p2`) は canonical v1 path で WasmGC types + linear-memory bridge を使用
- T1 (`wasm32-wasi-p1`) は non-GC 環境向けの compatibility path
- Component Model は current deployment ABI ではない
- backend validation (`W0004`) に通らない Wasm は build failure

## Backend boundary の current reading

### Layer 1 — frontend / MIR artifacts

- 利用者向け互換性の対象ではない
- diagnostics / dumps / baselines の比較対象

### Layer 2 — shipped backend contract

- core Wasm artifact
- T1 runtime assumptions
- no stable component ABI surface yet

### Layer 3 — future design space

- real T3 GC/component ABI
- native / C ABI completion
- wider host interop surface

## ABI guidance

現時点でこの文書を読む価値が高いのは次です。

- current shipped ABI surface は T1 側
- T3 docs を current shipped contract と混同しない
- backend planning 境界は frontend semantics を変えない

## 関連

- [../current-state.md](../current-state.md)
- [wasm-features.md](wasm-features.md)
- [../language/memory-model.md](../language/memory-model.md)

## T3 aggregate layout (bridge mode)

T3 uses linear memory for struct and enum values with the following layouts:

### Struct layout

Fields are stored sequentially. Element sizes: 4 bytes (i32, bool, String ptr, Vec ptr),
8 bytes (i64, f64).

```text
[field_0][field_1]...[field_N]
```

A struct pointer (i32) points to the start of the field area.
Field access computes byte offset from field index and type sizes.

### Enum layout

```text
[tag: 4 bytes][payload_field_0][payload_field_1]...
```

- Tag is an i32 discriminant (0, 1, 2, … per variant order).
- Payload area is sized for the variant's fields (type-aware: 8 bytes for i64/f64).
- `Option<T>` and `Result<T, E>` follow the same tag+payload model.

### Match lowering

Match expressions are lowered to MIR `Switch` terminators that branch on the enum tag.
Each arm extracts payload fields via `EnumPayload` with byte-offset computation.
Struct patterns use `FieldAccess` for each matched field.
