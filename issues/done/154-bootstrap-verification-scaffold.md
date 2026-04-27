---
Status: done
Created: 2026-03-29
Updated: 2026-04-15
ID: 154
Track: cross-cutting
Depends on: 153
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v1 exit: False
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
Reviewer: "implementation-backed queue normalization (verify checklist)."
# 横断基盤: `scripts/run/verify-bootstrap.sh` と fixpoint 検証 scaffold
---
# 横断基盤: `scripts/run/verify-bootstrap.sh` と fixpoint 検証 scaffold

---

## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/154-bootstrap-verification-scaffold.md` — incorrect directory for an open issue.


## Summary

`docs/process/roadmap-cross-cutting.md` §6.5 は v5 で `scripts/run/verify-bootstrap.sh` による
Stage0 → Stage1 → Stage2 → fixpoint 検証を要求している。
まだセルフホスト本体は先だが、verify 導線と artifact 契約は先に scaffold 化しておいた方が、
v5 に入ったときの曖昧さを減らせる。

## 受け入れ条件

1. `scripts/run/verify-bootstrap.sh` の雛形が追加され、将来の Stage0/1/2/fixpoint 手順を受け止められる
2. 期待する artifact 命名、比較対象、失敗時の diff 出力方針が明文化される
3. `docs/compiler/bootstrap.md` の着手前提となる scaffold ができる
4. verify-harness から将来的に組み込み可能な契約が定義される

## 実装タスク

1. v5 roadmap と cross-cutting の bootstrap requirements を抽出する
2. script 雛形と usage / TODO セクションを作る
3. Stage artifact 名、比較方法、決定性前提を決める
4. process docs に bootstrap verify の入口を追加する

## 参照

- `docs/process/roadmap-cross-cutting.md` §6.5
- `docs/process/roadmap-v5.md`
- `scripts/run/verify-harness.sh`

## Progress (verification infra)

- **2026-04-18:** Acceptance #2 — `scripts/run/verify-bootstrap.sh` header documents
  artifact naming, Stage 2 comparison (`sha256sum` digest equality), and
  failure/diff policy; `docs/compiler/bootstrap.md` links to that header contract.

---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).