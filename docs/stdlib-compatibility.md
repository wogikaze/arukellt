# Stdlib Compatibility Policy

> Policy explanation for how Arukellt's standard library evolves. `std/manifest.toml` is the only factual source for API identities and current stability labels; generated counts live in [`stdlib/reference.md`](stdlib/reference.md).

## Stability Levels

All public entries in `std/manifest.toml` carry one of four lifecycle labels.
The definitions follow [ADR-014](adr/ADR-014-stability-labels.md).

| Level | Meaning |
|-------|---------|
| `stable` | API is frozen. Breaking changes require a migration guide and a prior deprecation cycle. |
| `provisional` | API is broadly settled but minor changes are possible. No breaking changes without changelog notice. |
| `experimental` | Active design; breaking changes possible without prior notice. Not recommended for production code. |
| `deprecated` | Still callable during a migration window, but superseded by the manifest entry's `deprecated_by` replacement. |

## Current Status

Do not record a hand-maintained count or blanket tier in this policy. To view the current manifest-derived counts:

```bash
python3 scripts/gen/generate-docs.py
# counts are shown in docs/stdlib/reference.md
```

## Deprecation Process

1. Add `deprecated = true` and `deprecated_reason = "..."` fields to the manifest entry.
2. Update the stdlib reference docs (`python3 scripts/gen/generate-docs.py`).
3. If the function is `stable`, provide a migration guide entry in the issue.
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
- `scripts/gen/generate-docs.py` — generates reference docs with stability column
- `docs/std/reference.md` — generated stdlib reference (do not edit by hand)
