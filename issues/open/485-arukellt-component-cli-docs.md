# docs: arukellt component サブコマンド CLI リファレンス

**Status**: open
**Created**: 2026-04-03
**Updated**: 2026-04-03
**ID**: 485
**Depends on**: 475
**Track**: docs
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #475
**Blocks v{N}**: none

---

## Decomposed from 475

Issue 475 (`arukellt-component-subcommand`) は CLI 実装と docs 更新を混ぜている。
この issue は **docs layer のみ** を担当する。
**#475 (実装) が close されるまでこの issue に着手してはならない**。

Upstream: #475 (CLI 実装) — close 後に着手

---

## Summary

`arukellt component` サブコマンド (#475) が実装された後に、
`docs/cli-reference.md` (または同等のファイル) に `component` サブコマンドの
リファレンスセクションを追加する。

記載内容:
- `arukellt component build <file.ark>` の用法・オプション・出力
- `arukellt component inspect <file.component.wasm>` の用法・出力形式
- `arukellt component validate <file.component.wasm>` の用法・exit code

## Why this is a separate issue

docs が実装前に「`arukellt component` が使える」と案内する構造を防ぐ。

475 の実装と同じ PR に docs を混ぜると、
実装が部分的でも docs が完成して「done 感」が出てしまう。

## Visibility

user-visible (ユーザーが読む docs; 存在しない機能を案内してはならない)

## Primary paths

- `docs/cli-reference.md` — `component` サブコマンドのセクション追加

## Allowed adjacent paths

- `README.md` (CLI overview の更新があれば)

## Non-goals

- `arukellt component` の CLI 実装 (#475)
- CHANGELOG の更新
- wasm-tools compose docs (#476)
- 非公開コマンド (internal debug commands) の文書化

## Acceptance

1. `docs/cli-reference.md` に `arukellt component` セクションが存在する
2. `arukellt component build`, `arukellt component inspect`, `arukellt component validate`
   の 3 サブコマンドが docs に記載されている
3. docs に書かれた各コマンドの `--help` 出力テキストが
   `arukellt component <subcmd> --help` の実際の出力と一致する
4. `bash scripts/run/verify-harness.sh --quick` が pass

## Required verification

- `grep "component build\|component inspect\|component validate" docs/cli-reference.md` が 3 件以上ヒット
- `arukellt component build --help` の出力と docs の記載が一致する (手動または CI 確認)
- `python3 scripts/check/check-docs-consistency.py` が pass

## Close gate

- `docs/cli-reference.md` に `arukellt component` セクションが存在する (grep で確認)
- **#475 が close 済みであること** — 実装のない機能を docs に書かない
- docs に書かれたサブコマンドが全て `arukellt component --help` で表示される

## Evidence to cite when closing

- `docs/cli-reference.md` の `arukellt component` セクション行番号
- `arukellt component --help` 出力との照合結果

## False-done risk if merged incorrectly

- #475 実装前に docs を書いて「docs も実装も done」と言う
  → Close gate に「#475 が close 済み」を必須にすることで防止
- docs の help テキストと実際の `--help` が乖離する
  → acceptance 3 でテキスト一致を要求
