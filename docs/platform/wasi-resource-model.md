# Archived WASI resource model

この文書は、過去に検討していた capability-first な WASI API 設計メモです。
現在の実装の source of truth ではありません。

## Current source of truth

- [../current-state.md](../current-state.md)
- [../stdlib/io.md](../stdlib/io.md)

## なぜ archive 化したか

以前の `wasi-resource-model.md` は:

- `DirCap` / `RelPath` / `Capabilities` ベースの I/O 設計
- `main(caps: ...)` 前提の API
- WASI p2 / capability model を中心にした将来像

を扱っていました。

しかし現行実装では、利用者向け host API は主に次の `std::host::*` wrapper です。

```ark
std::host::fs::read_to_string(path: String) -> Result<String, String>
std::host::fs::write_string(path: String, content: String) -> Result<(), String>
std::host::clock::monotonic_now() -> i64
std::host::random::random_i32() -> i32
```

そのため、この文書を active guidance として残すと誤読されやすくなっていました。

## 位置づけ

今後は「将来の capability 設計を振り返るための履歴資料」としてのみ扱ってください。

## いま見るべき文書

- 実装の現在地: [../current-state.md](../current-state.md)
- 現行 host API: [../stdlib/io.md](../stdlib/io.md)
- T1/T3 の位置づけ: [wasm-features.md](wasm-features.md)
