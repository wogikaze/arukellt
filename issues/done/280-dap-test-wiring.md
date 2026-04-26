# DAP smoke test と E2E test を配線する

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 280
**Depends on**: 277, 278
**Track**: parallel
**Blocks v1 exit**: no

## Summary

DAP 実装と VS Code debug contribution が整っても、テストがなければ scaffold で止まる状態が繰り返される。DAP の smoke test と E2E test を CI に配線し、デバッグ機能が継続的に動作することを保証する。

## Acceptance

- [x] DAP プロトコルレベルの smoke test（initialize → launch → disconnect）が実装されている
- [x] `setBreakpoints → continue → stopped event` のシーケンスをテストするユニットテストが実装されている
- [x] VS Code 拡張側で launch が成功することを確認する E2E テストが実装されている
- [x] これらのテストが CI ジョブとして配線されている

## Scope

- `crates/ark-dap/tests/` に DAP プロトコルテストを追加
- `extensions/arukellt-all-in-one/src/test/` に debug launch E2E テストを追加
- CI ジョブへの組み込み

## References

- `crates/ark-dap/src/lib.rs`
- `issues/open/277-dap-breakpoint-step-implementation.md`
- `issues/open/278-vscode-debug-contribution.md`
- `issues/open/255-dap-end-to-end-workflow.md`
