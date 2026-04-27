---
Status: done
Created: 2026-03-30
Updated: 2026-04-03
ID: 278
Track: parallel
Depends on: 276
Orchestration class: implementation-ready
Blocks v1 exit: no
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Evidence: contributes.debuggers in package.json, arukellt debug-adapter command in main.rs
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---

- [x] `type: "arukellt"` の launch configuration で `arukellt debug-adapter` が起動する
# VS Code 拡張に debug contribution を追加する

---

## Closed by audit — 2026-04-03




## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/278-vscode-debug-contribution.md` — incorrect directory for an open issue.


## Summary

`extensions/arukellt-all-in-one/package.json` に `debuggers` contribution と launch configuration template がなく、VS Code から `arukellt debug-adapter` を使うことができない。

## Acceptance

- [x] `package.json` に `contributes.debuggers` エントリが追加されている
- [x] `.ark` ファイルのデフォルト launch configuration template が提供されている
- [x] `type: "arukellt"` の launch configuration で `arukellt debug-adapter` が起動する
- [x] F5 キーで `.ark` ファイルのデバッグを開始できる

## Scope

- `package.json` の `contributes.debuggers` セクションを追加
- `debugAdapterExecutable` または `debugAdapterServer` の設定
- `launch.json` snippet の提供
- 拡張機能側の debug adapter 起動コードの追加

## References

- `extensions/arukellt-all-in-one/package.json`
- `extensions/arukellt-all-in-one/src/extension.js`
- `issues/open/276-dap-core-verbs-implementation.md`
- `issues/open/255-dap-end-to-end-workflow.md`