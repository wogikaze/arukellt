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

Example:

```ark
use std::host::stdio
```

### Layer C (Component)

Layer C is the external component-boundary import surface.

- Planned keyword: `import`
- Planned identifier space: WIT/component identifiers (for example `namespace:package/interface@version`)
- Purpose: declare component-boundary dependencies, not source-module paths.

Layer C is not the same namespace as Layer S and must not be interpreted as `::` module resolution.

## 2. Current Behavior (Implemented)

As of current implementation and docs contract:

- `use` is the canonical source import mechanism for Layer S.
- `import` is reserved by ADR-009 for future Layer C component-boundary use.
- Legacy `import <single_identifier>` still exists in current parser behavior for file-level source import compatibility.
  This legacy source usage is transitional and is not Layer C.

Current behavior source of truth:

- `docs/current-state.md`
- `docs/adr/ADR-009-import-syntax.md`

## 3. Planned / Deferred Behavior (Not Yet Implemented)

The following is planned/deferred and not yet implemented as a stable source feature:

- Layer C source syntax for component-boundary imports (for example string-literal `import` forms targeting WIT identifiers).
- Full remapping of legacy source `import <single_identifier>` to `use`-only source imports.

Until those changes land, do not treat `import` as active component-boundary syntax in user code.

## 4. Non-Goals

This contract does not redefine Arukellt source module paths to WIT package syntax.

- `std::...` (Layer S) is not rewritten into `namespace:package/...`.
- WIT package identifiers belong to Layer C boundary contracts.

## 5. Cross References

- ADR decision: [../adr/ADR-009-import-syntax.md](../adr/ADR-009-import-syntax.md)
- Draft elaboration (paths vs WIT IDs, collisions, migration): [../adr/ADR-025-use-paths-vs-wit-package-identifiers.md](../adr/ADR-025-use-paths-vs-wit-package-identifiers.md)
- Current behavior contract: [../current-state.md](../current-state.md)
- Language reference import section: [../language/spec.md](../language/spec.md)
