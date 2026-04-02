# Language Docs Ownership Map

> Defines the ownership, maintenance tier, and release gate responsibilities
> for every file in `docs/language/`. See also [docs/stdlib/ownership-map.md](../stdlib/ownership-map.md)
> for the stdlib counterpart.

## Ownership Tiers

| Tier | Meaning | Edit policy |
|------|---------|-------------|
| **generated** | Auto-generated from `docs/data/language-doc-classifications.toml` via scripts | Regenerate with the listed generator; **do not hand-edit** |
| **curated** | Human-authored normative or explanatory content | Edit directly; requires review before merge |
| **transitional** | Documents describing planned changes; retire when items land | Edit directly; review for staleness each release |

---

## File Ownership Map

### Generated Files

These files are produced by `scripts/generate-docs.py` from
`docs/data/language-doc-classifications.toml` as the single source of truth.
Hand-edits will be overwritten on the next regeneration.

| File | Generator | Source of truth | CI check |
|------|-----------|-----------------|----------|
| `README.md` | `generate-docs.py` | `docs/data/language-doc-classifications.toml` | `generate-docs.py --check` |
| `maturity-matrix.md` | `generate-docs.py` | `docs/data/language-doc-classifications.toml` | `check_maturity_matrix_freshness` |

### Curated Documents

These files are authored and maintained by human contributors. Changes require review.
Each document carries a classification per ADR-018 (normative / explanatory).

| File | Class | Purpose | Stakeholder | CI check |
|------|-------|---------|-------------|----------|
| `spec.md` | normative | Frozen authoritative language specification | language team | `check_spec_guide_sync`, `check_spec_guide_feature_drift` |
| `guide.md` | explanatory | Practical language walkthrough — stable/implemented features only | language team | `check_spec_guide_sync`, `check_spec_guide_feature_drift` |
| `syntax.md` | normative | Complete syntax reference for implemented constructs | language team | — (manual review) |
| `type-system.md` | normative | Type rules, generics, inference, and trait-like behavior | language team | — (manual review) |
| `error-handling.md` | normative | Result, Option, `?` operator, and recovery patterns | language team | — (manual review) |
| `memory-model.md` | normative | GC-native ownership, value semantics, and lifetime model | language team | — (manual review) |
| `ownership-map.md` | — | This file — docs ownership and release gates | docs tooling maintainers | — (manual review) |

### Transitional Documents

These documents describe planned or in-progress changes. They retire
when all described items land in their target normative documents.

| File | Target | Retirement condition | CI check |
|------|--------|----------------------|----------|
| `syntax-v1-preview.md` | `spec.md`, `syntax.md` | All v1 syntax items merged into spec.md and syntax.md | — (manual review at each release) |

---

## Source-of-Truth Chain

```
docs/data/language-doc-classifications.toml     (canonical feature maturity data)
        │
        ├─► scripts/generate-docs.py ──► docs/language/README.md
        │                              └─► docs/language/maturity-matrix.md
        │
        └─► scripts/check-docs-consistency.py
                    │
                    ├─► validates maturity-matrix.md freshness
                    │       (check_maturity_matrix_freshness)
                    │
                    ├─► validates spec ↔ guide top-level sync
                    │       (check_spec_guide_sync)
                    │
                    └─► validates spec ↔ guide per-subsection drift
                            (check_spec_guide_feature_drift)

spec.md  ◄──── check_spec_guide_sync ────►  guide.md
         ◄── check_spec_guide_feature_drift ─►
```

---

## Release Gate Checklist

The following checks must **all pass** before language docs changes can be merged
or published. Items marked **CI** are automated in `check-docs-consistency.py` or
`verify-harness.sh`. Items marked **Manual** require human verification during review.

### Gate 1 — Generated Docs Freshness

| # | Check | Type | Command / Function |
|---|-------|------|--------------------|
| 1.1 | Generated docs match TOML classifications | **CI** | `python3 scripts/generate-docs.py --check` |
| 1.2 | Maturity matrix matches `[[features]]` in TOML | **CI** | `check_maturity_matrix_freshness` |

### Gate 2 — Spec ↔ Guide Sync

| # | Check | Type | Command / Function |
|---|-------|------|--------------------|
| 2.1 | Stable spec features covered in guide.md | **CI** | `check_spec_guide_sync` |
| 2.2 | Per-subsection feature drift within tolerance | **CI** | `check_spec_guide_feature_drift` |

### Gate 3 — Anchor & Permalink Integrity

| # | Check | Type | Command / Function |
|---|-------|------|--------------------|
| 3.1 | No S1 headings renamed without redirect alias (ADR-019) | **Manual** | Review diff for `##` heading changes in normative docs |
| 3.2 | Internal file links resolve | **CI** | `scripts/check-links.sh` (file references; anchor fragments are v2) |

### Gate 4 — Classification Consistency

