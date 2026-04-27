---
Status: done
Created: 2026-03-30
Updated: 2026-04-03
ID: 258
Track: main
Depends on: 257
Orchestration class: implementation-ready
---
# core Wasm と component 出力の保証レベルを分離する
**Blocks v1 exit**: yes

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: docs/target-contract.md has emit-core and emit-component as separate rows

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/258-core-wasm-vs-component-guarantee-split.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`--emit component` は `wasm32-wasi-p2` 上で利用可能とされているが、`wasm-tools` と adapter バイナリに依存するため、core Wasm 出力と component 出力の保証レベルは同列ではない。この区別が CI と docs の両方で曖昧になっている。

## Acceptance

- [x] `docs/target-contract.md` で `emit-core` と `emit-component` が別行として定義されている
- [x] CI で core Wasm 検証と component 検証が独立した step として実行される
- [x] `wasm-tools` / adapter 依存の有無が target contract に明記されている
- [x] component 出力が optional smoke tier であることが CI のジョブ名・docs の両方で明確になっている

## Scope

- `docs/target-contract.md` の emit 行を core / component に分割
- CI の component 出力 step に `wasm-tools` 依存チェックを追加
- `scripts/run/verify-harness.sh` の `--component` フラグと CI ジョブの対応を整理

## References

- `scripts/run/verify-harness.sh`
- `issues/open/257-target-contract-table.md`
- `issues/open/251-target-matrix-execution-contract.md`