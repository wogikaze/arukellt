# ADR-031: Import Syntax and WIT Package Identifier Unification

**Status**: DECIDED — Two-layer separation confirmed; `use` reserved for Layer S, `import` reserved for Layer C
**Date**: 2026-04-25
**Track**: language-design
**Issue**: [#123](../../issues/open/123-import-syntax-unification.md)
**Supersedes**: none (refines and consolidates ADR-009, ADR-025, ADR-026)

---

## Context

Arukellt currently exposes two syntactically distinct "module reference" surfaces that risk
conflation as the language targets Component Model output:

| Surface | Example | Separator | Where it appears |
|---------|---------|-----------|-----------------|
| **Layer S — source import** | `use std::io` | `::` | `.ark` source files |
| **Layer S — local file import** | `import math` | (single identifier) | `.ark` source files |
| **Layer C — WIT package identifier** | `wasi:cli/stdin@0.2.10` | `:` `/` `@` | `.wit` files, CLI flags, manifests |

These surfaces do not collide lexically today because WIT text lives in `.wit` files and
tooling, not inside `use` paths. However, three problems emerge as Layer C declarations move
closer to source:

1. **Conceptual confusion** — LLMs and new contributors conflate `std::io` and `wasi:io/streams`
   as equivalent concepts; they are not (`std::io` is an Arukellt stdlib module path;
   `wasi:io/streams` is a WebAssembly Component Model package identifier).
2. **Undefined syntax** — no source-level syntax exists yet for referencing an external WIT
   interface from Arukellt source (needed for Component Model output, issue #124).
3. **Two `import` surfaces** — `import math` (local file) and `use std::io` (stdlib path) coexist
   with no clear rule for which to use, creating a "which keyword?" question for every new module.

### Current parser state (v3)

```
// crates/ark-parser/src/parser.rs
// TokenKind::ColonColon  -> path separator for `use`
// TokenKind::Import      -> `import foo` (single identifier; local file)
// TokenKind::Use         -> `use std::io::something` (:: separated path; stdlib)
```

Both `import` and `use` are live keywords with separate parse paths.

---

## WIT Package Identifier Syntax (reference)

Per the WebAssembly Component Model / WIT specification:

```wit
package wasi:clocks@0.2.10;            // namespace:name@version

interface monotonic-clock {
}

world imports {
    import monotonic-clock;
}
```

Structure: `namespace:package-name/interface-name@semver.{symbols}`

- `:` — namespace / package separator
- `/` — package / interface separator
- `@` — version
- `.{}` — symbol enumeration

---

## Options Considered

### Option A — Adopt WIT package identifier syntax wholesale

Replace Arukellt source `::` paths with the WIT `namespace:package/module` format for all imports.

```ark
use arukellt:std/io           // stdlib import (was: use std::io)
use wasi:cli/stdin            // WASI import
```

**Cons:**
- **Breaking change to all 409 existing test fixtures** — every `use std::` path must be rewritten
- `arukellt:std/io` is redundant and verbose (analogous to writing `rust:std/io` in Rust)
- WIT `namespace:package` is designed for organisational/registry identity, not intra-language
  module paths
- Self-hosting readability degrades: the compiler itself uses `use std::host::stdio`; switching to
  `arukellt:std/host/stdio` hurts clarity
- Standard library functions become `arukellt:std/io::writeln_stdout()` — high learning overhead

**Verdict**: **Rejected.** The WIT identifier format was designed for cross-organisation
component identity, not for referencing items within a single language's standard library.

### Option B — Define an Arukellt-native import syntax that maps to WIT IDs (two-layer split)

Keep `use` + `::` for Layer S (source-level module imports). Treat WIT identifiers as Layer C
boundary data expressed via strings, attributes, or external `.wit` + CLI flags.

```ark
use std::io                   // Layer S -- stdlib module (unchanged)
use std::host::fs             // Layer S -- host-bound stdlib module (unchanged)
// Layer C expressed outside source:
//   --wit my_interface.wit   (CLI flag, already accepted)
//   #[wit_import("wasi:cli/stdin@0.2.10")]  (future attribute form, v4+)
```

**Pros:**
- Zero breaking change to existing code (409 fixtures unaffected)
- Matches the approach taken by Rust, Go, Python, JavaScript, MoonBit (all separate source imports
  from WIT boundary processing)
- Visual distinction (`::` vs `:` + `/`) signals different abstraction layers to readers and tools
- Self-hosting compatibility: the Arukellt compiler is written in Arukellt and reads cleanly
- ADR-006 compliance: Layer 2A (raw Wasm ABI) and Layer 2B (WIT ABI) are already separated by
  design; source syntax does not need to mirror binary format

**Cons:**
- Two concepts ("Layer S" vs "Layer C") require documentation
- Inline Component Model declarations in source still need a syntax decision (deferred to v4)

**Verdict**: **Chosen** (see Decision section).

### Option C — `wit import` dedicated keyword

Introduce a compound keyword for Layer C imports alongside the existing `use`/`import` keywords:

```ark
use std::io                   // Layer S (unchanged)
wit import "wasi:cli/stdin"   // Layer C -- new compound keyword form
```

**Pros:**
- Fully explicit disambiguation of layers at the grammar level
- LLMs and IDE tooling can unambiguously classify the statement

**Cons:**
- New compound keyword surface area
- Largely redundant with Option B attribute/string form — both solve the same problem at v4

**Verdict**: Partially folded into the chosen direction. Option B's v4 delivery path
(`import "..."`) achieves the same disambiguation with a single keyword.

### Option D — Unify `import`/`use` source keywords; reserve `import` for Layer C

Deprecate `import <single-identifier>` (local file import) in favour of `use`, freeing the
`import` keyword for Layer C (WIT) declarations.

```ark
// v3 (current)
import math           // local file module
use std::io           // stdlib module

// v4 (deprecation)
use math              // W0101 warning: `import <id>` is deprecated; use `use <id>`
use std::io           // unchanged

import "wasi:cli/stdin@0.2.10"   // WIT package import via freed keyword
```

**Pros:**
- Eliminates "two import keywords" confusion for ordinary modules
- `import` keyword explicitly signals external/component boundary (distinct semantics)
- No structural disruption to `use` paths

**Cons:**
- Requires parser change and a deprecation diagnostic (W0101) in v4
- `import <single-id>` users must migrate; impact is limited to local-file module imports

**Verdict**: **Adopted as the v4 migration path** (combined with Option B). The `import` keyword
is reserved from v3 onwards for the Layer C surface.

---

## Decision

**Chosen direction: Option B + Option D combined.**

1. **`use path::to::module` confirmed as Layer S source import syntax** — no change to existing
   source files or fixtures.

2. **`import <single-identifier>` deprecated in v4, removed in v5** — local file imports migrate
   to `use <identifier>`. Deprecation diagnostic: W0101.

3. **`import` keyword reserved for Layer C (WIT/Component Model) in v4** — specific syntax TBD
   (string form `import "wasi:cli/stdin@0.2.10"` is the current candidate; see issue #124).

4. **WIT package identifiers remain Layer C boundary data** — they appear in `.wit` files,
   valid `use` path segments.

5. **Layer naming is canonical**:

The WIT `namespace:package/interface@version` format was designed for cross-organisation package
identity in the WebAssembly Component Model ecosystem. Applying it to intra-language standard
| Rust (cargo-component) | `use crate::...` (`::`) | `wit-bindgen` generates code; WIT does not appear in source |
| Go (WASI) | `import "path/to/pkg"` | WIT in external tooling |
| Python (componentize-py) | `import module` | WIT in external files |
| JavaScript (componentize-js) | ESM `import` | WIT in external `.wit` files |

In every case the source-level import syntax is unchanged and WIT is treated as a binary-boundary
tooling concern. Arukellt adopts the same separation.

The `import` keyword unification (Option D) is a narrow quality-of-life cleanup that reduces
beginner confusion ("should I write `import math` or `use math`?") without requiring any changes
to stdlib paths.

---

## Migration Impact

### Existing code (v3)

**No changes required.** All existing `use std::...` and `import <local>` source syntax continues
to compile and pass diagnostics in v3.

### v4 migration path

| Syntax | v4 behaviour | Affected scope |
|--------|-------------|----------------|
| `use std::io` | Unchanged, no warning | All stdlib / host imports |
| `use path::to::module` | Unchanged, no warning | All `use` path imports |
| `import math` (local file) | W0101 deprecation warning; still compiles | Local file module imports only |
| `import "wasi:cli/stdin"` | New Layer C syntax (gated on `--emit component`) | New code only |

Estimated fixture impact for `import <single-id>` to `use` migration: **localised**. The stdlib
uses `use` throughout; most `import <single-id>` patterns appear in specific test fixtures.

### v5

`import <single-identifier>` parse path removed. Any remaining occurrences are hard errors.

---

## Implementation Timeline

| Phase | Item | Tracking |
|-------|------|---------|
| v3 (immediate) | `docs/spec/import-system.md` documents Layer S / Layer C split | Done |
| v3 (immediate) | `--wit <path>` CLI flag accepted (binding generation deferred) | Done (issue #124 Phase 1) |
| v4 | W0101 deprecation warning for `import <single-identifier>` | issue #123 implementation work |
| v4 | `import "namespace:package/interface@ver"` syntax design + parser support | issue #124 |
| v4 | WIT-imported functions accessible via normal `use` in ARK source | issue #124 |
| v5 | Remove `import <single-identifier>` parse path | post-v4 |

---

## Consequences

- `use` is the stable, permanent Arukellt source module import keyword. It will not be
  redefined.
- `import` will serve as the Component Model / WIT boundary keyword from v4 onward. Its
  current `import <single-id>` semantics are a known migration target.
- Tooling (IDE, LLM prompts, documentation) should describe `use` and `import` as distinct
  layers, not synonyms.
- The `std::` path prefix is an Arukellt source namespace, not a WIT namespace. It is not
  equivalent to `wasi:` or `arukellt:` in WIT identity terms.

---

## Related

- [ADR-009-import-syntax.md](ADR-009-import-syntax.md) — primary decision record (DECIDED); this ADR consolidates and expands that decision with English prose, a full options table, and explicit migration impact.
- [ADR-025-use-paths-vs-wit-package-identifiers.md](ADR-025-use-paths-vs-wit-package-identifiers.md) — collision policy and syntax exploration (draft).
- [ADR-026-import-vs-wit-package-syntax.md](ADR-026-import-vs-wit-package-syntax.md) — decision record (DECIDED); same layer split.
- [ADR-006-abi-policy.md](ADR-006-abi-policy.md) — ABI layers; does not require source syntax to mirror WIT text.
- [ADR-007-targets.md](ADR-007-targets.md) — T3 (wasm32-wasi-p2) as primary target.
- [../spec/import-system.md](../spec/import-system.md) — normative Layer S / Layer C contract page.
- [../module-resolution.md](../module-resolution.md) — Layer S resolution behaviour for `use` / `import`.
- Issue [#074](../../issues/open/074-wasi-p2-native-component.md) — WASI p2 native component output.
- Issue [#124](../../issues/open/124-wit-component-import-syntax.md) — WIT component import syntax implementation.
