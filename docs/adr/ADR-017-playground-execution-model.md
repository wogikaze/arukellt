# ADR-017: Playground v1 Product Contract

ステータス: **ACCEPTED** — client-side hybrid（v1 にサーバー側 executor なし）
作成日: 2026-03-31
決定日: 2026-03-31
改訂日: 2026-07-11 — v2 アーキテクチャを ADR-032 へ再分離
範囲: Playground (web) v1

---

## 文脈

Arukellt の web playground は、実装前に製品契約が必要である。
v1 でサーバー側 executor を出すと運用コスト・悪用面・遅延が増え、
主価値（即時フィードバック）はより軽いクライアント側ツールで得られる。

ブラウザでの **compile + run（v2）** は独立に変更しうるため [ADR-032](ADR-032-playground-compiler-wasm-runner.md) に分離する。
本 ADR は v1 のみを固定する。

Issue 378 は下流作業（379, 428）の前にこの判断を強制するために開かれた。

---

## 決定

### playground frontend component（理想契約）

| 面 | 要件 |
|----|------|
| parse | ソース → AST / 構文診断 |
| format | ソース整形 |
| typecheck | 型検査 |
| diagnostics | 安定した構造化診断 API |

共通要件: **WASI 非依存**、**安定した構造化 API**。
実装言語・バンドル形式は契約ではない（current-state / plan）。

### 実行モデル: client-side hybrid（サーバー executor なし）

| 面 | 実行場所 | v1? |
|----|----------|-----|
| Edit | browser | ✅ |
| Format / Parse / Check / Diagnostics | browser（frontend component） | ✅ |
| Examples | static / browser | ✅ |
| Share / permalink | browser + static host | ✅ |
| Full compile / Run | — | ❌ v2（ADR-032） |

### v1 スコープ

> **v1 = edit + format + parse + check + diagnostics + examples + share**

### v1 非目標

- Wasm バイナリへのフルコンパイル、ユーザープログラム実行
- サーバー側実行サンドボックス、Native 実行、WASI P3 / async
- ブラウザ内 LSP、認証・保存プログラム・ユーザーアカウント

### 一時実装メモ（契約ではない）

現行 v1 frontend は Rust crate の `wasm32-unknown-unknown` バンドル等でありうる。
selfhost frontend へ置換しても、上記理想契約が変わらなければ本 ADR は改訂不要。

---

## 帰結

1. Issue 379 は frontend component 契約向けに進めてよい。
2. freestanding 待ちは不要（ADR-007 廃止）。v2 は ADR-032。
3. share/permalink は静的ホストで足り、実行バックエンドは不要（詳細は ADR-021 / RFC-001）。

---

## 検討した代替案

| 案 | 結果 |
|----|------|
| v1 向けサーバー executor | 却下（運用・悪用・遅延） |
| v2 着地まで v1 を止める | 却下（短期価値を失う） |
| コンパイルのみ v1（emit あり・実行なし） | 却下（効用低・コスト高） |
| 任意のサーバー run 付きハイブリッド | 却下（v2 へ延期） |

---

## docs / tests / examples

- Docs: ADR-007 は読み取り専用参照。v1 は変更しない。
- Tests: frontend API smoke + docs-consistency + 最小 browser smoke。
- Examples: 静的キュレート `.ark`。compile-check クリーン。stdlib から自動生成しない。

---

## 関連

- [ADR-032](ADR-032-playground-compiler-wasm-runner.md) — v2 browser compile/run
- [ADR-021](ADR-021-playground-share-url-format.md) / [RFC-001](../rfcs/001-playground-share-url-format.md)
- [ADR-007](ADR-007-targets.md)、[ADR-013](ADR-013-primary-target.md)
- `docs/current-state.md`
