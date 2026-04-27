---
Status: done
Created: 2026-03-30
Updated: 2026-04-03
ID: 256
Track: main
Depends on: 251
Orchestration class: implementation-ready
Blocks v1 exit: yes
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Evidence: ARUKELLT_TARGET set to wasm32-wasi-p2 and wasm32-wasi-p1 in ci.yml
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---
# CI の target matrix に実際の CLI 引数・emit 種別を注入する

---

## Closed by audit — 2026-04-03




## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/256-ci-target-matrix-inject-args.md` — incorrect directory for an open issue.


## Summary

`.github/workflows/ci.yml` の `target-behavior` job は matrix で `wasm32-wasi-p1` / `wasm32-wasi-p2` を回しているが、matrix 値が harness 実行に注入されておらず、実際には同一コマンドを2回実行しているだけである。

## Acceptance

- [x] `target-behavior` job の matrix 値が harness 実行時の `--target` / `--emit` 引数に注入されている
- [x] `wasm32-wasi-p1` と `wasm32-wasi-p2` のジョブが異なる CLI 引数で実行される
- [x] emit-core と emit-component が target ごとに分離した step として実行される
- [x] CI ログで「どの target の、どの emit 種別のテスト」かが判別できる

## Scope

- `.github/workflows/ci.yml` の `target-behavior` job の matrix → harness 引数マッピングを実装
- `cargo test -p arukellt --test harness` に `ARUKELLT_TARGET` 等の環境変数を渡す仕組みを追加
- harness 側でその環境変数を参照して target を切り替える配線を追加

## References

- `.github/workflows/ci.yml`
- `tests/harness.rs`
- `issues/open/251-target-matrix-execution-contract.md`