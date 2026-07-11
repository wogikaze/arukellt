# ADR-019: リンクチェックカバレッジポリシー

ステータス: **ACCEPTED** — リンクチェックカバレッジポリシーを採用
作成日: 2026-04-14
改訂日: 2026-07-06 — ポリシーを柱3（リンクチェック）のみに縮小。柱1（アンカー命名規則）・柱2（リダイレクトポリシー）は運用実態に合わないため削除。
範囲: Language documentation (`docs/language/`), all Markdown docs under `docs/`, docs site (`docs/index.html`)
決定日: 2026-04-14

---

## 背景

言語ドキュメントが増えるにつれ、内部リンク（`path.md#anchor`）やファイル参照（`path/to/file.md`）は、ドキュメントの移動・改名・再編成で壊れやすくなる。CI と `verify quick` でドリフトを検出するリンクチェックハーネスが必要である。

本 ADR は**リンクチェックカバレッジ** — ハーネスが何を検査し、何がスコープ外か — を扱う。

> **履歴メモ:** 本 ADR は当初、アンカー命名規則（S1/S2/S3 ティア、明示的 `<a id="">` アンカー）とリダイレクト/エイリアスポリシー（Docsify エイリアス、スタブファイル、`SPLIT_FROM`/`MERGED_FROM` コメント）も規定していた。それらのポリシーは運用上強制されておらず削除された。GFM 自動アンカー規則と明示的 `<a id="">` アンカーは引き続き有効な Markdown/Docsify の挙動だが、本 ADR によってはもはや義務付けられない。

---

## 決定

### 1. 既存カバレッジ: `scripts/check/check-links.sh`

`scripts/check/check-links.sh` にリンクチェックスクリプトが存在する。これは次を行う:

- `docs/` と `issues/` 配下のすべての Markdown ファイルをスキャンする。
- `](path...)` リンク先の相対ファイル参照が既存ファイルに解決されることを検証する。
- **意図的にスキップ**するもの: 純粋なアンカーのみの参照（`#anchor`）と、ファイルパスに付加されたアンカー（`path.md#anchor`） — ファイルの存在のみを検査する。
- 外部 URL（`http://`、`https://`）は**検査しない**。

このスクリプトが **正規のリンクチェッカー**である。特定の作業指示で本スクリプトのスコープが明らかに不十分であることが示されない限り、第2のリンクチェッカーを追加しない。

### 2. アンカーフラグメント検査（実装済み）

アンカーフラグメントの検証は `scripts/check/check-anchor-fragments.py` に実装され、`python3 scripts/manager.py verify quick` に組み込まれている（静的パス、`scripts/check/check-links.sh` の直後）。

チェッカーは次を行う:

- `docs/` と `issues/` 配下の Markdown、および `README.md` と `AGENTS.md` をスキャンする。
- `path.md#anchor` 形式の相対リンクと、同一ファイル内の `#anchor` 参照を検証する。
- GFM 見出しスラッグ規則と明示的 `<a id="">` アンカーでターゲットを解決する。
- 外部 URL（`http://`、`https://`、`mailto:`）と Docsify ルーターパス（`#/...`）をスキップする。
- 既知の例外用に `scripts/check/anchor-allowlist.txt` のオプション allowlist をサポートする。

著者はドキュメント間のアンカーリンクを追加する際もローカルでターゲットを確認すべきである。ハーネスは CI と `verify quick` でドリフトを検出する。

### 3. リンクチェック保証の要約

| 検査 | ツール | ステータス |
|-------|------|--------|
| 内部ファイル参照（例: `path/to/file.md`） | `scripts/check/check-links.sh` | ✅ Covered |
| アンカーフラグメント（例: `file.md#section-id`） | `scripts/check/check-anchor-fragments.py` | ✅ Covered |
| ページ内純粋アンカー（例: `#section-id`） | `scripts/check/check-anchor-fragments.py` | ✅ Covered |
| 外部 URL（`https://...`） | — | ❌ Out of scope |

---

## 結果

- `scripts/check/check-links.sh` は内部ファイル参照を検証する。`scripts/check/check-anchor-fragments.py` はアンカーフラグメントを検証する — 両方とも `verify quick` で実行される。
- ADR-018 の分類バナーは本ポリシーと直交する。両方が独立に適用される。
- ドキュメントの移動/改名/分割/統合に Docsify エイリアス、スタブファイル、`SPLIT_FROM`/`MERGED_FROM` コメントは不要（以前義務付けられていたポリシーは削除済み）。著者は受信リンクを手動で更新すべきである。リンクチェックハーネスが壊れた参照を検出する。

---

## 検討した代替案

**Markdown フロントマターにリダイレクトヘッダーを埋め込む**
却下: ドキュメントサイト（Docsify）はリダイレクト用の YAML フロントマターを処理しない。

**今すぐ `check-links.sh` にアンカーフラグメント検査を追加する**
却下: 見出し抽出とスラッグ重複排除は専用の `check-anchor-fragments.py` スクリプトの方が保守しやすい（issue #644 で実装）。

**別途 `_redirects` ファイルを使う（Netlify 方式）**
却下: プロジェクトは現時点で Netlify や `_redirects` を読むホスティングにデプロイしていない。`docs/index.html` の Docsify エイリアスは自己完結しており、ホスティング固有の機能を要しない。

---

## 参照

- `docs/index.html` — Docsify 設定
- `scripts/check/check-links.sh` — 内部ファイル参照チェッカー
- `scripts/check/check-anchor-fragments.py` — アンカーフラグメントチェッカー（GFM スラッグ + 明示 id）
- ADR-018: Language Docs Classification — Normative / Explanatory / Transitional
- Issue #644: Docs anchor fragment link-check (ADR-019 後続)
- Issue #412: Language Docs: 安定した anchor / permalink 体系を整える
