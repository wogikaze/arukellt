# Archived GC ↔ C ABI bridge design

この文書は、Wasm GC 前提だった時期の GC ↔ C 境界設計メモです。
現在の実装の source of truth ではありません。

## Current source of truth

- [../current-state.md](../current-state.md)
- [../platform/abi.md](../platform/abi.md)

## なぜ archive 化したか

以前の `gc-c-abi-bridge.md` は:

- Wasm GC 参照と C ABI の橋渡し
- capability I/O を前提にした std 内部変換
- pinning / handle table / future FFI 構想

をまとめていました。

現行ブランチでは production path が linear memory ベースであり、
この文書を active guidance として読むと reality とずれます。

## 位置づけ

今後は「将来の FFI 設計や過去の判断理由を振り返るための履歴資料」としてだけ扱ってください。

## いま見るべき文書

- 現在の実装: [../current-state.md](../current-state.md)
- ABI 方針: [../platform/abi.md](../platform/abi.md)
- メモリモデル: [../language/memory-model.md](../language/memory-model.md)
