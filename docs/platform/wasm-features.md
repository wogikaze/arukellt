# Wasm 機能レイヤー

> **Current-first**: 実装の現在地は [`../current-state.md`](../current-state.md) を参照。
> このページは target layering と現在の deployment surface を整理するための要約です。

## 現在の reality

- **T1 `wasm32-wasi-p1`** — WASI Preview 1 向け linear-memory path
- **T3 `wasm32-wasi-p2`** — WASI Preview 2 向け linear-memory path (コンポーネントモデル出力対応)
- **注意**: 全ターゲットでデータ表現は linear-memory（bump allocation）を使用。Wasm GC 命令は未実装
- `--emit core-wasm` は T1 / T3 の通常出力
- `--emit component`, `--emit wit`, `--emit all` は `wasm32-wasi-p2` で利用可能
- Component output には外部 `wasm-tools` と WASI adapter module が必要
- backend validation failure (`W0004`) は hard error

## Current target view

| Target | Tier | Current status | Notes |
|-------|------|----------------|-------|
| `wasm32-wasi-p1` | T1 | Implemented | WASI P1 linear-memory path |
| `wasm32-freestanding` | T2 | Implemented | compile-only, no WASI |
| `wasm32-wasi-p2` | T3 | Implemented | WASI P2 linear-memory path (default) |
| `native` | T4 | Scaffold | compile-only asm stub |
| `wasm32-wasi-p3` | T5 | Not started | target id exists, no backend |

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

## SIMD feature status (ADR-037, #698)

`std::simd` は explicit SIMD library API として実装済み（`stability = "experimental"`）。

| Feature | T1 (wasi-p1) | T2 (freestanding) | T3 (wasi-p2) | T4 (native) | T5 (wasi-p3) |
|---------|-------------|-------------------|-------------|-------------|-------------|
| v128 first-class type | ✅ scalar expansion | ✅ native SIMD | ✅ native SIMD | #699 split | not started |
| `std::simd` lane types | ✅ 11 types | ✅ 11 types | ✅ 11 types | #699 | not started |
| `std::wasm` raw v128 intrinsics | ✅ | ✅ | ✅ | #699 | not started |
| GC struct/array v128 field | n/a (linear) | n/a | fixtures ready, runtime pending | #699 | not started |
| shuffle / swizzle | deferred | deferred | deferred | deferred | deferred |

- **T1**: v128 は4連続i32ローカルとしてスカラー展開（`call_simd_scalar*.ark`）。SIMD命令はemitしないが同一セマンティクス
- **T2/T3**: Wasm SIMD命令を直接emit（`call_simd_native.ark`）。`+simd128` feature flag required
- **T4**: LLVM native SIMD vector types（`<4 x i32>` 等）— #699に分離
- **`std::simd`** に load/store API はなし（`std::wasm` との境界強制）
- **Stability promotion criteria**: ADR-037 §14 — portable API凍結、スカラーfallback同一性、GC lowering準拠、raw API境界確定、conformance/lowering test完備

### Non-goals

- Compiler hint-based autovectorization（`#[vectorize]` 等）— ADR-037 で reject、v5+ evaluation対象
- Relaxed-SIMD proposal — 現時点で対象外

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

## Component composition (`wasm-tools` / `wac`)

Arukellt component 出力は **core wasm → `wasm-tools component embed` → `wasm-tools component new`**（WASI P1 では `wasi_snapshot_preview1.reactor.wasm` adapter）で `.component.wasm` にします。

複数コンポーネントのリンクは **`wac plug`**（`wasm-tools compose` の後継）を使います。

1. **Provider**（例: `math_lib.ark`）を `--emit wasm` で core wasm にコンパイルし、WIT world と `component embed/new` で plug component を生成する。
2. **Consumer socket** は import を宣言した WIT world が必要（#124 完了までは `tests/component-interop/compose/runner-cargo/` の cargo-component ゲストが socket を供給）。
3. `wac plug --plug <provider.component.wasm> <socket.component.wasm> -o composed.component.wasm`
4. `wasmtime run --wasm gc --wasm component-model --invoke 'run()' composed.component.wasm`

スモークテスト: `bash tests/component-interop/compose/run.sh`（`ARUKELLT_TEST_COMPOSE=1` で `verify quick` に含める）。

## 関連

- [../current-state.md](../current-state.md)
- [abi.md](abi.md)
- [abi-reference.md](abi-reference.md)
- [../process/policy.md](../process/policy.md)
- [../migration/t1-to-t3.md](../migration/t1-to-t3.md)
- [../migration/v1-to-v2.md](../migration/v1-to-v2.md)
