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

## Layer 2B: Component Model Canonical ABI (v2)

When compiling with `--emit component`, the Component Model canonical ABI applies at
the component boundary. The canonical ABI sits between the GC-native internal
representation and the component interface.

### Type mapping

| Internal (GC) | Canonical ABI | WIT |
|----------------|---------------|-----|
| `i32` (unboxed) | i32 flat | `s32` |
| `i64` (unboxed) | i64 flat | `s64` |
| `f32` (unboxed) | f32 flat | `f32` |
| `f64` (unboxed) | f64 flat | `f64` |
| `i32` (bool) | i32 flat (0/1) | `bool` |
| `(ref (array i8))` | ptr+len in linear mem | `string` |
| `(ref struct{arr,len})` | ptr+len in linear mem | `list<T>` |

### Export convention

- User `pub fn` with WIT-compatible signatures are exported in core Wasm using
  kebab-case names (e.g., `is_even` → `is-even`).
- The component encoder (`wasm-tools component new`) generates canonical ABI
  adapter functions that lift/lower between the core module's calling convention
  and the component interface.
- A WASI adapter module (`wasi_snapshot_preview1.reactor.wasm`) bridges WASI
  preview1 imports to WASI preview2 component model interfaces.

### Import convention

- Host imports declared in WIT files are parsed and registered as `MirImport`
  entries in the MIR module.
- The WIT parser (`wit_parse.rs`) converts WIT interface declarations to MIR
  import entries with typed parameters.

### Linear memory budget

- The core module retains 1 page (64 KB) of linear memory for WASI I/O marshaling.
- Of this, 256 bytes are reserved for the iov buffer (fd_write/fd_read).
- Available budget for canonical ABI string/list flattening: 65,280 bytes.
- Complex type passing (string, list) at component boundaries uses this linear
  memory for temporary serialization during lift/lower operations.
