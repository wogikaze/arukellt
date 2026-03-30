# `script run` / `script list` CLI surface

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 203
**Depends on**: 202
**Track**: parallel
**Blocks v1 exit**: no

## Summary

`[scripts]` schema、named execution、args passthrough、JSON listing、stable exit behavior を `arukellt script run` / `script list` の CLI surface として定義する。manifest schema と project discovery の上に載る child issue。

## Acceptance

- [x] `script run` / `script list` の command surface が追跡できる
- [x] args / env / cwd / exit behavior の責務が定義されている
- [x] machine-readable script listing を issue queue 上で追跡できる

## References

- `issues/open/188-ark-toml-project-workspace-and-scripts.md`
- `issues/open/202-ark-toml-schema-and-project-workspace-discovery.md`
- `crates/arukellt/src/main.rs`
