---
Status: done
Created: 2026-03-31
Updated: 2026-04-03
ID: 388
Track: stdlib-api
Depends on: —
Orchestration class: implementation-ready
---
# Stdlib: collections / seq family の algorithm parity と回帰検証を強化する
**Blocks v1 exit**: no
**Priority**: 6

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: tests/fixtures/stdlib_collections/ with seq_basic and seq_edge_cases

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/388-stdlib-collections-seq-parity-tests.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

collections / seq family の API を、名前だけの存在から実運用で信頼できる面に引き上げる。map/filter/fold、search、sort、set/map 操作の基本パターンについて parity テストとエッジケース検証を追加する。

## Current state

- collections / seq は関数数が多い一方、組み合わせ使用やエッジケースの回帰テストが十分ではない。
- アルゴリズム系 helper は docs に並ぶが、空集合・重複・安定順序の扱いが fixture で固まっていない。
- モノモーフィックな historical API と canonical API の両方が見えており、推奨面が太くない。

## Acceptance

- [x] collections / seq family に対する高密度 fixture が追加される。
- [x] 空入力・重複・順序保持などのエッジケースがテストで固定される。
- [x] canonical API を優先した recipe または examples が追加される。
- [x] 回帰時に壊れた family を特定できる test grouping が用意される。

## References

- ``std/collections/**``
- ``std/seq/**``
- ``tests/fixtures/``
- ``docs/stdlib/reference.md``