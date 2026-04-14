# LSP: Full Multi-Root Workspace and Cross-Package Resolution

**Status**: open
**Created**: 2026-04-14
**Updated**: 2026-04-14
**ID**: 502
**Depends on**: 441
**Track**: vscode-ide
**Blocks v1 exit**: no
**Priority**: 3

## Context

Issue 441 added the initial workspace/package scaffolding to the LSP server.
The following fields and handlers exist but do not yet implement actual
multi-package resolution:

- `workspace_roots: Mutex<Vec<PathBuf>>` — populated on init, never used post-init
- `did_change_watched_files` — rebuilds index but only for the single primary root
- `goto_definition` — cross-file within the primary root only; no cross-package lookup

This issue covers the unimplemented acceptance items that would require > 200 lines
of focused LSP work.

## Scope

### 1. Module graph from `ark.toml` dependencies

`Manifest` already parses `dependencies: HashMap<String, DependencySpec>`.  A new
helper in `ark-manifest` or `ark-lsp` must walk the dependency graph (handling
`DependencySpec::Path { path }` and potentially future registry deps) and return a
flat list of resolved package roots.

### 2. Multi-package workspace resolution

After initialization, the server should:
- Enumerate all `workspace_roots`
- For each root find its `ark.toml`, load the manifest, and walk the dependency graph
- Build a per-package symbol index for each discovered package root

### 3. Cross-package go-to-definition

`goto_definition` falls back to the symbol index.  Once the index covers all package
roots discovered in step 2, cross-package navigation will work without additional
request-level changes.

### 4. Package-aware import resolution

`refresh_diagnostics` uses only `project_root/std` as a stdlib root.  Cross-package
imports (e.g. `use my_lib::utils`) need the resolver to also search the indexed
package roots, passing their source directories alongside the stdlib root.

### 5. Index rebuild across all package roots on workspace changes

`did_change_watched_files` currently only calls `index_project_files(&root)` for the
primary root.  It must also trigger re-indexing of all dependent package roots when
any `ark.toml` in the workspace changes.

## Acceptance

- [ ] `ark.toml` dependency graph is walked and all package roots are discovered
- [ ] Symbol index covers all discovered packages, not just the primary root
- [ ] Cross-package go-to-definition resolves symbols from dependency packages
- [ ] Import resolution searches dependency package source directories
- [ ] Workspace changes (any `ark.toml` in the tree) rebuild the full multi-root index

## PRIMARY_PATHS

- `crates/ark-lsp/src/server.rs`
- `crates/ark-manifest/src/lib.rs`

## ALLOWED_ADJACENT_PATHS

- `tests/package-workspace/` (add or expand cross-package LSP fixtures)
- `extensions/arukellt-all-in-one/src/extension.js` (if extension changes needed)

## Implementation Notes

- `DependencySpec::Path { path }` is relative to the manifest directory; resolve with
  `manifest_dir.join(path)` before calling `Manifest::find_root`.
- Registry-based deps (`DependencySpec::Version`) are out of scope until the registry
  is implemented; skip them silently.
- Keep the graph traversal cycle-safe (a `HashSet<PathBuf>` of visited roots is enough
  for local deps).
- Estimated scope: 200–300 lines across `ark-manifest` and `ark-lsp`.

## References

- `crates/ark-manifest/src/lib.rs` — `DependencySpec` enum, `Manifest::find_root`
- `crates/ark-lsp/src/server.rs` — `workspace_roots`, `index_project_files`,
  `did_change_watched_files`, `goto_definition`
- `docs/ark-toml.md`
- Spawned from: `issues/done/441-vscode-project-aware-workspace-package-ark-toml.md`
