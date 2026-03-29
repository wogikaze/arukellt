# Wasm 機能レイヤー

> **Current-first**: 実装の現在地は [`../current-state.md`](../current-state.md) を参照。
> このページは target layering と現在の deployment surface を整理するための要約です。

## 現在の reality

- **T1 `wasm32-wasi-p1`** は non-GC 環境向け compatibility path
- **T3 `wasm32-wasi-p2`** は canonical GC-native path
- T3 のデータ表現は GC-native。linear memory は主に WASI I/O marshaling に残る
- `--emit core-wasm` は T1 / T3 の通常出力
- `--emit component`, `--emit wit`, `--emit all` は `wasm32-wasi-p2` で利用可能
- Component output には外部 `wasm-tools` と WASI adapter module が必要
- backend validation failure (`W0004`) は hard error

## Current target view

| Target | Tier | Current status | Notes |
|-------|------|----------------|-------|
| `wasm32-wasi-p1` | T1 | Implemented | compatibility path for non-GC environments |
| `wasm32-freestanding` | T2 | Not implemented | registry only |
| `wasm32-wasi-p2` | T3 | Implemented | canonical GC-native path |
| `native` | T4 | Not implemented | LLVM scaffold only |
| `wasm32-wasi-p3` | T5 | Future | not started |

## Emit surface

| Emit kind | T1 | T3 | Notes |
|-----------|----|----|-------|
| `core-wasm` | Yes | Yes | default production path |
| `component` | No | Yes | requires external `wasm-tools` + adapter |
| `wit` | No | Yes | WIT export surface generation |
| `all` | No | Yes | emits both core Wasm and component artifacts |

## Alias policy

旧ターゲット alias は受理されるが `W0002` を出します。canonical 名を使ってください。

- `wasm32-wasi` → `wasm32-wasi-p1`
- `wasm-gc` → `wasm32-wasi-p2`
- `wasm-gc-wasi-p2` → `wasm32-wasi-p2`
- `wasm32` → `wasm32-freestanding`

## Current layering guidance

### Layer A: current production / supported paths

- T1 (`wasm32-wasi-p1`) core Wasm
- T3 (`wasm32-wasi-p2`) core Wasm
- T3 component / WIT output for the currently supported export surface

### Layer B: still incomplete / future work

- full canonical ABI coverage for every complex type case
- WASI Preview 3 / async-first runtime work
- native backend completion

## Runtime / capability notes

- Filesystem access is deny-by-default unless `--dir` grants it
- `--deny-fs` is supported
- `--deny-clock` and `--deny-random` are still hard-error placeholders rather than fully enforced capability filtering

## What not to infer from old docs

古い v1-era 文書には次のような historical state が残っていますが、current reality ではありません。

- `--emit component` が hard error のまま
- T3 が linear-memory bridge mode のまま
- Component / WIT が design-only で shipped behavior ではない

判断に迷ったら `docs/current-state.md` を優先してください。

## Runtime model terminology

`BackendPlan` / target planning では次の区別を使います。

| RuntimeModel | Meaning | Current state |
|-------------|---------|---------------|
| `T1LinearP1` | Linear memory + WASI P1 | Active |
| `T3WasmGcP2` | Wasm GC-native runtime on `wasm32-wasi-p2` | Active |
| `T4LlvmScaffold` | LLVM native scaffold | Optional / not implemented |

## 関連

- [../current-state.md](../current-state.md)
- [abi.md](abi.md)
- [abi-reference.md](abi-reference.md)
- [../process/policy.md](../process/policy.md)
- [../migration/t1-to-t3.md](../migration/t1-to-t3.md)
- [../migration/v1-to-v2.md](../migration/v1-to-v2.md)
