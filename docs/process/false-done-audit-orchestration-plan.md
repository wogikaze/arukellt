# False-Done 全件監査 — オーケストレーション計画

> For cloud `/orchestrate` root planner.  
> Contract: `prompts/research.md`, prevention: `docs/process/false-done-prevention.md`

## ゴール

1. `issues/done/` **全件**を分類（`truly-done` / `must-reopen` / …）
2. false-done は `issues/open/` へ移動 + acceptance 巻き戻し
3. 根本原因を `false-done-prevention.md` に反映
4. reopen した issue ごとに **close-gate fixture** を追加してからのみ再-close

## 完了済み（2026-06-12）

| Wave | Reopen | Commits |
|------|--------|---------|
| 1 | #074, #510, #472, #500, #051 | `c07e4486` |
| 2 | #123 + #034 resolution (done) | `5af62956` |

## 推奨分解（subplanner）

| Slice | 対象 | 件数目安 |
|-------|------|----------|
| A | `issues/done/` に `Moved to open` 履歴がある件（re-close 証拠の有無） | ~150 |
| B | user-visible: playground, extension, CLI, docs routes | ~40 |
| C | component / WIT / WASI (#074 依存グラフ) | ~30 |
| D | stdlib / host intrinsics | ~50 |
| E | LSP / IDE / vscode | ~40 |
| F | release / benchmark / hygiene | ~30 |
| G | 残り mechanical truly-done スポットチェック | 残 |

各 subplanner は **50件/wave** 上限。確証 false-done のみ reopen。

## Worker タスクテンプレート（audit）

```
1. Read issues/done/<batch>/*.md
2. For each: grep acceptance, check manifest/fixtures/src
3. Classify per research.md
4. must-reopen → git mv + reopen section + uncheck acceptance
5. python3 scripts/gen/generate-issue-index.py
6. python3 scripts/manager.py verify quick
7. Commit orchestration-state only
8. Append to docs/process/false-done-audit-2026-06-12.md
```

## Worker タスクテンプレート（close-gate test）

対象: 既に reopen 済み open issue（#074, #510, #472, #500, #051, #123, …）

```
1. Read issue acceptance + false-done-prevention.md table
2. Add minimal fixture under tests/fixtures/
3. Register in manifest.txt
4. Ensure verify quick passes
5. Commit implementation + issue progress note (separate from audit commit)
```

## ブロッカー記録

Cloud kickoff failed (2026-06-12):

```
validation_error: Failed to verify existence of branch 'master' in repository wogikaze/arukellt
```

**Remediation**: Cursor Dashboard で `wogikaze/arukellt` を連携；personal `CURSOR_API_KEY`；必要なら default branch を cloud が見える状態に。再実行:

```bash
bun cli.ts kickoff "<goal>" --repo https://github.com/wogikaze/arukellt.git --ref master
```

Local fallback: root planner が `prompts/research.md` を読み、上記 slice を順次実行。
