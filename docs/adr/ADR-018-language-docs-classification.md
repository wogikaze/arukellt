# ADR-018: Language Docs Classification — Normative / Explanatory / Transitional

**Status**: DECIDED
**Created**: 2026-04-14
**Scope**: Language documentation (`docs/language/`), docs tooling

---

## Context

The `docs/language/` directory contains documents with mixed purposes:

- Some documents are authoritative specifications of implemented, fixture-backed behavior (e.g. `spec.md`, `syntax.md`).
- Some documents explain concepts and usage patterns without being the primary behavior contract.
- Some documents describe planned or in-progress changes that have not yet fully landed (e.g. `syntax-v1-preview.md`).

Without a formal classification system, readers cannot tell at a glance whether a document defines behavior, explains it, or describes work in progress. This leads to:

- Users treating transitional docs as authoritative.
- Contributors making inconsistent choices about whether to update a doc or create a new one.
- No clear lifecycle rule for when a transitional doc is retired or promoted.

ADR-014 established stability labels for individual spec *sections* and stdlib *entries*. This ADR establishes a complementary per-document classification for the language docs directory, operating at the document level rather than the feature level.

---

## Decision

### Three document classes

| Class | Meaning | Fixture backing required? | Change process |
|-------|---------|--------------------------|----------------|
| **normative** | Authoritative specification of implemented, verifiable behavior. The document's claims are tested by the fixture harness. | Yes (or explicit exemption) | Changes require spec review; breaking changes follow ADR-016 |
| **explanatory** | Tutorial, conceptual, or usage-oriented content. Explains but does not define behavior. May reference normative documents. | No | Normal PR process; must not contradict normative docs |
| **transitional** | Documents a planned or in-progress change that has not fully landed. Will be promoted to normative, merged into an existing normative doc, or retired when the feature lands. | No (until promoted) | Must carry a `DONE_WHEN` condition describing when it retires or graduates |

### Banner templates

Each document in `docs/language/` MUST carry exactly one classification banner immediately after the title heading. Banner templates are defined below.

#### Normative banner

```markdown
> **Normative**: This document defines the authoritative behavior of Arukellt as implemented.
> Behavior described here is verified by the fixture harness. Changes require spec review.
> For current verified state, see [../current-state.md](../current-state.md).
```

#### Explanatory banner

```markdown
> **Explanatory**: This document explains concepts and usage patterns.
> It is not the authoritative specification. For normative behavior, see [../language/spec.md](../language/spec.md)
> and [../current-state.md](../current-state.md).
```

#### Transitional banner

```markdown
> **Transitional**: This document describes planned or in-progress changes to Arukellt.
> It will be promoted to normative, merged, or retired when the feature lands.
> For current behavior, see [../current-state.md](../current-state.md) and [../language/spec.md](../language/spec.md).
> DONE_WHEN: <condition under which this document graduates or is retired>
```

### Classification of current language docs

| File | Class | Rationale |
|------|-------|-----------|
| `spec.md` | normative | Frozen authoritative language specification; fixture-backed; post-freeze changes require an ADR |
| `syntax.md` | normative | Current-first syntax reference; reflects implemented, tested behavior |
| `error-handling.md` | normative | Current-first error handling reference; reflects implemented `Result`/`Option` behavior |
| `memory-model.md` | normative | Current-first memory model reference; reflects GC-native T3 implementation |
| `type-system.md` | normative | Current-first type system reference; reflects implemented type checker behavior |
| `syntax-v1-preview.md` | transitional | Describes planned v1 syntax additions not yet normative; retires when all items land in `spec.md` |

### Applying banners (phased rollout)

This ADR defines the classification and banner templates. Banners are **not** applied to all pages in this initial run. Application is a separate work order:

- **Phase 0 (this ADR)**: Define classification, templates, and table. Update docs/language/README.md classification table.
- **Phase 1 (separate issue)**: Apply normative banners to all five normative docs.
- **Phase 2 (separate issue)**: Apply transitional banner to `syntax-v1-preview.md` and add `DONE_WHEN` condition.

---

## Consequences

- Every new document added to `docs/language/` MUST declare its class in `docs/data/language-doc-classifications.toml` before merging.
- The `docs/language/README.md` (generated) will show a classification table reflecting the TOML data.
- Transitional documents that have been superseded without formal retirement are considered "stale transitional" — a future harness check may detect these.
- Explanatory documents are explicitly out of scope for fixture-backed verification; this is a feature, not a gap.

---

## Alternatives considered

**Single stability label per document (reuse ADR-014 labels)**
Rejected: ADR-014 labels (`stable`, `provisional`, `experimental`) describe implementation readiness for *features*. The classification here describes the *epistemic role* of the document (authoritative vs. conceptual vs. in-progress). The two axes are orthogonal — a `stable` feature can still have an `explanatory` tutorial, and a `transitional` design note is not "experimental" in the ADR-014 sense.

**No formal classification; rely on existing blockquote banners**
Rejected: Existing banners are inconsistent (`Current-first`, `Transitional`, `Frozen for v5`). Without a normative list of classes and templates, the inconsistency will grow. A single TOML source of truth makes classification machine-readable and enables a future harness check.

---

## References

- `docs/language/` — language documentation directory
- `docs/data/language-doc-classifications.toml` — machine-readable classification data (created in this work order)
- ADR-014: Stability Labels for Language Spec and Stdlib API
- ADR-016: Breaking Change Process — Three-Piece Set
- `scripts/generate-docs.py` — generates `docs/language/README.md` including classification table
