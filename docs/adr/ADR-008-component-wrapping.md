# ADR-008: Component Model ラッピング戦略

ステータス: **ACCEPTED** — `--emit component` は in-tree で生成する

決定日: 2026-03-28  
改訂日: 2026-07-11 — 1 ページ scratch 上限を plan へ移す

---

## 文脈

Component Model 対応にあたり、core Wasm モジュールを `.component.wasm` に変換する
方法を決める必要がある。

選択肢:
1. **外部ツール (`wasm-tools component new`)** — subprocess 委譲
2. **ツリー内実装** — component binary を自前生成

---

## 決定

1. **`--emit component` / `--emit all` の component 生成は in-tree 実装とする**
   （`src/compiler/component/`）。`wasm-tools component new` への恒久依存は置かない。
2. **`wasm-tools` の役割は補助に限定する**: `component wit` / `component validate` 等の
   検査・診断、および開発用ブリッジ補助。これらが無いことは `--emit component` の
   失敗理由にしない。
3. **複数 component の合成**は ADR-034（`wac plug` 委譲）の範囲であり、本 ADR の
   単体 wrapping とは分離する。
4. **ブラウザ向け ESM 化**（`jco transpile`）はコンパイラ外のパッケージング手順である。
5. **canonical ABI 用の一時領域と再確保契約は in-tree 実装が管理する。**
   具体的なページ数・offset・bump 戦略は実装詳細であり、本 ADR では固定しない
   （現行値は [`docs/plans/component-canonical-memory.md`](../plans/component-canonical-memory.md)）。

理由:
- Arukellt はセルフホスト言語であり、component 生成を外部ツールに鎖しない
- WASI 0.2 で利用する Component Model の機能集合は producer/consumer 間で安定運用されている
  （ただし Component Model **全体**は現在も標準化中であり、binary format や gated feature の
  変化を追跡する必要がある）
- 必要な subset を限定し、conformance tests と binary encoder を内部管理できる
- ビルド再現性と CI の自己完結性を優先する

現行の補助スクリプトや fixture の挙動は `docs/current-state.md` を参照する。

---

## 代替案

### 外部 `wasm-tools` 委譲（却下）

- 利点: 仕様追従を上流に任せられる
- 欠点: PATH / 版ピン / セルフホスト非完結。`--emit component` が外部依存になる

---

## 関連

- [ADR-007](ADR-007-targets.md) — emit surface（component は in-tree）
- [ADR-034](ADR-034-component-composition-linking.md) — 合成は `wac plug`
- [`docs/plans/component-canonical-memory.md`](../plans/component-canonical-memory.md) — scratch 現行値
- `docs/current-state.md` — 実装・fixture の現行挙動
