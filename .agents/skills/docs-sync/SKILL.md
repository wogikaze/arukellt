---
name: docs-sync
description: 実装、ADR、構造化データとArukelltのドキュメントを同期し、生成物・current-state・言語文書分類のドリフトを直す。docs変更、公開挙動変更、generator入力変更、古い記述の監査に使う。
---

# Documentation sync

文書の種類ごとに正本を守り、生成物を直接修正しない。

## 手順

1. `docs/directory-ownership.md` で対象が hand-maintained か generated か確認する。
2. 現行挙動は `docs/current-state.md` と `docs/data/*.toml`、設計理由は ADR、詳細仕様は RFC、計画は plan として分離する。
3. 生成ファイルの場合は generator または TOML/manifest などの入力を編集する。
4. `docs/language/` を追加・変更する場合は `docs/data/language-doc-classifications.toml` と ADR-018 を確認し、normative / explanatory / transitional の役割を混ぜない。
5. normative な主張は実装・fixture の証拠と一致させる。未実装の理想形を current behavior として書かない。
6. stdlib reference は `std/manifest.toml` 等の正本から生成する。公開 API 例は ADR-044/046 に従い method / associated 形を使い、free function を推奨しない。
7. 廃止された target alias、Rust-era の正本、古い verification command、SUPERSEDED ADR を現行方針として参照していないか検索する。

## 検証

通常は次を実行する。

1. `python3 scripts/manager.py docs regenerate`
2. `python3 scripts/manager.py docs check`
3. 挙動・例・fixture に触れた場合は `python3 scripts/manager.py verify quick`

再生成後の差分を確認し、手編集した生成物だけが変わっていない状態にする。
