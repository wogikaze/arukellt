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

- Keyword: string-literal `import`
- Identifier space: WIT/component identifiers (`namespace:package/interface[@version]`)
- Purpose: declare component-boundary dependencies, not source-module paths.
- WIT identifiers belong to the component boundary contract, not to Layer S path resolution.
- A full version, when present, must exactly match the loaded WIT document. An
  omitted version is a source shorthand that resolves to the loaded document's
  version.

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
- String-literal `import` is implemented for the Layer C/WIT boundary. Its
  canonical identity is rendered as `namespace:package/interface@full-version`
  when the WIT document has a version. Core Wasm ABI renderings such as a
  major/minor suffix are target-specific and do not change this language identity.
- `pub` controls Layer S source visibility only. A Component export requires
  explicit `export fn`; `pub export fn` satisfies both contracts.

Current behavior source of truth:

- `docs/current-state.md`
- `docs/adr/ADR-009-import-syntax.md`

## 3. Planned / Deferred Behavior

The following remains outside this contract:

- Removal of the compatibility form after the migration window defined by ADR-031.
- Source-level remapping of Arukellt module names to WIT package identifiers such as `arukellt:std/io`.
- Inline world declarations and other Component surfaces not covered by `export fn`.

## 4. Non-Goals

This contract does not redefine Arukellt source module paths to WIT package syntax.

- `std::...` (Layer S) is not rewritten into `namespace:package/...`.
- WIT package identifiers belong to Layer C boundary contracts.
- The spec does not make WIT package identifiers source-module paths.

## 5. Cross References

- ADR decision: [../adr/ADR-009-import-syntax.md](../adr/ADR-009-import-syntax.md)
- Export and identity refinement: [../adr/ADR-071-component-export-visibility-boundary.md](../adr/ADR-071-component-export-visibility-boundary.md)
- Draft elaboration (paths vs WIT IDs, collisions, migration): [../adr/ADR-025-use-paths-vs-wit-package-identifiers.md](../adr/ADR-025-use-paths-vs-wit-package-identifiers.md)
- Current behavior contract: [../current-state.md](../current-state.md)
- Language reference import section: [../language/spec.md](../language/spec.md)
