# Module, Package, Workspace, and Dependency Resolution

> **Status**: Provisional (v1 scope). Workspace and registry dependency features
> are scaffolded but not fully implemented.
>
> This document is the authoritative specification for how Arukellt resolves
> modules, packages, workspaces, and dependencies. The implementation lives in
> `crates/ark-manifest/src/lib.rs` and `crates/ark-resolve/src/lib.rs`.

## 1. Concepts

| Term | Definition |
|------|-----------|
| **Module** | A single `.ark` source file. Each file is exactly one module. |
| **Package** | A directory containing `ark.toml`. The manifest defines the package boundary. |
| **Workspace** | A set of packages managed together. Currently single-package only (v1). |
| **Standard library** | The `std/` tree with entries in `std/manifest.toml`. Always available via `use`. |

## 2. Module Resolution

### 2.1 File ↔ module name mapping

Each `.ark` file is a module whose name is derived from its filename:

| File path | Module name |
|-----------|------------|
| `src/utils.ark` | `utils` |
| `src/math/trig.ark` | `math::trig` (subdirectory path) |
| `src/main.ark` | entry point (no module name needed) |

The `[bin]` section of `ark.toml` identifies the entry-point file.
All other `.ark` files in the source tree are importable as sibling modules.

### 2.2 `import` resolution order

`import math` resolves as follows (in order, first match wins):

1. **Sibling file** — look for `<current-file-dir>/math.ark`
2. **Sibling directory** — look for `<current-file-dir>/math/mod.ark`
3. **Error** — no match → compile error E0105 (unresolved import)

`import` is a v0 keyword. As of v1, prefer `use` for all standard library
imports. `import` will be reserved for Component Model boundary imports in v4+
(see ADR-009).

### 2.3 `use` resolution order

`use std::host::stdio` resolves as follows:

1. **Standard library** — look up the path in `std/manifest.toml` module table
2. **Error** — no match → compile error E0105 (unresolved import)

`use` is the preferred import keyword for stdlib access.
The standard library module tree is defined by `std/manifest.toml`.

### 2.4 Qualified access

After importing, items are accessed via their module alias:

```ark
import math
let x = math::add(1, 2)

use std::host::stdio
stdio::println("hello")
```

A module alias can be renamed at import time:

```ark
import math as m
let x = m::add(1, 2)
```

## 3. Package Boundary

A **package** is a directory that contains an `ark.toml` file.

### 3.1 Package discovery algorithm

Given a starting directory `start_dir` (typically the current working directory):

```
1. If start_dir/ark.toml exists → this directory is the package root
2. Pop one directory level (parent)
3. Repeat until filesystem root
4. If no ark.toml found → single-file mode (no package)
```

This is the algorithm in `Manifest::find_root(start_dir)` and
`Manifest::find_and_load(start_dir)` in `crates/ark-manifest/src/lib.rs`.

