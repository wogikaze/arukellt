# Docs size / split plan (manual documents)

Tracked under **#770** (Phase 4) / originally #765.

| Field | Value |
|-------|-------|
| Owner | docs-audit track |
| Status | in progress |
| First slices | 2026-07-11 |

## First slices completed

| Document | Action | Result |
|----------|--------|--------|
| `docs/language/syntax-v1-preview.md` | Retired → `docs/history/language/` | Landed items live in spec/syntax |
| `docs/stdlib/modules/io.md` | `overview_only` family landing | Cross-links only; API tables on per-module pages |
| `docs/stdlib/io.md` | Legacy alias page (already thin) | Points at host family overview |

## Remaining coarse manuals

| Document | Approx size | Proposed split | Owner | Target |
|----------|-------------|----------------|-------|--------|
| `docs/language/spec.md` | ~1600 lines | Keep normative core; move examples to guide/cookbook | docs | next #770 follow-up |
| `docs/stdlib/cookbook.md` | ~900 lines | Topic chapters under `docs/cookbook/` | docs | after skip budget ratchet |
| `docs/compiler/ir-spec.md` | ~1800 lines | Per-IR chapter files + landing README | compiler | separate issue |

Do not start further splits until owners agree per chapter. Prefer stable anchors
(heading ids) when moving content so inbound links keep working.
