# Import System Contract

Status: normative for language/docs alignment in issue #123 acceptance slice.

This page formalizes the import model as two distinct layers.

- Layer S (Source): source-module imports inside Arukellt code.
- Layer C (Component): component-boundary imports for WIT/component integration.

The two layers are intentionally separate.

## 1. Layer Model

### Layer S (Source)

Layer S is the source-language module import surface.

- Canonical form: `use path::to::module`
- Path separator: `::`
- Purpose: resolve Arukellt modules (stdlib and source modules) during source compilation.
- `as` gives the imported module an explicit local alias.
- `use path::to::module::{item, item2}` selects public items from one module.
- `pub use` re-exports selected public items or all public items selected by a module path.
- Layer S does not use WIT package identifiers as module paths.
- `std::...` names remain source-level Arukellt imports, not WIT boundary IDs.

Example:

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
use std::host::stdio
```

### Layer C (Component)

Layer C is the external component-boundary import surface.

- Planned keyword: `import`
- Planned identifier space: WIT/component identifiers (for example `namespace:package/interface@version`)
- Purpose: declare component-boundary dependencies, not source-module paths.
- WIT identifiers belong to the component boundary contract, not to Layer S path resolution.

Layer C is not the same namespace as Layer S and must not be interpreted as `::` module resolution.

## 2. Current Behavior (Implemented)

As of current implementation and docs contract:

- `use` is the canonical source import mechanism for Layer S.
- `use path as alias` resolves qualified access through `alias` while retaining the
  canonical module path for lookup.
- A selective `use` loads the containing module and creates aliases for the selected
  items. An item alias is not a second module import.
- `pub use` exposes only symbols that are public in the source module; a private
  source symbol cannot become public through a facade.
- Legacy `import <identifier> [as <alias>]` remains accepted for source-module
  migration. It follows the same local module and alias resolution as `use`, and
  emits `W0101` at parse phase. New source should use `use`.
- String-literal `import` remains reserved for the Layer C/WIT boundary and is
  specified separately by issue #867.

Current behavior source of truth:

- `docs/current-state.md`
- `docs/adr/ADR-009-import-syntax.md`

## 3. Planned / Deferred Behavior (Not Yet Implemented)

The following is planned/deferred and not yet implemented as a stable source feature:

- Layer C source syntax for component-boundary imports (for example string-literal `import` forms targeting WIT identifiers).
- Removal of the compatibility form after the migration window defined by ADR-031.
- Source-level remapping of Arukellt module names to WIT package identifiers such as `arukellt:std/io`.

Until those changes land, do not treat `import` as active component-boundary syntax in user code.

## 4. Non-Goals

This contract does not redefine Arukellt source module paths to WIT package syntax.

- `std::...` (Layer S) is not rewritten into `namespace:package/...`.
- WIT package identifiers belong to Layer C boundary contracts.
- The spec does not make WIT package identifiers source-module paths.

## 5. Cross References

- ADR decision: [../adr/ADR-009-import-syntax.md](../adr/ADR-009-import-syntax.md)
- Draft elaboration (paths vs WIT IDs, collisions, migration): [../adr/ADR-025-use-paths-vs-wit-package-identifiers.md](../adr/ADR-025-use-paths-vs-wit-package-identifiers.md)
- Current behavior contract: [../current-state.md](../current-state.md)
- Language reference import section: [../language/spec.md](../language/spec.md)
