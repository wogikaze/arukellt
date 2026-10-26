# Stdlib Compatibility Policy

> Policy explanation for how Arukellt's standard library evolves. `std/manifest.toml` is the only factual source for API identities and current stability labels; generated counts live in [`stdlib/reference.md`](stdlib/reference.md).

## Stability Levels

All public entries in `std/manifest.toml` carry one of four lifecycle labels.
The definitions follow [ADR-014](adr/ADR-014-stability-labels.md).

| Level | Meaning |
|-------|---------|
| `stable` | API is frozen. Breaking changes require a migration guide and a prior deprecation cycle. During 0.x only, the explicit [ADR-087](adr/ADR-087-0x-deprecation-exception.md) exception may shorten that cycle. |
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

1. Set `stability = "deprecated"`, `deprecated_by = "..."`, and `deprecated_reason = "..."` on the manifest entry.
2. Update the stdlib reference docs (`python3 scripts/gen/generate-docs.py`).
3. If the function is `stable`, provide a migration guide entry in the issue.
4. The function remains available until the next major version, and for at least one complete release after deprecation.
5. Removal moves the function to a `removed_functions` section and produces a compile error with guidance.
6. During 0.x only, a removal may happen earlier under [ADR-087](adr/ADR-087-0x-deprecation-exception.md): W0009 must fire, the migration guide must record the replacement, and the [stdlib changelog](stdlib/changelog.md) must record the wave. Under that exception `remove_in` is an upper bound. From 1.0.0 the normal rule applies and the exception is void.

## Version Compatibility

- **Minor version bump**: no breaking changes to `stable` APIs; `provisional` APIs may have breaking changes with changelog notice.
- **Major version bump**: all stability levels may have breaking changes; full migration guide required.
- **Pre-v1 (current)**: `provisional` APIs may change between patch releases. Users should pin their version. `stable` removals may occur in a 0.x minor only through the explicit [ADR-087](adr/ADR-087-0x-deprecation-exception.md) exception (W0009 + migration guide + [changelog](stdlib/changelog.md)); the exception ends at 1.0.0.

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
- `docs/adr/ADR-087-0x-deprecation-exception.md` — 0.x stable-removal exception (ADR-014 addendum)
- `docs/stdlib/changelog.md` — stdlib API lifecycle changelog
- `scripts/gen/generate-docs.py` — generates reference docs with stability column
- `docs/stdlib/reference.md` — generated stdlib reference (do not edit by hand)
