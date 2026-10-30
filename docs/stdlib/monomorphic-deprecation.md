# Deprecated API Migration Table

> Generated from `std/manifest.toml` by `scripts/gen/generate-docs.py`.
> Lifecycle state and replacement are never maintained separately here.
> A replacement string is not proof of current callability; migration evidence remains required.

Deprecated APIs remain callable for the policy window in
[stability-policy.md](stability-policy.md). Monomorphic compatibility
helpers are included alongside any other deprecated public entry.

| API | Module | Stability | Replacement | Deprecated since | Earliest removal | Reason |
|-----|--------|-----------|-------------|------------------|------------------|--------|
| `get` | `prelude` | `deprecated` | `Vec::get` | `0.1.0` | `1.0.0` | Superseded API |
| `get_unchecked` | `prelude` | `deprecated` | `Vec::get_unchecked` | `0.1.0` | `1.0.0` | Superseded API |
| `slice_len` | `prelude` | `deprecated` | `Slice::len` | `0.1.0` | `1.0.0` | Superseded API |

Total deprecated public entries: **3**.
