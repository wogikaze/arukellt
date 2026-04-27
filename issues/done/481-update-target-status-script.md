---
Status: done
Created: 2026-04-03
Updated: 2026-04-03
ID: 481
Track: main
Depends on: 256, 257
Orchestration class: implementation-ready
---
# scripts/update-target-status.sh: CI 出力から target-contract.md を更新
**Blocks v1 exit**: no

---

## Decomposed from 260

Issue 260 (`current-state-target-table-from-ci`) は:
1. `scripts/update-target-status.sh` の実装
2. CI での drift-check ジョブ追加

の 2 層を混ぜている。この issue は **script implementation layer** のみを担当する。
CI 配線 (drift-check ジョブ) は issue 260 が引き続き担当する。

Downstream: #260 (CI drift-check) — この issue 完了後に着手

---

## Summary

`scripts/update-target-status.sh` (または同等のスクリプト) を実装する。
スクリプトは CI の target-behavior テスト結果 (JSON/テキスト) を読み取り、
`docs/target-contract.md` の保証レベルセルを自動更新する。

スクリプトが存在することで、#260 の CI drift-check がこのスクリプトを
呼び出した結果と committed ファイルを比較できるようになる。

## Why this is a separate issue

スクリプト実装と CI 配線は別々に PR でき、別々にレビューできる。
スクリプトが動作確認済みになって初めて、CI が正しく呼び出せる。
CI job を先に作ってもスクリプトがなければ drift-check は機能しない。

## Visibility

internal-only (CI / Makefile から呼ばれるだけ; ユーザーが直接触る surface ではない)

## Primary paths

- `scripts/update-target-status.sh` — 新規作成

## Allowed adjacent paths

- `docs/target-contract.md` — スクリプトが更新するファイル (実行後の変化)

## Non-goals

- CI ジョブへの組み込み (#260)
- drift-check の実装 (#260)
- `docs/current-state.md` の自動更新 (separate concern)
- target-contract.md の schema 変更

## Acceptance

1. `scripts/update-target-status.sh` が repo に存在し、実行可能 (chmod +x)
2. `bash scripts/update-target-status.sh --dry-run` が exit 0 を返し、
   更新予定の変更内容を stdout に出力する
3. `bash scripts/update-target-status.sh` が `docs/target-contract.md` の
   保証レベルセルを入力データに基づいて更新する (最低 1 行の更新を確認)
4. スクリプトの入力フォーマット (CI 出力の形式) が `docs/` または `scripts/` の
   README または inline comment に記載されている

## Required verification

- `test -x scripts/update-target-status.sh` が exit 0
- `bash scripts/update-target-status.sh --help` が exit 0 (usage 出力)
- `bash scripts/run/verify-harness.sh --quick` が pass

## Close gate

- `scripts/update-target-status.sh` が repo に存在し実行可能
- `--dry-run` モードが動作する
- 入力フォーマットが documented である
- CI ジョブ (#260) はこの issue の close 条件ではない

## Evidence to cite when closing

- `scripts/update-target-status.sh` (file path)
- `bash scripts/update-target-status.sh --dry-run` の出力

## False-done risk if merged incorrectly

- スクリプトが存在するだけで「CI が target-contract.md を更新している」と誤解される
  → Visibility を internal-only にし、CI 配線は #260 が担当と明記
- CI drift-check が通っていない状態でスクリプト実装だけを done にしてしまう
  → Close gate に「CI ジョブは #260 の担当」と明記; 混同を防ぐ

## Close Evidence

- `scripts/update-target-status.sh` created, executable (`-rwxr-xr-x`)
- `--help` exits 0 with full usage text
- `--dry-run` mode works: shows unified diff of proposed changes without modifying file
- Input format documented inline (see INPUT JSON FORMAT section in script)
- `bash scripts/run/verify-harness.sh --quick`: 19/19 PASS

### dry-run sample output

```
$ echo '{"wasm32-wasi-p1": {"parse": "smoke"}}' | bash scripts/update-target-status.sh --dry-run
--- a/target-contract.md
+++ b/target-contract.md
@@ -25,7 +25,7 @@
 | Surface | Status | Detail |
 |---------|--------|--------|
-| parse | guaranteed | 209 `run` + ...
+| parse | smoke | 209 `run` + ...

[dry-run] 1 line(s) would be updated in docs/target-contract.md
```