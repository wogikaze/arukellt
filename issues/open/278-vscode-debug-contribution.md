# VS Code 拡張に debug contribution を追加する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 278
**Depends on**: 276
**Track**: parallel
**Blocks v1 exit**: no

## Summary

`extensions/arukellt-all-in-one/package.json` に `debuggers` contribution と launch configuration template がなく、VS Code から `arukellt debug-adapter` を使うことができない。

## Acceptance

- [ ] `package.json` に `contributes.debuggers` エントリが追加されている
- [ ] `.ark` ファイルのデフォルト launch configuration template が提供されている
- [ ] `type: "arukellt"` の launch configuration で `arukellt debug-adapter` が起動する
- [ ] F5 キーで `.ark` ファイルのデバッグを開始できる

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
