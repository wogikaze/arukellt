---
Status: open
Created: 2026-03-28
Updated: 2026-03-28
ID: 069
Track: wasm-feature
Depends on: —
Orchestration class: implementation-ready
---
# Wasm Typed Function References: ref.func / call_ref フル活用
**Blocks v4 exit**: no

**Status note**: Wasm proposal — deferred to v5+. Not implemented.

## Summary

WebAssembly Typed Function References 提案 (`docs/spec/spec-3.0.0/proposals/function-references/Overview.md`) の
`ref.func`・`call_ref`・`br_on_null`・`br_on_non_null` を Arukellt のクロージャ実装に完全活用する。
現在のクロージャ実装がどの程度 `call_ref` を使っているかを確認し、
テーブル不要のダイレクト関数参照パターンをすべて `call_ref` に統一する。

## 受け入れ条件

1. 型付きクロージャが `call_ref` で呼び出される (テーブル経由の `call_indirect` を排除)
2. `ref.func` で関数参照を作成する全パターンを確認
3. `br_on_null` / `br_on_non_null` を nullable 参照のガード処理に使用
4. クロージャ呼び出しのベンチマークで `call_indirect` 比 5% 以上高速化確認

## 参照

- `docs/spec/spec-3.0.0/proposals/function-references/Overview.md`