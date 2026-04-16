# ADR-025: Source module paths vs WIT package identifiers — collision policy and syntax exploration

**Status**: Proposed (draft)
**Date**: 2026-04-16
**Track**: language-design (issue #123)

## Context

Arukellt today has two different naming surfaces that both look like “imports” to newcomers and to tooling:

| Surface | Example | Role |
|---------|---------|------|
| Layer S — source modules | `use std::host::stdio` | Resolve `.ark` modules and stdlib during compilation (`::` paths). |
| Layer C — component / WIT | `wasi:cli/stdin@0.2.10` | Identify packages and interfaces at the Component Model binary boundary. |

ADR-009 **decided** to keep these as **separate layers** with different syntax and keywords (`use` vs planned `import` for Layer C). This ADR does **not** reopen that split; it records **additional design candidates**, **collision-avoidance tactics**, a **non-binding syntax sketch** for future Layer C source forms, and **migration notes** so issue #123 has a single elaborated artifact beyond ADR-009.

## Decision candidates (summary)

### Candidate A — Single format everywhere (`namespace:pkg/interface@ver` in source)

- **Idea**: Replace `::` source paths with WIT-style package IDs for stdlib and user code (e.g. `use arukellt:std/io`).
- **Pros**: One mental model; close alignment with emitted component metadata.
- **Cons**: Breaking change at massive scale; WIT IDs encode registry/organization semantics ill-suited to normal language modules; poor ergonomics for self-hosting and everyday code (see ADR-009 “Alternatives Considered A”).
- **Verdict in this draft**: **Rejected** as a default; kept only as an explicit non-goal anchor.

### Candidate B — Two layers, distinct syntax (ADR-009 default)

- **Idea**: Layer S stays `use` + `::` paths; Layer C uses WIT strings / dedicated forms and never reuses `::` resolution rules.
- **Pros**: Zero collision between path grammar and WIT grammar; matches common industry pattern (source imports vs external WIT tooling); preserves existing fixtures and stdlib layout.
- **Cons**: Two concepts to teach; documentation must be explicit (this ADR + `docs/spec/import-system.md`).
- **Verdict in this draft**: **Recommended default** — consistent with ADR-009 and current repo contract.

### Candidate C — `wit` keyword or attribute bridge

- **Idea**: `wit import "wasi:cli/stdin@0.2.10"` or `#[wit_import("…")]` on modules/items.
- **Pros**: Visually unambiguous; optional if only build manifests carry WIT.
- **Cons**: Extra surface area; ADR-009 chose to reserve bare `import` for Layer C instead of a compound keyword (partially considered there).
- **Verdict**: **Optional variant** if implementers want stronger disambiguation than a string-literal `import`; not required for the two-layer default.

### Candidate D — WIT only outside source

- **Idea**: No WIT text in `.ark`; worlds/interfaces live in `.wit` and CLI flags (e.g. `--wit`), bindings generated or implied.
- **Pros**: Minimal parser complexity; matches “WIT as toolchain input” workflows.
- **Cons**: Less convenient for “single file defines component” examples; still need a story for generated symbol visibility into Layer S.
- **Verdict**: **Valid delivery path** for early phases; compatible with Candidate B (Layer S unchanged).

## Namespace collision avoidance

1. **Lexical shape**
   - Layer S paths use **`::`** and (today) identifier segments; they do not use WIT’s **`ns:pkg`** colon pairing or **`/`** between package and interface in the same token stream pattern as WIT.
   - WIT package IDs use **`:`** (namespace delimiter), **`/`** (interface), **`@`** (version), and optionally **`.{…}`** for symbol lists in WIT files — not Arukellt expression syntax.

2. **Keyword separation**
   - **`use`** — only Layer S in normative docs.
   - **`import`** — legacy file import today; reserved for Layer C per ADR-009 once `import <id>` is retired from source modules.

3. **Planned Layer C source forms**
   - Prefer **string literals** carrying the full WIT package/interface string (see sketch below) so the lexer never interprets `wasi:cli` as a path of identifiers.
   - Avoid bare `import wasi:cli/...` without quoting until a dedicated grammar is specified; unquoted forms invite parser ambiguity with paths, generics, or future operators.

4. **Conceptual collision (not lexical)**
   - **`std::io` is not a WIT package ID** and must not be documented as interchangeable with `wasi:io/…` without an explicit bridge (stdlib host facades vs raw WASI imports).
   - Tooling SHOULD NOT silently map between layers; any mapping belongs in the compiler / manifest / binding generator with explicit configuration.

5. **Reserve org namespaces in WIT only**
   - Follow WIT ecosystem convention: organization-owned namespaces (`wasi:`, vendor-specific prefixes). Arukellt language modules remain ordinary paths under `std::`, package roots, etc.

## Non-binding syntax sketch (Layer C in source)

> **Non-normative.** Illustrative only; parser, keyword placement, and attribute forms are subject to future ADR/issue decisions.

<!-- skip-doc-check -->
```ark
// String form: WIT package + interface + optional version inside quotes.
import "wasi:cli/stdin@0.2.10"

// Possible future: named binding for generated surface (spelling TBD).
// import "wasi:cli/stdin@0.2.10" as cli_stdin

// Layer S unchanged: ordinary module import.
use std::host::stdio
```

Bindings produced from Layer C would then appear as ordinary imported modules/types in Layer S (exact name resolution rules TBD in implementation issues).

## Migration and compatibility

| Phase | Layer S | Layer C / `import` keyword |
|-------|---------|-----------------------------|
| Current | `use` + legacy `import foo` for sibling modules | WIT IDs appear in `.wit` / tooling; not yet first-class `import "…"` syntax in user `.ark` per `docs/current-state.md` |
| v4 (planned) | Deprecate then remove `import <single_identifier>` in favor of `use` (ADR-009 timeline) | Repurpose `import` for Layer C declarations |
| Ecosystem | Existing fixtures keep `::` paths | New syntax additive behind design in ADR-009 + implementation |

**Compatibility principles**

- No automatic rewrite of `use std::…` into WIT strings.
- Teach Layer S vs Layer C in docs (`docs/spec/import-system.md`) to prevent LLM/user conflation of `std::io` with `wasi:io/…`.

## Recommended default (this draft)

Adopt **Candidate B** as the continuing default: **do not unify** Arukellt source paths with WIT package identifier grammar; keep strict layer separation and use the collision-avoidance tactics above. Layer C surface may evolve (Candidate C/D) without changing this default.

## Relationship to other ADRs

- **ADR-009**: Normative **DECIDED** split (`use` vs reserved `import` for WIT). This ADR is a **draft elaboration** for issue #123; it defers to ADR-009 where they overlap.
- **ADR-006**: ABI layers — source semantics vs WIT ABI remain distinct.
- **ADR-023**: Registry resolution applies to Layer S dependencies, not to rewriting module paths into WIT IDs.

## Related

- [ADR-009-import-syntax.md](ADR-009-import-syntax.md)
- [../spec/import-system.md](../spec/import-system.md)
- Issue #123 — import syntax and WIT package identifier unification policy
- Issue #124 — WIT component import / `--wit` wiring
