# VSCode Extension: Workspace / Package / ark.toml を理解した project-aware editor にする

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-18
**ID**: 441
**Depends on**: 333, 335, 340
**Track**: vscode-ide
**Blocks v1 exit**: no
**Priority**: 3

## Audit normalization — 2026-04-18

The 2026-04-14 audit note below captured the partial pre-`#502` state. `#502` is now
closed under `issues/done/502-lsp-full-multi-root-workspace.md`, and current repo
evidence shows the dependency-root discovery, multi-root indexing, cross-package
definition lookup, and package-aware diagnostics paths are present in
`crates/ark-lsp/src/server.rs` with regression coverage in
`crates/ark-lsp/tests/lsp_e2e.rs` and `tests/package-workspace/multi-root-indexing/`.

This issue remains `done`, but the historical partial-state note below should no
longer be read as the current repo truth.

## Historical audit snapshot — 2026-04-14

**Action**: Audited acceptance items against actual code; partial scaffolding confirmed.
Full multi-root work was carved out into `issues/done/502-lsp-full-multi-root-workspace.md`.

### What IS implemented (scaffolding)

- `ArukellBackend.workspace_roots: Mutex<Vec<PathBuf>>` is populated from
  `workspace_folders` on LSP initialization (`server.rs:3696–3740`).
- `ArukellBackend.project_root` is discovered via `Manifest::find_root` on init.
- `did_change_watched_files` handler reacts to `ark.toml` changes and rebuilds the
  symbol index for the primary project root (`server.rs:3862–3930`).
- Cross-file `goto_definition` falls back to the project-wide symbol index
  (`server.rs:4083–4176`).

### What is NOT implemented (deferred to #502)

- No module graph is built from `Manifest.dependencies` — dependencies are parsed
  but never traversed.
- `workspace_roots` is populated but never used after initialization; only
  `project_root` (first root) drives all subsequent LSP behavior.
- The symbol index covers the primary project root only; dependency packages are not
  indexed.
- Import resolution is not package-aware (`std_root` is the only resolved stdlib path).
- Index rebuild on workspace changes only applies to the primary root.

At the time of this historical note, all five acceptance checkboxes were marked `[x]`
prematurely; the scaffolding existed but the functional multi-package behavior did not.
That remaining work later landed in `issues/done/502-lsp-full-multi-root-workspace.md`.

## Summary

VSCode拡張を単なるファイル単位ツールから、ark.toml・workspace・package構成を理解した project-aware editor にする。import解決、package境界、multi-root workspace に対応する。

## Acceptance

- [x] `ark.toml` を元に module graph を構築する。 *(completed by #502; historical pre-landing note above)*
- [x] workspace 内複数 package を解決可能にする。 *(completed by #502; historical pre-landing note above)*
- [x] cross-package go-to-definition が動作する。 *(completed by #502; historical pre-landing note above)*
- [x] import 解決が package aware になる。 *(completed by #502; historical pre-landing note above)*
- [x] workspace 変更時に index が再構築される。 *(completed by #502; historical pre-landing note above)*

## References

- `crates/ark-manifest/src/lib.rs`
- `docs/ark-toml.md`
- `crates/ark-lsp/src/server.rs`
- `tests/package-workspace/`
- `issues/done/502-lsp-full-multi-root-workspace.md`
