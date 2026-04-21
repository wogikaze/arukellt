# Stdlib Docs Ownership Map

> Defines the ownership, maintenance tier, and release gate responsibilities
> for every file in `docs/stdlib/`. See also [docs/directory-ownership.md](../directory-ownership.md)
> for the repo-wide directory ownership map.

## Ownership Tiers

| Tier | Meaning | Edit policy |
|------|---------|-------------|
| **generated** | Auto-generated from `std/manifest.toml` via scripts | Regenerate with the listed generator; **do not hand-edit** |
| **curated** | Human-authored content | Edit directly; requires review before merge |
| **manifest-data** | Structured data files consumed by generators or CI checks | Edit directly; validated by `check-docs-consistency.py` |
| **legacy** | Archived pages kept for backward-compatible URLs | Do not update; redirect readers to replacement pages |

---

## File Ownership Map

### Generated Files

These files are produced by `scripts/gen/generate-docs.py` from `std/manifest.toml` as the
single source of truth. Hand-edits will be overwritten on the next regeneration.

| File | Generator | Source of truth | CI check |
|------|-----------|-----------------|----------|
| `README.md` | `generate-docs.py` | `std/manifest.toml` | `generate-docs.py --check` |
| `reference.md` | `generate-docs.py` | `std/manifest.toml` | `generate-docs.py --check`, stability/target metadata checks |
| `name-index.md` | `generate-docs.py` | `std/manifest.toml` | `check_name_index_completeness` |
| `scoreboard.md` | `generate-scoreboard.sh` | `std/manifest.toml` + fixture counts | — |
| `modules/bytes.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |
| `modules/collections.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |
| `modules/component.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |
| `modules/core.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |
| `modules/csv.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |
| `modules/fs.md` | `generate-docs.py` | `std/manifest.toml` + host badges | `check_host_badge_presence`, `check_cross_page_metadata_consistency` |
| `modules/io.md` | `generate-docs.py` | `std/manifest.toml` + host badges | `check_host_badge_presence`, `check_cross_page_metadata_consistency` |
| `modules/json.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |
| `modules/path.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |
| `modules/process.md` | `generate-docs.py` | `std/manifest.toml` + host badges | `check_host_badge_presence`, `check_cross_page_metadata_consistency` |
| `modules/random.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |
| `modules/seq.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |
| `modules/test.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |
| `modules/text.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |
| `modules/time.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |
| `modules/toml.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |
| `modules/wasm.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |
| `modules/wit.md` | `generate-docs.py` | `std/manifest.toml` + curated overview | `check_cross_page_metadata_consistency` |

### Curated Documents

These files are authored and maintained by human contributors. Changes require review.

| File | Purpose | Stakeholder | CI check |
|------|---------|-------------|----------|
| `cookbook.md` | Hands-on usage recipes with fixture links | stdlib maintainers | `check_cookbook_example_drift`, `check_recipe_fixture_links` |
| `stability-policy.md` | Stability label definitions and tier change process | project leads | — (manual review) |
| `std.md` | Comprehensive design rationale | stdlib architects | — (manual review) |
| `module-system.md` | Import syntax documentation | language team | — (manual review) |
| `generation-schema.md` | Schema enforced by the docs generator | docs tooling maintainers | — (manual review) |
| `expansion-policy.md` | Policy for which modules accept new APIs | project leads | — (manual review) |
| `migration-guidance.md` | Deprecated API migration paths and examples | stdlib maintainers | `check_deprecated_badge_presence` (cross-check) |
| `monomorphic-deprecation.md` | Monomorphic API deprecation plan | stdlib maintainers | — (manual review) |
| `prelude-dedup.md` | Prelude namespace deduplication rules | language team | — (manual review) |
| `prelude-migration.md` | Historical v3 prelude migration record | — (archive) | — |
| `ownership-map.md` | This file — docs ownership and release gates | docs tooling maintainers | `check_ownership_map_completeness` |

### Manifest Data

| File | Purpose | Consumers | CI check |
|------|---------|-----------|----------|
| `recipe-manifest.toml` | Maps cookbook recipes to test fixture files | `check-docs-consistency.py` | `check_recipe_fixture_links` |

### Legacy / Archived Pages

| File | Superseded by | Reason kept |
|------|---------------|-------------|
| `core.md` | `modules/core.md` | Backward-compatible URLs |
| `io.md` | `modules/io.md` | Backward-compatible URLs |

---

## Source-of-Truth Chain

```
std/manifest.toml                        (canonical API definition)
        │
        ├─► scripts/gen/generate-docs.py ──► docs/stdlib/reference.md
        │                              ├─► docs/stdlib/modules/*.md
        │                              ├─► docs/stdlib/name-index.md
        │                              └─► docs/stdlib/README.md
        │
        └─► scripts/check/check-docs-consistency.py
                    │
                    ├─► validates generated docs freshness
                    ├─► validates metadata consistency across pages
                    ├─► validates recipe-manifest.toml ◄── docs/stdlib/recipe-manifest.toml
                    ├─► validates cookbook example drift ◄── docs/stdlib/cookbook.md
                    ├─► validates host badge presence
                    ├─► validates name-index completeness
                    ├─► validates stability metadata
                    └─► validates ownership map completeness ◄── docs/stdlib/ownership-map.md
```

---

## Release Gate Checklist

The following checks must **all pass** before stdlib docs changes can be merged or published.
Items marked **CI** are automated in `check-docs-consistency.py` or `verify-harness.sh`.
Items marked **Manual** require human verification during review.

### Gate 1 — Generated Docs Freshness

