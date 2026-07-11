# Document Ownership (shared schema)

Shared ownership vocabulary for section-specific maps:

- [`../language/ownership-map.md`](../language/ownership-map.md)
- [`../stdlib/ownership-map.md`](../stdlib/ownership-map.md)
- [`../directory-ownership.md`](../directory-ownership.md)

## Schema

| Field | Meaning |
|-------|---------|
| `tier` | `generated` \| `curated` \| `transitional` \| `archive` \| `product` \| `internal` |
| `owner` | Team or subsystem (`language`, `stdlib`, `compiler`, `docs`, …) |
| `source_of_truth` | Path to machine-readable input, or `null` for curated prose |
| `generator` | Command that regenerates the artifact, or `null` |
| `ci_check` | Gate script / verify label that enforces freshness |

## Rules

1. Facts (counts, target tables, capability matrices) live in TOML/JSON — Markdown explains or is generated.
2. Directory-level “generated” claims are forbidden when the directory mixes inputs and outputs (`docs/data/`).
3. Section ownership maps may only add deltas; do not fork this schema.

Enforced by `scripts/check/gate-765-docs-ci-hard-gates.py`.
