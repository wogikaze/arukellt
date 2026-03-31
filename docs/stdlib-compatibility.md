# Stdlib Compatibility Policy

> Source of truth for how Arukellt's standard library evolves and how API changes are communicated.

## Stability Levels

All public entries in `std/manifest.toml` carry one of three stability labels.
The definitions follow [ADR-014](adr/ADR-014-stability-labels.md).

| Level | Meaning |
|-------|---------|
| `stable` | API is frozen. Breaking changes require a migration guide and a prior deprecation cycle. |
| `provisional` | API is broadly settled but minor changes are possible. No breaking changes without changelog notice. |
| `experimental` | Active design; breaking changes possible without prior notice. Not recommended for production code. |

## Current Status

As of the 2026-04-01 baseline, all 267 public stdlib functions are labelled `provisional`.
No functions have reached `stable` yet; that promotion happens at the v1 release freeze.

To view stability counts:

```bash
python3 scripts/generate-docs.py
# counts are shown in docs/std/README.md under each module section
```

## Deprecation Process

1. Add `deprecated = true` and `deprecated_reason = "..."` fields to the manifest entry.
2. Update the stdlib reference docs (`python3 scripts/generate-docs.py`).
3. If the function is `stable`, provide a migration guide entry in `docs/migration/`.
4. The function remains available for at least one minor release after deprecation.
5. Removal moves the function to a `removed_functions` section and produces a compile error with guidance.

## Version Compatibility

- **Minor version bump**: no breaking changes to `stable` APIs; `provisional` APIs may have breaking changes with changelog notice.
- **Major version bump**: all stability levels may have breaking changes; full migration guide required.
- **Pre-v1 (current)**: `provisional` APIs may change between patch releases. Users should pin their version.

## Promotion Criteria

A function is promoted from `provisional` to `stable` when:

1. It has been implemented and tested for at least one full release cycle.
2. Its signature and semantics match the spec with no known design issues.
3. It has fixture coverage in `tests/fixtures/`.
4. The core team agrees the design is final.

At v1 release freeze, all functions that meet these criteria will be promoted to `stable`.

## References

- `std/manifest.toml` — canonical list of public API entries with stability fields
- `docs/adr/ADR-014-stability-labels.md` — stability label definitions
- `scripts/generate-docs.py` — generates reference docs with stability column
- `docs/std/reference.md` — generated stdlib reference (do not edit by hand)
