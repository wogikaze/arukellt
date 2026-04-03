# Extension README: 設定一覧テーブル追加

**Status**: open
**Created**: 2026-04-03
**Updated**: 2026-04-03
**ID**: 480
**Depends on**: 479
**Track**: docs
**Blocks v1 exit**: no

---

## Decomposed from 462

Issue 462 (`extension-settings-rationalization`) の **docs layer** を担当する。
LSP サーバーで設定が実際に動作するようになった (#479) 後に、
README に設定一覧テーブルを追加する。

**実装前に docs を先行させない** — これがこの issue を分離した理由。

Upstream: #479 (LSP 実装) — 完了後に着手

---

## Summary

`extensions/arukellt-all-in-one/README.md` に "Extension Settings" セクションを追加し、
全 7 設定 (既存 5 + 新規 2) の一覧テーブルを設ける。
テーブルには設定名・型・デフォルト値・説明を含める。

---

## Why this is a separate issue

docs update を実装 issue と混ぜると「README を更新しただけ」で issue が done になる構造になる。
この issue は #479 (LSP 実装) が完了して初めて着手できる。
README が「設定が使える」と説明する時点で、実際にその設定が動作していなければならない。

## Visibility

user-visible (README はユーザーが読む。存在しない機能を案内しないことが重要)

## Primary paths

- `extensions/arukellt-all-in-one/README.md`

## Allowed adjacent paths

- なし

## Non-goals

- package.json の変更 (#477)
- extension.js の変更 (#478)
- LSP サーバーの変更 (#479)
- CHANGELOG の更新 (別途行う)
- ウェブサイト/外部ドキュメントの更新

## Acceptance

1. `README.md` に `## Extension Settings` (または同等の heading) セクションが存在する
2. セクションには全 7 設定を含む Markdown テーブルがあり、
   `arukellt.check.onSave` が含まれている
3. テーブルの各行に: 設定名・型・デフォルト値・説明 の 4 列がある
4. README に書かれた設定名が全て `package.json` の `contributes.configuration.properties`
   に存在するキーと一致する
5. README に書かれたデフォルト値が `package.json` の `default` 値と一致する

## Required verification

- `grep "Extension Settings" extensions/arukellt-all-in-one/README.md` が 1 件ヒットする
- `grep "check.onSave" extensions/arukellt-all-in-one/README.md` が 1 件ヒットする
- テーブル内の設定名を `package.json` に対して照合し、全設定が一致する
  (手動または `python3 scripts/check/check-docs-consistency.py` 相当)

## Close gate

- `README.md` に settings テーブルが存在する (grep で確認)
- テーブルの全設定名が `package.json` に存在する (照合)
- **#479 が close 済みであること** — 実装が完了した設定だけを docs に書く

## Evidence to cite when closing

- `extensions/arukellt-all-in-one/README.md` の `## Extension Settings` 行番号
- `package.json` のキー一覧との照合結果

## False-done risk if merged incorrectly

- #479 完了前に README に「設定が使える」と書いて close する
  → Close gate に「#479 が close 済みであること」を必須にすることで防止
- テーブルのデフォルト値が package.json と異なる
  → acceptance 5 でデフォルト値一致を明示的に要求
