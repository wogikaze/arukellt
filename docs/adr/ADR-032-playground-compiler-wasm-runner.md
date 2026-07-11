# ADR-032: Playground v2 ブラウザ Compile + Run

ステータス: **ACCEPTED** — ブラウザで compile + run（`wasm32-gc` component → jco）
決定日: 2026-05-17
改訂日: 2026-07-11 — ADR-017 から再分離（v1 契約と独立に変更可能にする）
関連 issue: [#632](../../issues/done/632-playground-compiler-wasm-build-run-loop.md)

---

## 文脈

[ADR-017](ADR-017-playground-execution-model.md) は playground **v1**（client-side frontend のみ、
サーバー executor なし）を固定する。v2 はブラウザ内でユーザープログラムを
コンパイル・実行する。v1 出荷判断と v2 技術スタックは独立に進化しうるため、
本 ADR に分離する。

製品経路のターゲット前提は ADR-007 / ADR-013（`wasm32-gc` + jco packaging）。
検証状態は `docs/research/target-runtime-verification.md`
（Node E2E 済み、Chrome jco component E2E は未検証）。

---

## 決定

1. **Playground v2 はブラウザで compile + run する。** TypeScript で言語インタプリタを再実装しない。
2. **Two-stage pipeline:**
   - **Compile**: コンパイラ Wasm を Web Worker で実行（in-memory host）
   - **Run**: `wasm32-gc` component を `jco transpile` し、ESM + JS glue として実行
3. **TypeScript 層**はオーケストレーション、仮想 FS、タイムアウト、stdio、診断、UI のみ。
4. **ホスト関数**は WASI P2/P3 imports（jco が JS glue 化）。旧 `arukellt_io` は使わない。

### Compile stage

```text
bootstrap/arukellt-selfhost.wasm
  -> docs/playground/assets/arukellt-selfhost.wasm

arukellt compile /work/main.ark --target wasm32-gc --emit component -o /work/out.component.wasm
```

（`--emit core-wasm` + component 組み立ても許容。）

Worker host は argv / env / stdio capture / in-memory FS / timeout / size limits を提供する。
ネットワーク・ホスト FS は提供しない。

### Run stage

component → `jco transpile` → ESM + JS glue。WASI P2 imports は jco が変換する。

### Non-goals

- Arukellt の TypeScript インタプリタ / 個別構文の TS 再実装
- ユーザープログラムからの Node/DOM/fetch/FS/network 直接アクセス
- ブラウザ run での wasmtime 直接実行（jco 経由のみ）

---

## 関連

- [ADR-017](ADR-017-playground-execution-model.md) — v1 製品契約
- [ADR-007](ADR-007-targets.md) — `wasm32-gc` + jco
- [ADR-008](ADR-008-component-wrapping.md) — in-tree component
- `docs/research/target-runtime-verification.md`
- `docs/current-state.md`
