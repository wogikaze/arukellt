# LSP: project-wide symbol index を構築する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 333
**Depends on**: —
**Track**: lsp-navigation
**Blocks v1 exit**: no
**Priority**: 1

## Summary

LSP の analysis_cache を単一ファイルの HashMap<Url, CachedAnalysis> から、workspace 全体の symbol index を持つ構造に拡張する。現在 project_root と workspace_roots は initialize 時に解決・保持されているが、どの handler もこれを参照せず、全機能が single-file mode で動作している。

## Current state

- `crates/ark-lsp/src/server.rs` (3,429 行): `analysis_cache: Mutex<HashMap<Url, CachedAnalysis>>` が per-URI
- `project_root` / `workspace_roots` は `initialize()` で設定されるが handler から参照されない
- `analyze_source()` は lex → parse → resolve → typecheck を 1 ファイルに対して実行するが、他ファイルを読み込まない
- `goto_definition`, `references`, `rename` 等すべてが current file の AST / token のみ走査

## Acceptance

- [x] `project_root` 配下の全 `.ark` ファイルを起動時に index する仕組みが動作する
- [x] index が top-level symbol (fn, struct, enum, trait, impl) を file:span 付きで保持する
- [x] `did_open` / `did_change` / `did_change_watched_files` で index が差分更新される
- [x] index が stdlib module の公開 symbol も含む (#334 の前提)
- [x] `workspace/symbol` が index を検索し、open file 以外の結果も返す

## References

- `crates/ark-lsp/src/server.rs:121-154` — `analyze_source()` 単一ファイル解析
- `crates/ark-lsp/src/server.rs:1966-2009` — `initialize()` で `project_root` 設定
- `crates/ark-lsp/src/server.rs:2428-2446` — `workspace/symbol` が open file のみ検索
