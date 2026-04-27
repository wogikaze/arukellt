# 親オーケストレータ（exec）— redirect

**統合済み。** 運用ルール・分類・wave・worktree・ゲート・close 手順はすべて次に集約されています。

- **[`orchestration.md`](orchestration.md)** ← **ここだけ使う**

実装者 / 検証者向け（従来どおり）:

- [`subagent-slice.md`](subagent-slice.md)
- [`subagent-verify.md`](subagent-verify.md)

**スコープ機械チェック:**

```bash
python3 scripts/util/check-diff-scope.py --base origin/master --head HEAD \
  --primary <paths>... --allowed <paths>... [--forbidden <paths>...]
```

**worktree 作成:**

```bash
bash scripts/util/agent-worktree-add.sh wt/<id> feat/<issue>-<slice> master
```
