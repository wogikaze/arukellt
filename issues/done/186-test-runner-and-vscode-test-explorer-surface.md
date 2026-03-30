# Test UX: `arukellt test` と VS Code Test Explorer surface

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 186
**Depends on**: 196, 197, 198
**Track**: parallel
**Blocks v1 exit**: no

**Status note**: Parent issue for test discovery/runner, VS Code test integration, and advanced test UX.

## Summary

Arukellt の test UX は、CLI runner の整備、VS Code Test Explorer 連携、高度な snapshot / impact / fuzz UX で責務が分かれる。
拡張側の見た目だけ先に作らず、CLI / runner / IDE integration / advanced UX を child issue で分けて追跡する。

## Acceptance

- [x] #196, #197, #198 が完了している
- [x] runner core / VS Code test integration / advanced test UX の責務が child issue に分解されている
- [x] test UX の残課題が issue queue 上で追跡できる

## References

- `issues/open/196-arukellt-test-discovery-runner-and-json-reporter.md`
- `issues/open/197-vscode-test-explorer-and-inline-test-execution.md`
- `issues/open/198-advanced-test-ux-impact-analysis-snapshot-diff-and-fuzz-ui.md`
- `issues/done/056-std-test.md`
- `docs/cookbook/testing-patterns.md`
