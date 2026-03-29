# `arukellt test` discovery / runner / JSON reporter

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 196
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: no

## Summary

テスト宣言ルール、discovery、package / file / single-test execution、machine-readable reporter を `arukellt test` の CLI surface として定義する。VS Code 側の test UI が依存する中核 child issue。

## Acceptance

- [ ] test discovery / execution ルールが追跡できる
- [ ] `arukellt test` command surface と JSON reporter が定義されている
- [ ] test result / location / duration reporting の責務が issue queue 上で追跡できる

## References

- `issues/open/186-test-runner-and-vscode-test-explorer-surface.md`
- `issues/done/056-std-test.md`
- `docs/cookbook/testing-patterns.md`
