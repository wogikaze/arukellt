# False-Done 全件監査 — オーケストレーション計画

> For cloud `/orchestrate` root planner.  
> Contract: `prompts/research.md`, prevention: `docs/process/false-done-prevention.md`  
> **Status: complete (2026-06-12)** — see `docs/process/false-done-audit-2026-06-12.md`

## ゴール

1. `issues/done/` **全件**を分類（`truly-done` / `must-reopen` / …）
2. false-done は `issues/open/` へ移動 + acceptance 巻き戻し
3. 根本原因を `false-done-prevention.md` に反映
4. reopen した issue ごとに **close-gate fixture** を追加してからのみ再-close

## 完了済み（2026-06-12）

| Wave / slice | Reopened | New issues | Notes |
|--------------|---------:|------------|-------|
| Wave 1 | 5 | — | #074, #510, #472, #500, #051 |
| Wave 2 | 1 | — | #123 |
| Wave 3 | — | — | prevention + hygiene scripts |
| Wave 3b | 2 | #633 | #446, #447 |
| Slice F | 2 | — | #418, #422 |
| Slice A | 7 | — | FD-01 metadata (#064, #067, #070, #080, #082, #083, #115) |
| Slice B | 7 | — | user-visible (#216, #217, #219, #440, #456, #464, #491) |
| Slice C | 7 | — | component/WIT/WASI (#034, #073, #117, #118, #138, #443, #618) |
| Slice D | 6 | — | stdlib/host (#137, #292, #293, #295, #358, #445) |
| Slice E | 29 | #634 | LSP/IDE/vscode cluster |
| Slice G | 2 | — | #439, #441 |
| **Total** | **68** | **2** | Open 80 / Done 542 / Blocked 2 |

Deliverables: `false-done-prevention.md`, `check-false-done-hygiene.py`, `check-false-done-close-gates.py`, `playground/src/tests/typecheck-close-gate.test.ts`, full audit report.

Verify at close: `python3 scripts/manager.py verify quick` → **149/149**.

## 分解計画（実行済み）

| Slice | 対象 | 結果 |
|-------|------|------|
| A | `issues/done/` に `Moved to open` 履歴がある件 | 156 候補 → 7 reopen, 149 kept done |
| B | user-visible: playground, extension, CLI, docs routes | 7 reopen |
| C | component / WIT / WASI (#074 依存グラフ) | 7 reopen |
| D | stdlib / host intrinsics | 6 reopen |
| E | LSP / IDE / vscode | 29 reopen + #634 |
| F | release / benchmark / hygiene | 2 reopen |
| G | 残り mechanical spot-check | 278 uncovered → 2 reopen, 171 truly-done |

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

対象: reopen 済み open issue（68件 + 新規 #633/#634）

```
1. Read issue acceptance + false-done-prevention.md table
2. Add minimal fixture under tests/fixtures/
3. Register in manifest.txt
4. Ensure verify quick passes
5. Commit implementation + issue progress note (separate from audit commit)
```

優先レーン: #074/#510, #472/#500, #634, #633

## ブロッカー記録

Cloud kickoff failed (2026-06-12):

```
validation_error: Failed to verify existence of branch 'master' in repository wogikaze/arukellt
```

**Remediation**: Cursor Dashboard で `wogikaze/arukellt` を連携；personal `CURSOR_API_KEY`；必要なら default branch を cloud が見える状態に。

Local fallback で slices A–G を完了（`CURSOR_API_KEY` 未設定の cloud VM では planner が serial 実行）。
