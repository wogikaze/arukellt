# ADR-033: クロージャ呼び出しを call_ref に移行

ステータス: **ACCEPTED** — `call_indirect` をベースラインとし、`call_ref` へ段階移行する
日付: 2026-06-14
トラック: wasm-feature
Issue: [#069](../../issues/done/069-wasm-typed-func-ref.md)
廃止: なし（issue #019, #025 の GC-native クロージャ記述を精緻化）

---

## 文脈

Arukellt のクロージャと高階関数（HOF）は、関数テーブル付きの Wasm
`call_indirect` に lower されるのがベースラインである。
WebAssembly Typed Function References 提案は `ref.func`、`call_ref`、
`br_on_null`、`br_on_non_null` を追加し、呼び出し先シグネチャがコンパイル時に
分かる場合に table-free な型付きディスパッチを可能にする。

Typed Function References は Wasm 3.0 で Phase 5 shipped
（`typedFunctionReferences`）。主要ランタイム（wasmtime 46、V8 14.6）はデフォルト有効。
Post-MVP 調査は ADR-043 を参照。

## 決定

1. **ベースライン**: 汎用クロージャ / HOF ディスパッチは `call_indirect` を維持する。
   移行が段階的でもユーザー可視の退行を起こさない。
2. **段階移行**: `call_indirect` から `call_ref` へフェーズ分割で移す。
   ベンチマーク結果でゲートする。
3. **本 ADR の範囲外**: `return_call_ref` テールコール（#492）、
   `call_indirect` の全廃、エスケープ解析で table-free が証明される前の Table/Elem 削除。

## 帰結

- `call_ref` 採用は段階的であり、各フェーズはベンチマークゲートで検証する。
- 完全な HOF 移行を主張する前に、出力に `call_ref` バイトがあることを fixture で証明する。
- MIR はベンチマークゲート通過までテーブル経路を残したまま、
  `FnRef` / `call_ref` の lowering フックを追加してよい。

## 参照

- `docs/spec/spec-3.0.0/proposals/function-references/Overview.md`
- `issues/done/025-gc-native-closures.md`
- `issues/done/492-t3-return-call-ref.md`
