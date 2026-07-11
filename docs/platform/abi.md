# 公開 ABI 方針

> **Current-first**: 現在の実装確認は [../current-state.md](../current-state.md) を参照してください。
> カテゴリ定義の正本は [ADR-006](../adr/ADR-006-abi-policy.md)。

## 現行 reality

- **primary** `wasm32-gc`: Wasm GC 表現へ移行中 + 既定 host WASI P2。component emit あり（ADR-008）
- **supported** `wasm32`: 同一言語意味論の linear-memory lowering（AtCoder 等）
- **stable 公開境界**: WIT / Canonical ABI（ADR-006）。raw GC layout は compiler-private
- backend validation (`W0004`) に通らない Wasm は build failure

## ABI カテゴリ（ADR-006）

| カテゴリ | 役割 |
|----------|------|
| Compiler-private | GC layout、呼び出し規約など。互換保証なし |
| Stable interoperability | WIT / Canonical ABI |
| Experimental raw Wasm | スカラー / opaque handle 等に限定 |
| Reserved native | 未決定（ADR-045） |

## ABI guidance

- shipped の **言語 primary** は `wasm32-gc`（ADR-013）
- `wasm32` を「唯一の shipped ABI」と混同しない
- component / WIT は primary 経路の公開相互運用面（完成度は current-state）
- frontend semantics を backend planning で変えない

## 関連

- [../current-state.md](../current-state.md)
- [ADR-006](../adr/ADR-006-abi-policy.md) / [ADR-007](../adr/ADR-007-targets.md)
- [../language/memory-model.md](../language/memory-model.md)
- [../plans/component-canonical-memory.md](../plans/component-canonical-memory.md)

## Aggregate layout notes（実装メモ・非契約）

`wasm32`（および移行途中の箇所）では struct/enum が linear memory 上に置かれることがある:

### Struct layout

Fields are stored sequentially. Element sizes: 4 bytes (i32, bool, String ptr, Vec ptr),
8 bytes (i64, f64).

```text
[field_0][field_1]...[field_N]
```

### Enum layout

```text
[tag: 4 bytes][payload_field_0][payload_field_1]...
```

- Tag is an i32 discriminant
- `Option<T>` / `Result<T, E>` follow tag+payload

GC 側の既定方針は [ADR-035](../adr/ADR-035-wasm-gc-implementation.md)。

## Component Model Canonical ABI

`--emit component` 時、component 境界では Canonical ABI が適用される。
詳細・scratch 現行値は plans / current-state。
