# ADR-026: Arukellt source imports vs WIT package syntax — decision record

ステータス: **DECIDED** — source imports stay Layer S (`use` + `::`); WIT identifiers stay Layer C boundary data
**Date**: 2026-04-18
**Track**: language-design (issue [#123](../../issues/open/123-import-syntax-unification.md))

## Context / problem

Arukellt currently exposes two different “import-like” surfaces:

| Surface | Example | Role |
|---------|---------|------|
| Source modules (Layer S) | `use std::io` | Resolve `.ark` modules and stdlib; path segments use `::`. |
| WIT / Component packages (Layer C) | `wasi:cli/stdin@0.2.10` | Package and interface identifiers at the Component Model boundary; use `:` between namespace and package, `/` before interface, `@` for versions. |

They do not collide lexically today because WIT text normally lives in `.wit` files, manifests, and tooling — not in ordinary `use` paths. As Layer C declarations move closer to source (issue [#123](../../issues/open/123-import-syntax-unification.md), [#124](../../issues/open/124-wit-component-import-syntax.md)), the risk shifts to **conceptual** confusion (stdlib vs WASI) and to **future grammar** choices, not to accidental parsing of `std::` as WIT.

Normative split for current behavior: [ADR-009-import-syntax.md](ADR-009-import-syntax.md) and [../spec/import-system.md](../spec/import-system.md).

## Options (summary)

| Option | Idea | Tradeoff sketch |
|--------|------|-----------------|
| **A — Two layers, distinct syntax (status quo + docs)** | Keep `use` + `::` for Layer S; express WIT IDs via strings, attributes, and/or external `.wit` + CLI (`--wit`). | Matches common toolchains; no mass migration of fixtures; teaching cost is “two concepts”. **Aligned with ADR-009.** |
| **B — WIT-shaped paths in source** | Replace or mirror `::` paths with `namespace:pkg/interface@ver` for stdlib and user code. | Theoretically one format; **large breaking change** and poor fit for ordinary language modules (see ADR-009 “Alternatives Considered”). |
| **C — Dedicated keyword / attribute** | e.g. `wit import "…"` or `#[wit_import("…")]` for Layer C only. | Clear disambiguation; extra surface area; overlaps option A delivery paths. |
| **D — Unify `import` / `use` keywords** | Retire `import <single-id>` for files in favor of `use`, reserve `import` for Layer C (ADR-009 direction). | Reduces “two import keywords” confusion; requires a deprecation timeline and implementation work outside this ADR. |

Elaboration, collision notes, and syntax sketches: [ADR-025-use-paths-vs-wit-package-identifiers.md](ADR-025-use-paths-vs-wit-package-identifiers.md).

## Decision record

This ADR records the same decision as ADR-009 and does not reopen the layer split:

- Layer S remains the source-language import surface: `use path::to::module`
- Layer C remains the component-boundary surface for WIT identifiers
- No silent mapping is defined between `std::…` source paths and `wasi:…` identifiers

The deferred implementation work is intentionally separate:

- source-level syntax for Layer C imports
- parser / lowering support for any `import "…"` or attribute-based bridge
- migration away from legacy `import <single_identifier>` source imports

Those items stay tracked with issue [#124](../../issues/open/124-wit-component-import-syntax.md) and implementation work, not with this record.

## WIT-related paths in this repository

These are the primary **in-repo** places that show WIT package / world syntax and how the toolchain consumes it:

| Location | What it illustrates |
|----------|----------------------|
| [../spec/import-system.md](../spec/import-system.md) | Layer S vs Layer C contract; how docs name `use` vs WIT IDs. |
| [../../tests/fixtures/component/](../../tests/fixtures/component/) | Checked-in `.wit` fixtures (`package …;`, `world`, `import` / `export`) used by component compile tests. Examples: `export_record.wit`, `world_command.expected.wit`, `import_flags_type.wit`. |
| [../../tests/component-interop/jco/enum-colors/colors.wit](../../tests/component-interop/jco/enum-colors/colors.wit) | Interop sample WIT. |

A full vendored copy of the upstream WIT grammar / WASI snapshot is **not** tracked under `docs/spec/` at present; normative grammar and ecosystem definitions remain in the WebAssembly Component Model and WIT specifications upstream.

## Related

- [ADR-009-import-syntax.md](ADR-009-import-syntax.md) — **DECIDED** layer split and `import` reservation.
- [ADR-025-use-paths-vs-wit-package-identifiers.md](ADR-025-use-paths-vs-wit-package-identifiers.md) — draft exploration and collision policy.
- [ADR-006-abi-policy.md](ADR-006-abi-policy.md) — ABI layers; does not require source syntax to equal WIT text.
- [../module-resolution.md](../module-resolution.md) — Layer S resolution for `import` / `use`.
