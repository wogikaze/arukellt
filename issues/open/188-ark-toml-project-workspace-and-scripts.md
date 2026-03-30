# `ark.toml`: project / workspace metadata と `script run` surface

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 188
**Depends on**: 202, 203, 204
**Track**: parallel
**Blocks v1 exit**: no

**Status note**: Parent issue for manifest schema, script CLI surface, and project explain/inspection features.

## Summary

`ark.toml` 系の責務は、manifest schema と root discovery、`script run` / `script list` の CLI surface、project understanding / explain 系 DX に分かれる。
WIT import 用の最小 manifest 構想 (#124) を踏まえつつ、IDE と CLI が共有する project surface を child issue に分解して追跡する。

## Acceptance

- [x] #202, #203 が完了している
- [x] manifest schema / script CLI の責務が完了している
- [ ] project explain-inspection (#204) および残課題が issue queue 上で追跡できる

## References

- `issues/open/124-wit-component-import-syntax.md`
- `issues/open/202-ark-toml-schema-and-project-workspace-discovery.md`
- `issues/open/203-script-run-and-script-list-cli-surface.md`
- `issues/open/204-project-explain-build-explain-and-script-sandbox-surface.md`
- `crates/arukellt/src/main.rs`
