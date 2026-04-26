# VS Code Test Explorer + inline test execution

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-03-30
**ID**: 197
**Depends on**: 196
**Track**: parallel
**Blocks v1 exit**: no

## Summary

`arukellt test` の runner surface を使って、VS Code Test Explorer、gutter/code-lens 実行、rerun/filter 導線を整備する。CLI runner と editor integration を分離して追う。

## Acceptance

- [x] VS Code Test Explorer integration の責務が追跡できる
- [x] inline / gutter / code-lens 実行導線が定義されている
- [x] rerun / filter / focused execution UX を issue queue 上で追跡できる

## References

- `issues/open/186-test-runner-and-vscode-test-explorer-surface.md`
- `issues/open/196-arukellt-test-discovery-runner-and-json-reporter.md`
- `docs/current-state.md`
