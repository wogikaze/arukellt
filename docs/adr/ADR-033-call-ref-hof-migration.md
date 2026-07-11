# ADR-033: クロージャ呼び出しを call_ref に移行

ステータス: **ACCEPTED** — 段階移行。table-free パターンが揃うまで `call_indirect` をベースラインとする
日付: 2026-06-14
トラック: wasm-feature
Issue: [#069](../../issues/done/069-wasm-typed-func-ref.md)
廃止: なし（issue #019, #025 の GC-native クロージャ記述を精緻化）

---

## 文脈

Arukellt のクロージャと高階関数（HOF）は現在、関数テーブル付きの Wasm
`call_indirect` に lower される（`docs/current-state.md` の Closures 行）。
WebAssembly Typed Function References 提案は `ref.func`、`call_ref`、
`br_on_null`、`br_on_non_null` を追加し、呼び出し先シグネチャがコンパイル時に
分かる場合に table-free な型付きディスパッチを可能にする。

> **2026-07 更新**: Typed Function References は Wasm 3.0 で Phase 5 shipped
> （`typedFunctionReferences`）。wasmtime 46 と V8 14.6（Chrome 146 /
> Node.js 26）はデフォルト有効。Post-MVP 調査は ADR-043 を参照。

歴史的 issue（#019, #025, #024）は GC-native の `call_ref` パスを計画していたが、
selfhost emitter は現行の T3 パスで汎用 HOF ディスパッチに依然 `call_indirect`
を使う。ギャップ解消は issue #069、詳細フェーズ計画（emitter 監査、nullable refs、
ベンチマークゲート）は issue #722 で追跡する。

## 決定

1. **ベースライン（現在）**: 汎用クロージャ / HOF ディスパッチは `call_indirect` を維持する。
   移行が段階的でもユーザー可視の退行を起こさない。
2. **段階移行**: `call_indirect` から `call_ref` へフェーズ分割で移す。
   ベンチマーク結果でゲートする。詳細計画は **issue #722**。
3. **本 ADR の範囲外**: `return_call_ref` テールコール（#492）、v5 前の
   `call_indirect` 全廃、エスケープ解析で table-free が証明される前の Table/Elem 削除。

## 帰結

- `docs/current-state.md` の Closures 行は、現行デフォルトが `call_indirect`、
  `call_ref` 採用が本 ADR に沿った段階移行であることを明記する。
- 新しい emitter 作業は、完全な HOF 移行を主張する前に、出力に `call_ref`
  バイトがあることを fixture で証明してから入れる。
- MIR は Phase C のベンチマークゲート通過までテーブル経路を残したまま、
  `FnRef` / `call_ref` の lowering フックを追加してよい。

## 参照

- `docs/spec/spec-3.0.0/proposals/function-references/Overview.md`
- `issues/done/025-gc-native-closures.md`
- `issues/done/492-t3-return-call-ref.md`
