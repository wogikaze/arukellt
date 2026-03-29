# Extension security surface analysis

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 207
**Depends on**: 184, 188
**Track**: parallel
**Blocks v1 exit**: no

## Summary

external tool 実行、env 受け渡し、script 実行、manifest 利用、他 extension との連携を含むデータ露出面を洗い出し、危険な workflow や設定を警告できるようにする。all-in-one 拡張の security/DX 境界を追う child issue。

## Acceptance

- [ ] extension の data exposure / command execution surface が追跡できる
- [ ] 危険な workflow / setting warning の責務が定義されている
- [ ] security review 系の残課題を issue queue 上で追跡できる

## References

- `issues/open/183-vscode-arukellt-all-in-one-extension-epic.md`
- `issues/open/184-vscode-extension-foundation.md`
- `issues/open/188-ark-toml-project-workspace-and-scripts.md`