| # | Check | Type | Command |
|---|-------|------|---------|
| 1.1 | Generated docs match manifest | **CI** | `python3 scripts/gen/generate-docs.py --check` |
| 1.2 | Maturity matrix matches TOML classifications | **CI** | `check_maturity_matrix_freshness` |
| 1.3 | Fixture count matches project-state.toml | **CI** | `check_fixture_count_freshness` |
| 1.4 | Issue indexes up to date | **CI** | `check_issue_index_freshness` |

### Gate 2 — Metadata Consistency

| # | Check | Type | Command |
|---|-------|------|---------|
| 2.1 | Stability metadata: manifest ↔ reference.md | **CI** | `check_stability_metadata_in_reference` |
| 2.2 | Target metadata: manifest ↔ reference.md | **CI** | `check_target_metadata_in_reference` |
| 2.3 | Cross-page metadata: modules/* ↔ manifest | **CI** | `check_cross_page_metadata_consistency` |
| 2.4 | Stability vs implementation consistency | **CI** | `check_stability_implementation_consistency` |

### Gate 3 — Badge & Badge Drift

| # | Check | Type | Command |
|---|-------|------|---------|
| 3.1 | Host module pages have 🎯 Target badges | **CI** | `check_host_badge_presence` |
| 3.2 | Deprecated functions have ⚠️ Deprecated badges | **CI** | `check_deprecated_badge_presence` |

### Gate 4 — Cookbook & Recipe Integrity

| # | Check | Type | Command |
|---|-------|------|---------|
| 4.1 | Cookbook code blocks match fixture source | **CI** | `check_cookbook_example_drift` |
| 4.2 | recipe-manifest.toml fixture paths valid | **CI** | `check_recipe_fixture_links` |

### Gate 5 — Coverage Completeness

| # | Check | Type | Command |
|---|-------|------|---------|
| 5.1 | name-index.md lists all public functions | **CI** | `check_name_index_completeness` |
| 5.2 | host_stub functions have fixture coverage | **CI** | `check_host_stub_fixture_coverage` |
| 5.3 | All docs/stdlib/ files tracked in ownership map | **CI** | `check_ownership_map_completeness` |

### Gate 6 — Spec ↔ Guide Sync

| # | Check | Type | Command |
|---|-------|------|---------|
| 6.1 | Stable spec features covered in guide.md | **CI** | `check_spec_guide_sync` |
| 6.2 | Per-subsection feature drift within tolerance | **CI** | `check_spec_guide_feature_drift` |

### Gate 7 — Capability & State

| # | Check | Type | Command |
|---|-------|------|---------|
| 7.1 | host_stub count acknowledged in current-state.md | **CI** | `check_capability_state` |

### Gate 8 — Manual Review

| # | Check | Type | When required |
|---|-------|------|---------------|
| 8.1 | Curated overview accuracy | **Manual** | When module behavior changes |
| 8.2 | Stability tier change follows checklist | **Manual** | On any `stability` field change in manifest |
| 8.3 | Migration guidance updated for deprecations | **Manual** | When `deprecated_by` added to manifest |
| 8.4 | Expansion policy respected for new modules | **Manual** | When new `[[modules]]` entry added |
| 8.5 | No hand-edits to generated files | **Manual** | Every PR touching `docs/stdlib/` |

### Running All Gates

```bash
# Run all automated gates (required before merge):
python3 scripts/check/check-docs-consistency.py    # Gates 1–7 (18 checks)
python scripts/manager.py verify quick       # Includes docs consistency + harness checks

# If generated source inputs changed:
python3 scripts/gen/generate-docs.py             # Regenerate all generated files
python3 scripts/gen/generate-docs.py --check     # Verify freshness (Gate 1.1)

# Full verification (before releases):
bash scripts/manager.py --full        # All checks including heavy ones
```

---

## Coverage Gaps

Known gaps in stdlib docs coverage, documented here for tracking.
Each gap should reference a follow-up issue when one is filed.

### Gap 1 — Curated documents lack automated validation

**Files affected:** `stability-policy.md`, `std.md`, `module-system.md`,
`generation-schema.md`, `expansion-policy.md`, `prelude-dedup.md`

**Description:** These policy and design documents are not validated by any CI check.
If the manifest or runtime behavior changes, these documents may silently drift
from reality. Only manual review catches inconsistencies.

**Mitigation:** Gate 8 (manual review) covers these during PRs that touch `docs/stdlib/`.

### Gap 2 — No automated deprecation timeline tracking

**Description:** When a function is deprecated (`stability = "deprecated"` in manifest),
the stability policy requires keeping the API for "at least one major version."
No automated check tracks how long a function has been deprecated or enforces
removal timelines.

**Mitigation:** Deprecation dates could be added to manifest entries and validated
by a new consistency check.

### Gap 3 — Experimental promotion readiness not tracked

**Description:** The promotion process in `stability-policy.md` requires that an
experimental module's API be "unchanged for at least one minor release cycle."
No automated check tracks how many releases an experimental API has been stable.

**Mitigation:** A `since_version` field in manifest `[[modules]]` entries would
enable automated promotion-readiness checks.

### Gap 4 — scoreboard.md not validated by consistency checks

**Description:** `scoreboard.md` is generated by `scripts/gen/generate-scoreboard.sh`,
a different generator than `generate-docs.py`. The consistency checker does not
verify its freshness. It could drift without detection.

**Mitigation:** Add a `check_scoreboard_freshness` check to `check-docs-consistency.py`,
or unify scoreboard generation into `generate-docs.py`.

### Gap 5 — Legacy pages have no link-rot detection

**Description:** The legacy `core.md` and `io.md` archive pages contain redirect
notices pointing to `modules/core.md` and `modules/io.md`. If those target pages
are renamed, the redirect links silently break.

**Mitigation:** The existing `internal link integrity` check in `verify-harness.sh`
may cover this; verify that it scans `docs/stdlib/` legacy pages.
