---
Status: done
Created: 2026-03-31
Updated: 2026-06-12
ID: 333
Track: lsp-navigation
Depends on: —
Orchestration class: done
Blocks v1 exit: no
Priority: 1
---

## Completed — 2026-06-12

Selfhost LSP under `src/compiler/lsp/` now builds and searches a project-wide symbol index.

**Evidence:**
- `initialize` loads stdlib manifest symbols and walks the project import graph from `ark.toml` entry
- `did_open` / `did_change` / `did_close` / `did_change_watched_files` update the index
- `workspace/symbol` returns symbols outside the open file (`tests/fixtures/selfhost/lsp_symbol_index.lsp-script`)
- `python3 scripts/check/check-lsp-lifecycle.py` — 3/3 pass
- `python3 scripts/manager.py verify quick` — 150/150 pass

**Emitter workarounds (selfhost s2):** index vecs live flattened on `LspState`; stdlib+project loaders are inlined (no combined facade); workspace search scans project symbols only (stdlib loaded at init).

# LSP: project-wide symbol index を構築する

## Summary

LSP の workspace 全体の symbol index を selfhost 実装に追加する。`project_root` を `initialize` で解決し、import graph と stdlib manifest から symbol を索引し、`workspace/symbol` で検索できるようにする。

## Acceptance

- [x] `project_root` 配下の全 `.ark` ファイルを起動時に index する仕組みが動作する
- [x] index が top-level symbol (fn, struct, enum, trait, impl) を file:span 付きで保持する
- [x] `did_open` / `did_change` / `did_change_watched_files` で index が差分更新される
- [x] index が stdlib module の公開 symbol も含む (#334 の前提)
- [x] `workspace/symbol` が index を検索し、open file 以外の結果も返す

## References

- `src/compiler/lsp/symbol_index_*.ark` — index records, project discovery, search, stdlib manifest
- `src/compiler/lsp/init_workspace.ark` — initialize-time index build
- `src/compiler/lsp/feature_workspace_symbol.ark` — `workspace/symbol` handler
- `tests/fixtures/selfhost/lsp_symbol_index.lsp-script`
