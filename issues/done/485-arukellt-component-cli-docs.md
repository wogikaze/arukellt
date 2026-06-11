---
Status: done
Created: 2026-04-03
Updated: 2026-06-10
ID: 485
Track: docs
---

# docs: arukellt component サブコマンド CLI リファレンス

## Summary

Created `docs/cli-reference.md` documenting the `ark component` subcommand
with its three sub-subcommands (`build`, `inspect`, `validate`), options,
and usage examples. The doc covers the CLI from the selfhost compiler
perspective, noting that `inspect` and `validate` require `wasm-tools`
for full functionality.

## Resolution

Issue 475 (CLI implementation) is closed. The CLI reference docs have been
added to `docs/cli-reference.md` with comprehensive subcommand documentation.
