# ADR-021: Playground Share URL Format

ステータス: **ACCEPTED** — fragment ベースの share URL（versioned path）
作成日: 2026-05-15
決定日: 2026-05-15
改訂日: 2026-07-11 — wire format 詳細を RFC-001 へ分離
範囲: Playground (web), share/permalink surface

---

## 文脈

ADR-017 は playground v1 の製品契約を定めた。Share/permalink は v1 の明示スコープであり、
**静的ホスティングのみ**で動く必要がある — サーバー側ストレージ、DB、認証はなし。
共有状態はすべて **URL 自体に**符号化しなければならない。

v1 にバックエンドがないため、TypeScript Playground 方式（URL fragment に状態を載せる）を採る。

---

## 決定

1. **共有状態は URL fragment（`#`）に置く**（query ではない）。サーバーへ送られない。
2. **形式は版付きパス**: `#share/<format-version>/<payload>`。
3. **ペイロードは圧縮＋ URL-safe 符号化**し、ラウンドトリップ無損失を契約とする。
4. **前方互換**: 未知フィールドは無視できるスキーマとし、古い URL を壊さない。
5. **妥当な URL 長**を維持する。超過時のフォールバック方針は仕様側で定める。

wire format（JSON キー、DEFLATE、base64url、schema、size limit、error handling）の正本は
[`docs/rfcs/001-playground-share-url-format.md`](../rfcs/001-playground-share-url-format.md)。

---

## 帰結

1. Share/permalink は**任意の静的ホスト**で動く — バックエンド・DB・API キー不要。
2. URL は**プライバシー保護** — fragment は HTTP でサーバーへ送られない。
3. 8,192 文字バジェット内でソース約 **10,000–15,000 文字**を支え、playground 規模の snippet に十分。
4. **前方互換**を内蔵: 任意フィールド追加は形式版を上げずに可能。旧 URL は復号可能。
5. **版ピン留め**で診断コンテキストと将来の互換案内が可能。
6. **ラウンドトリップ契約**と試験ケース（§6.4）が実装作業の受入条件になる。
7. 実装にはブラウザバンドル内の DEFLATE ライブラリ（`pako`、`fflate`、または
   `CompressionStream("deflate-raw")`）が必要。ライブラリ選定は実装判断であり本 ADR では定めない。
8. `docs/adr/README.md` は `python3 scripts/gen/generate-docs.py` で再生成し本エントリを含める。

---


## 関連

- [ADR-017](ADR-017-playground-execution-model.md) — v1 製品契約
- [RFC-001: Share URL format](../rfcs/001-playground-share-url-format.md) — 詳細仕様
- [ADR-022](ADR-022-playground-deployment-and-caching.md) — 静的配信
