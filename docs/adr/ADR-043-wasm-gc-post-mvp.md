# ADR-043: 未標準化の Wasm GC 拡張を言語仕様の前提にしない

ステータス: **ACCEPTED** — Phase 0 の Wasm GC 拡張に言語仕様を依存させない

決定日: 2026-04-15  
改訂日: 2026-07-24 — issue #723 による exnref 評価を反映

旧番号メモ: 調査草稿は誤って ADR-008 として公開されたのち ADR-043 へ移した。
調査本文の正本は [`docs/research/wasm-gc-post-mvp.md`](../research/wasm-gc-post-mvp.md)。

---

## 文脈

Wasm GC MVP は Wasm 3.0 に含まれ主要ランタイムで利用可能である。一方、
Static Fields / Weak References / Type Parameters など Post-MVP 項目の一部は
W3C 上まだ Phase 0（個別 proposal すら無い）である。これらを言語意味論や
stdlib 契約の前提にすると、仕様が外部の未成熟提案に鎖される。

---

## 決定

1. **Phase 0 の Wasm GC 拡張には依存しない。** 言語仕様・stdlib・コンパイラ契約の
   前提にしない。特に `Weak<T>` / finalizer は未採択（観測可能意味論が未定義のため。
   循環参照は MVP GC で扱えるため ADR-002 で許可）。
2. **Wasm 3.0 に shipped 済みの機能**（例: Final Types、Typed Function References、
   `exnref`）は「Post-MVP 調査」ではなく通常の Wasm 機能として、必要なら別 ADR /
   issue で評価する（例: ADR-033）。
3. **再評価条件**: 対象提案が Phase 3 以上に進み、主要ランタイムで利用可能になり、
   かつ具体的な言語ニーズ（issue / RFC）が起票されたとき。Weak/finalizer を採択する
   場合は、無効化タイミング・upgrade 原子性・実行保証・順序・resurrection・
   終了時挙動・target 間の観測可能性を独立 ADR で固定する。
4. 詳細な提案比較・コスト見積もりは調査文書
   [`docs/research/wasm-gc-post-mvp.md`](../research/wasm-gc-post-mvp.md) に置く。
   本 ADR はそれを採択しない。
5. **Wasm Exception Handling (`exnref`) は言語機能として導入しない。**
   issue #723 で性能比較・JS interop 具体化度を評価した結果、
   `Result<T, E>` モデルとの置き換え動機は乏しく、ユーザーに `throw`/`catch` を公開する
   メリットがない。当面 `Result<T, E>` を維持する。
   JS interop / `extern` 呼び出しでホスト例外を catch する具体例が issue/RFC 化され、
   ADR-009 / ADR-011 / ADR-032 との整合が確認された時点で FFI 境界のみの利用を再検討する。

---

## 帰結

- emitter / 型システムは MVP GC と shipped 機能の範囲で設計する。
- Post-MVP Phase 0 項目の「早期採用」をロードマップ必須にしない。

## 関連

- [`docs/research/wasm-gc-post-mvp.md`](../research/wasm-gc-post-mvp.md)
- ADR-002（Wasm GC 採用）
- ADR-009（Import 構文）
- ADR-011（host-bound stdlib API）
- ADR-032（Playground v2 ブラウザ compile + run）
- ADR-033（`call_ref` 移行）
- ADR-035（GC 実装方針 — 提案）
