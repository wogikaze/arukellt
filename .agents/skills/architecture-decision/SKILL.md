---
name: architecture-decision
description: Arukelltの長期的な設計判断をADRとして新設・置換・整理する。複数の妥当な選択肢があり、将来の実装を拘束する判断が必要なときに使う。調査、詳細仕様、実装計画、進捗更新だけには使わない。
---

# Architecture decision

ADR-000 に従い、意思決定だけを ADR に残す。

## まず置き場を判定する

- 長期的な設計判断と却下した代替案: `docs/adr/`
- 詳細仕様・長文提案: `docs/rfcs/`
- 実装順序・PR分割・一時制限: `docs/plans/`
- 実測・比較・未決の調査: `docs/research/`
- 現行挙動・対応状況・件数: `docs/current-state.md` または `docs/data/*.toml`
- 作業追跡・完了条件: issue

ADR が不要なら作らない。

## ADR 作成・置換手順

1. `docs/adr/ADR-000-process.md` と生成索引 `docs/adr/README.md` を読む。
2. 関連する `ACCEPTED`、`PROPOSED`、`SUPERSEDED` ADR を確認する。
3. 現行決定を置換する場合は新しい ADR を作り、旧 ADR を `SUPERSEDED` にして `後継` を付ける。旧本文を上書きして履歴を消さない。
4. 本文は日本語を正とし、正規ヘッダを使う。
   - 提案: `ステータス: **PROPOSED** — ...` と `提案日:`
   - 採択: `ステータス: **ACCEPTED** — ...` と `決定日:`
5. 「理想形としての決定」と「現行の未実装・暫定制限」を分離する。実装ギャップは current-state / plan / issue へ参照する。
6. 文脈、決定、帰結、代替案、再検討条件、関連文書を記録する。進捗表や fixture 件数を埋め込まない。
7. 言語 docs を変える場合は ADR-018 の文書分類、公開 API を変える場合は ADR-014 の安定性を確認する。

## 検証

原則として次を実行する。

- `python3 scripts/manager.py docs check`
- `python3 scripts/manager.py verify quick`

ADR 索引が生成物なら生成元・既存 generator を使って再生成し、索引だけを手編集しない。