**CLI and LSP use the same algorithm.** Since `ark-lsp` now depends on
`ark-manifest`, both tools call `Manifest::find_root` with the same starting
directory, guaranteeing identical results (implemented in #238).

### 3.2 Single-file mode

If no `ark.toml` is found in the directory tree:

- `arukellt compile <file>` operates on the single file only
- `arukellt run <file>` operates on the single file only
- LSP operates on the single file, with prelude-only standard library access
- No `[targets]`, `[scripts]`, or dependency resolution is performed

### 3.3 Package root contents

A package root is expected to contain:

```
my-project/
  ark.toml       ← package manifest
  src/
    main.ark     ← entry point (path specified in [bin])
    utils.ark    ← importable as `import utils`
```

## 4. Workspace Discovery

> **Status**: Single-package workspaces only in v1. Multi-package workspace
> support is planned for v2.

In v1, each `ark.toml` defines exactly one package. There is no `[workspace]`
section or workspace member enumeration.

The workspace discovery algorithm for future multi-package support will be:

```
1. Find the uppermost ark.toml in the directory tree
2. If it contains [workspace], treat it as the workspace root
3. Enumerate workspace.members = ["packages/a", "packages/b", ...]
4. Load each member's ark.toml as a sub-package
```

This design mirrors Cargo's workspace model. Implementation target: v2.

## 5. Dependency Resolution

### 5.1 Dependency specification in ark.toml

Dependencies are declared in the `[dependencies]` table:

```toml
[dependencies]
my-lib = { path = "../my-lib" }    # local path dependency (v1)
some-pkg = "1.2.3"                 # registry version (future)
```

### 5.2 Resolution priority (planned)

When the same package name appears in multiple sources, resolution priority is:

1. **Local path** (`path = "..."`) — highest priority
2. **Workspace member** (`workspace = true`) — shared version within workspace
3. **Registry** (`"1.2.3"`) — lowest priority

In v1, only local path dependencies are supported. Registry support requires
a package registry, which is not yet implemented.

### 5.3 Local path resolution

A path dependency `{ path = "../my-lib" }` is resolved relative to the
**package root** (the directory containing the declaring `ark.toml`).

The dependency's `ark.toml` is loaded from the resolved path. Its `[bin]`
entry point becomes the importable module.

### 5.4 Circular dependency detection

Circular path dependencies are detected at load time. A circular dependency
graph produces compile error E0108 (circular dependency).

## 6. Standard Library Resolution

The standard library is always available and does not require a dependency
declaration.

### 6.1 Stdlib structure

The standard library lives in `std/` and is registered in `std/manifest.toml`.
Each module in `std/manifest.toml` is accessible via `use std::<module>`.

### 6.2 Prelude injection

The following items are injected into every module without an explicit import
(defined by `prelude = true` in `std/manifest.toml`):

**Types**: `Option`, `Result`, `String`, `Vec`
**Constructors**: `Some`, `None`, `Ok`, `Err`
**Literals**: `true`, `false`

All functions with `prelude = true` in `std/manifest.toml` are also available
without import.

### 6.3 Stdlib path normalization

`use std::host::stdio` is normalized to a module lookup: the `::` separators
are split into path components, and the last component is the module name.
The module's exported functions become accessible via the alias `stdio::`.

## 7. Visibility Rules

> **Status**: Provisional — visibility checking is partially implemented.
> See issue #234.

- Items are **private by default** (visible only within the defining module)
- `pub` makes an item visible to any importing module
- `pub(crate)` is reserved syntax but not enforced in v1

**Import re-export**: A `pub use` or `pub import` makes the imported module's
public items re-exported (not yet implemented; tracked in #234).

## 8. Resolution in the Extension and LSP

The VS Code extension and LSP server use the same `Manifest::find_root`
algorithm as the CLI. When the editor opens a file:

1. The LSP calls `Manifest::find_root(workspace_folder)` on initialize
2. If found, the project root governs which `ark.toml` is in effect
3. If not found, the LSP operates in single-file mode
4. When `ark.toml` changes on disk, the LSP re-calls `find_root` to pick up
   the change (via `didChangeWatchedFiles`)

This ensures that `arukellt build` in the terminal and LSP diagnostics always
agree on which project is active.

## 9. Error Codes

| Code | Name | Trigger |
|------|------|---------|
| E0105 | UnresolvedImport | `import foo` and no `foo.ark` found |
| E0108 | CircularDependency | Path dependency cycle detected |
| E0109 | ManifestNotFound | `arukellt build` with no `ark.toml` in tree |
| E0110 | ManifestParseError | `ark.toml` has a syntax error |
| E0111 | MissingBinSection | `ark.toml` has no `[bin]` section |

## 10. Open Work

| Issue | Topic | Status |
|-------|-------|--------|
| #234 | Visibility enforcement as compiler error | Open |
| #235 | Multi-root workspace tool layer unification | Open |
| v2 | Multi-package workspace support | Planned |
| v2 | Registry dependency resolution | Planned |
| v4 | Component Model boundary imports (`import` keyword v4) | Planned |
