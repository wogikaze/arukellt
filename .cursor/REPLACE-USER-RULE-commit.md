# Cursor User Rules: コミット方針の置き換え

グローバル User Rules にある `committing-changes-with-git`（「明示依頼時のみコミット」）は、
このリポジトリでは **`.cursor/rules/git-commits.mdc`**（`alwaysApply: true`）と矛盾します。

**Workspace rule が優先**されますが、エージェントが旧 User Rule を読み続けると停止・確認待ちが起きるため、User Rules も下記に置換してください。

## 手順

1. **Cursor Settings** → **Rules** → **User Rules** を開く
2. `<committing-changes-with-git>...</committing-changes-with-git>` ブロックを**削除**
3. 下記を User Rules に貼る（ブロック全体をこれに置換）

## 貼り付け用（新 User Rule）

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

## 優先順位（このリポジトリ）

| 層 | 正 |
|----|-----|
| `.cursor/rules/git-commits.mdc` (`alwaysApply: true`) | 作業完了ごとにコミット（ユーザー確認不要） |
| `AGENTS.md` Commit Policy | 同上 |
| `prompts/research.md` autonomous commit policy | 監査 orchestration も自律コミット |
| 旧 User Rule「明示依頼のみ」 | **削除または上記に置換** |
