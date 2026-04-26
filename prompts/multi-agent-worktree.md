# Multi-Agent Git Worktree Orchestration Prompt

## 目的

Open Issues Priority Table を git worktree 分割し、複数エージェントが並列・安全に作業できるようにする。最終的に全ブランチを master にマージし、すべての open issue を close することを目標とする。

## 前提

- リポジトリ: `wogikaze/arukellt`
- 作業起点ブランチ: `master`
- ツール: git worktree
- 最大同時 worktree 数: 6（+ master worktree = 7）

## Issue 分割戦略（7 Worktree Groups）

以下のグループは **track + トポロジカルレベル** で分割。各グループ内では Level 順に wave を実行する。

```
Group A: wt-retirement   — selfhost-retirement (長鎖依存: 563→564→574→575→576→577→{578,579,581}→582)
Group B: wt-selfhost     — selfhost / language-design / compiler (593→508/594, 595→596/597/599, 598→599, 600→601→602→603, 099, 123, 125→126, 610, 611→612, 614, 615)
Group C: wt-wasi         — wasi-feature / runtime (510→121→074→{076,077,124,139,474,475,476}, 475→485, 076→543, 077→136, 139→136)
Group D: wt-stdlib      — stdlib (044→054/055, 045, 047, 051, 512, 520, 604→605/606/607/608, 613)
Group E: wt-surface      — component-model / playground / docs / scripts (034, 036, 204, 205, 214, 436, 437, 468, 469, 470, 489, 500, 531, 588, 589, 590, 591, 592)
Group F: wt-release      — release tests (546, 547, 548, 549, 550, 551, 552, 553, 554, 555)
Group M: master          — 統合・マージ管理（元の worktree）
```

## 実行フロー

### Phase 0: 準備（master worktree で実行）

```bash
# 前提: master worktree で実行中
git fetch origin
git checkout master
git pull origin master

# 各 worktree 用ブランチを master から作成
git branch work/retirement origin/master
git branch work/selfhost origin/master
git branch work/wasi origin/master
git branch work/stdlib origin/master
git branch work/surface origin/master
git branch work/release origin/master

# worktree 追加
git worktree add ../wt-retirement work/retirement
git worktree add ../wt-selfhost work/selfhost
git worktree add ../wt-wasi work/wasi
git worktree add ../wt-stdlib work/stdlib
git worktree add ../wt-surface work/surface
git worktree add ../wt-release work/release
```

### Phase 1: Wave 実行（各 worktree で並列）

各 worktree で **Level 0 のみを先に並列実行**。完了後 Phase 2 へ。

**親オーケストレータの役割:**
1. `git worktree list` で各 worktree のブランチ・状態を確認
2. 各 worktree で `git status` → conflict / unmerged ファイルがないか確認
3. Level N の全 worktree wave が完了したら、**main worktree に戻してマージ**

### Phase 2: 統合マージ（main worktree で実行）

各 Level 完了後、以下を実行:

```bash
# main worktree で
git checkout main

# 各 worktree ブランチを順次マージ（衝突が少ない順）
git merge --no-ff work/selfhost -m "merge(selfhost): wave N complete"
git merge --no-ff work/stdlib -m "merge(stdlib): wave N complete"
git merge --no-ff work/wasi -m "merge(wasi): wave N complete"
git merge --no-ff work/surface -m "merge(surface): wave N complete"
git merge --no-ff work/release -m "merge(release): wave N complete"
git merge --no-ff work/retirement -m "merge(retirement): wave N complete"

# マージ完了後、次 Level の worktree を rebase
cd ../wt-selfhost && git rebase main
cd ../wt-wasi && git rebase main
# ... 各 worktree で同様
```

### Phase 3: 次 Level の dispatch

マージ後、各 worktree で **次 Level の issue を wave 実行**。繰り返し。

## Wave Barrier ルール（絶対）

1. **同じ worktree 内では必ず Level 順に実行**（下位 Level が未完了なら上位 Level を dispatch しない）
2. **main マージ前に、当該 Level の全 worktree wave が完了していることを確認**
3. **マージ後、次 Level 開始前に必ず `git worktree list` + `git status` でクリーン状態を確認**
4. **conflict 発生時は、その worktree のみ停止し、他 worktree は継続可能**
5. **1 worktree あたり最大 10 並列（同 Level 内の独立 issue）**

## マージ衝突対応

```bash
# 衝突発生時（main worktree）
git merge work/xxx
# CONFLICT 発生時
# 1. 衝突ファイルを特定
git diff --name-only --diff-filter=U
# 2. 該当 worktree で修正後 commit
# 3. main で merge --continue
```

## マルチエージェント割り当て例

| エージェント | 担当 Worktree | 初期 Level | 備考 |
|-------------|--------------|-----------|------|
| Agent-1 | wt-retirement | Level 0 (563, 571) | 長鎖依存、最も注意が必要 |
| Agent-2 | wt-selfhost | Level 0 (593, 595, 598, 600, 604, 610, 613, 614, 615, 099, 123, 125) | 最大グループ |
| Agent-3 | wt-wasi | Level 0 (510) | 510が後続多数をブロック |
| Agent-4 | wt-stdlib | Level 0 (044, 045, 047, 051, 512, 520, 604) | 独立が多い |
| Agent-5 | wt-surface | Level 0 (034, 036, 204, 205, 214, 436, 468-470, 489, 500, 531, 588-592) | 最も独立 |
| Agent-6 | wt-release | Level 0 (546-555) | リリース系、独立 |

## 最終目標チェックリスト

- [ ] 全 worktree の全 Level wave が完了
- [ ] 全 worktree ブランチが main にマージ済み
- [ ] `git worktree list` で extra worktree が残っていない（or 削除済み）
- [ ] `python scripts/manager.py verify --full` が pass
- [ ] `issues/open/` が空（または blocked-by-upstream のみ残存）
- [ ] 全 close issue に `chore(issues): close #NNN` コミットあり

## 禁止事項

- **異なる worktree 間で同じファイルを同時編集**（マージ衝突の元）
- **main への直接コミット**（必ず worktree ブランチ経由でマージ）
- **Level N+1 を Level N のマージ前に開始**
- **worktree 削除前に未コミット変更がある状態で削除**

## 後片付け（全 issue close 後）

```bash
# 不要な worktree を削除
git worktree remove ../wt-retirement
git worktree remove ../wt-selfhost
git worktree remove ../wt-wasi
git worktree remove ../wt-stdlib
git worktree remove ../wt-surface
git worktree remove ../wt-release

# リモートブランチ削除（必要に応じて）
git push origin --delete work/retirement work/selfhost work/wasi work/stdlib work/surface work/release
```