| # | Check | Type | When required |
|---|-------|------|---------------|
| 4.1 | ADR-018 banner present in new/modified normative docs | **Manual** | When adding or modifying normative documents |
| 4.2 | Classification in README matches document content | **Manual** | When document scope changes |

### Gate 5 — Curated Document Review

| # | Check | Type | When required |
|---|-------|------|---------------|
| 5.1 | spec.md post-freeze changes have an ADR | **Manual** | On any edit to `spec.md` |
| 5.2 | guide.md covers only stable/implemented features | **Manual** | On any edit to `guide.md` |
| 5.3 | syntax.md matches implemented parser behavior | **Manual** | When parser changes land |
| 5.4 | type-system.md matches implemented type checker | **Manual** | When type checker changes land |
| 5.5 | error-handling.md reflects current Result/Option behavior | **Manual** | When error types change |
| 5.6 | memory-model.md reflects current GC/ownership implementation | **Manual** | When runtime memory model changes |

### Gate 6 — Transitional Document Lifecycle

| # | Check | Type | When required |
|---|-------|------|---------------|
| 6.1 | syntax-v1-preview.md items checked against spec.md | **Manual** | Each release — retire items that have landed |
| 6.2 | Transitional docs marked for retirement when empty | **Manual** | Each release |

### Running All Gates

```bash
# Run all automated gates (required before merge):
python3 scripts/check-docs-consistency.py    # Gates 1–2 (automated checks)
bash scripts/verify-harness.sh --quick       # Includes docs consistency + harness checks

# If generated source inputs changed:
python3 scripts/generate-docs.py             # Regenerate README.md + maturity-matrix.md
python3 scripts/generate-docs.py --check     # Verify freshness (Gate 1.1)

# Full verification (before releases):
bash scripts/verify-harness.sh --full        # All checks including heavy ones
```

---

## Automated vs Manual Coverage Summary

| Scope | Automated checks | Manual checks | Notes |
|-------|------------------|---------------|-------|
| Generated files (`README.md`, `maturity-matrix.md`) | 2 CI checks | — | Fully covered by `generate-docs.py --check` and `check_maturity_matrix_freshness` |
| Spec ↔ Guide sync | 2 CI checks | — | `check_spec_guide_sync` (top-level) + `check_spec_guide_feature_drift` (subsection) |
| Anchor/link integrity | 1 CI check (`check-links.sh`) | 1 manual (ADR-019 heading renames) | Anchor fragment checking is v2 |
| Curated normative docs (`spec.md`, `syntax.md`, `type-system.md`, `error-handling.md`, `memory-model.md`) | — | 5 manual review items | No automated content-accuracy checks beyond spec↔guide sync |
| Curated explanatory docs (`guide.md`) | — | 1 manual review item | Covered indirectly by spec↔guide sync CI |
| Transitional docs (`syntax-v1-preview.md`) | — | 2 manual review items | Checked each release for retirement readiness |
| **Total** | **5 automated** | **9 manual** | |

---

## Coverage Gaps

Known gaps in language docs coverage, documented here for tracking.

### Gap 1 — Normative docs lack automated content-accuracy checks

**Files affected:** `syntax.md`, `type-system.md`, `error-handling.md`, `memory-model.md`

**Description:** These normative documents describe implemented behavior, but no CI
check validates that their content matches actual compiler/runtime behavior. If the
parser, type checker, or runtime changes, these docs may silently drift from reality.
Only the spec↔guide sync checks cover `spec.md` and `guide.md` indirectly.

**Mitigation:** Gate 5 (manual review) requires human verification when implementation
changes land. Future work could add fixture-backed doc validation similar to stdlib's
cookbook example drift checking.

### Gap 2 — No automated staleness check for transitional documents

**Files affected:** `syntax-v1-preview.md`

**Description:** When a planned syntax item lands in `spec.md`, no automated check
detects that the transitional document should be updated or retired. The item
could remain listed as "planned" indefinitely after implementation.

**Mitigation:** Gate 6 (manual review) checks transitional docs each release.
A future `check_transitional_staleness` function could cross-reference
`syntax-v1-preview.md` items against spec.md sections.

### Gap 3 — Anchor fragment link checking deferred to v2

**Files affected:** All documents with cross-document `#anchor` links

**Description:** `scripts/check-links.sh` validates file-level references but does
not check that `#heading-anchor` fragments resolve to actual headings in target
documents. Broken anchor links are invisible to CI.

**Mitigation:** ADR-019 documents this as a v2 check. Until then, heading renames
in normative docs require manual review (Gate 3.1).

### Gap 4 — Classification banner presence not enforced by CI

**Files affected:** All normative and explanatory documents

**Description:** ADR-018 requires normative/explanatory/transitional banners in each
document, but no automated check verifies banner presence or correctness. A document
could lose its banner during editing without detection.

**Mitigation:** Gate 4 (manual review) covers this during PR review. A future
`check_classification_banners` function could validate banner presence against
the classification table in `language-doc-classifications.toml`.
