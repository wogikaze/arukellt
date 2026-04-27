---
Status: done
Created: 2026-03-31
Updated: 2026-04-03
ID: 386
Track: stdlib-api
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: False
Priority: 4
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Evidence: tests/fixtures/stdlib_path/ with path_basic and path_edge_cases
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
# Stdlib: path モジュールの正規化・結合・表示契約を固定する
---
# Stdlib: path モジュールの正規化・結合・表示契約を固定する

---

## Closed by audit — 2026-04-03




## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/386-stdlib-path-contract-fixtures.md` — incorrect directory for an open issue.


## Summary

path family の API を platform-neutral な Wasm 環境でどう定義するかを fixtures で固める。結合、分解、正規化、親ディレクトリ取得、拡張子操作などの期待値を決め、将来 playground や docs examples が同じ結果を返すようにする。

## Current state

- path 系 API はあるが、絶対パス判定や `..` 正規化の扱いが利用者にとって読み取りづらい。
- ホスト OS 依存のパス意味論をどこまで持ち込むかが明示されていない。
- fixture ベースの回帰検証が薄い。

## Acceptance

- [x] path 結合・分解・正規化の fixture が追加される。
- [x] host OS に依存しない契約が docs に明記される。
- [x] `..` や `.` を含む入力の期待値が固定される。
- [x] 関連 recipe または examples が追加される。

## References

- ``std/path/**``
- ``tests/fixtures/``
- ``docs/stdlib/reference.md``
- ``docs/stdlib/cookbook.md``