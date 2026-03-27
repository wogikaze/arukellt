# 公開 ABI 方針

> **Current-first**: 現在の実装確認は [../current-state.md](../current-state.md) を参照してください。

## 現行 reality

- production path は **linear memory + WASI Preview 1**
- T3 は current branch では experimental fallback であり、ABI reality としては T1 側の制約を引きずる
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
