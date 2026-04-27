---
Status: open
Created: 2026-03-28
Updated: 2026-03-28
ID: 068
Track: wasm-feature
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: False
Status note: Partially implemented. Remaining items deferred to v5+.
# Wasm Reference Types: table.get / table.set / externref フル対応
---
# Wasm Reference Types: table.get / table.set / externref フル対応


## Summary

WebAssembly Reference Types 提案の `table.get`・`table.set`・`table.fill`・`table.copy`・
`table.grow`・`table.size` 命令と `externref` 型を T3 emitter でフル活用する。
現在は関数テーブルを静的に構築しているが、動的なテーブル操作で
クロージャ・動的ディスパッチ・JavaScript 相互運用が可能になる。

## 受け入れ条件

1. `table.get` / `table.set` を T3 が emit できる
2. `externref` 型の値を関数引数・戻り値・テーブルに格納
3. `std/wasm` に `table_get(idx)` / `table_set(idx, ref)` intrinsic 追加
4. JS 環境向け externref パススルー (Component Model では不要だが wasm-bindgen 相互運用考慮)

## 参照

- `docs/spec/spec-1.0.0/proposals/reference-types/Overview.md`
- `docs/spec/spec-3.0.0/OVERVIEW.md`