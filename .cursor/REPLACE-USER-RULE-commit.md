# コミット方針（Cursor Rules）

## このリポジトリで有効な設定

Cursor Settings → **Rules** に表示される次の項目で、自律コミットはすでに有効です。

| UI に表示される名前 | 実体 |
|--------------------|------|
| **git-commits** | `.cursor/rules/git-commits.mdc`（`alwaysApply: true`） |
| **AGENTS** | `AGENTS.md` → Commit Policy |
| CLAUDE | `CLAUDE.md`（リポジトリ境界・検証ループ） |
| japanese-summaries | `.cursor/rules/japanese-summaries.mdc` |

**追加の設定は不要です。** 作業完了後、エージェントはユーザーに「コミットしますか？」と聞かず、ターン終了前にコミットします。

## 優先順位

1. `.cursor/rules/git-commits.mdc`（常時適用）
2. `AGENTS.md` Commit Policy
3. `prompts/research.md` の autonomous commit policy（監査 orchestration 用）

## グローバル User Rules がある場合のみ（任意）

Cursor のバージョンやアカウント設定によっては、**プロジェクト Rules とは別**に
グローバル User Rules（「明示依頼時のみコミット」など）が残っていることがあります。
Settings に **User Rules** タブが見当たらない場合は、**無視して構いません** —
上記の Project Rules がこのリポジトリでは十分です。

グローバル側に `committing-changes-with-git` がある場合だけ、下記に置換してください。

```markdown
<committing-changes-with-git>
In the arukellt repository, follow `.cursor/rules/git-commits.mdc` and `AGENTS.md` Commit Policy.

- Commit after every completed work unit **before ending the turn**, without asking the user first.
- Do not end a turn with uncommitted implementation, docs, indexes, or regenerated artifacts.
- Do not `git push` unless the user explicitly requests it.
- Never update git config; never skip hooks; never force-push to main/master.
- Use HEREDOC for commit messages; fix pre-commit failures with a new commit (no amend unless hooks auto-modified files on your commit).
- Orchestration-only changes (issues, index, audit reports) may be a separate commit from product implementation when both land in one session.
</committing-changes-with-git>
```
