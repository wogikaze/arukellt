# Stdlib Docs Generation Schema

> **This schema is enforced at generation time.**
> `python3 scripts/generate-docs.py` validates every `[[functions]]` entry in
> `std/manifest.toml` against these rules before producing any output.
> A schema violation causes a non-zero exit and blocks CI.

## Page Kinds

The generator recognises four functional roles for `[[functions]]` entries.
The role is inferred from the combination of `kind` and `module` fields:

| Role | Condition | Example |
|------|-----------|---------|
| **prelude** | has `kind`, no `module` (or `prelude = true`) | `concat`, `println` |
| **module** | has `module`, no `kind` | `range_new` in `std::core` |
| **host_stub** | `kind = "host_stub"` | `request` in `std::host::http` |
| **mixed** | has both `kind` and `module` | `memory_copy` in `std::wasm` |

---

## Required Fields — Every `[[functions]]` Entry

These fields are mandatory regardless of role:

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Public function identifier (no `__intrinsic_` prefix) |
| `params` | list of strings | Ordered parameter types in Arukellt type syntax |
| `returns` | string | Return type in Arukellt type syntax |
| `stability` | string | One of the values in [Valid Stability Labels](#valid-stability-labels) |
| `doc_category` | string | Grouping key used by the reference page generator |

---

## Optional Fields

| Field | Type | Condition / Notes |
|-------|------|-------------------|
| `kind` | string | See [Valid Kind Values](#valid-kind-values). Required for prelude/host entries. |
| `module` | string | Fully-qualified module name (`std::core`, `std::host::http`, …). Required for module entries. |
| `intrinsic` | string | Backing `__intrinsic_*` name. Expected when `kind` is `prelude_wrapper` or `intrinsic_wrapper`. |
| `prelude` | bool | `true` if auto-imported without an explicit `import`. |
| `target` | list of strings | Target triples this function is available on. **Must be a list** when present. Required for `host_stub`. |
| `deprecated_by` | string | Replacement identifier. Signals that this entry is superseded. |
| `doc` | string | Inline documentation string for the function (currently unused by generator; reserved). |

---

## Valid Stability Labels

Defined in `spec.md` (ADR-013 §Stability) and enforced by `STABILITY_LABELS` in
`scripts/generate-docs.py`:

| Label | Meaning |
|-------|---------|
| `stable` | Public API; breaking changes require a deprecation cycle. |
| `provisional` | API shape is mostly settled but may change without a full deprecation. |
| `experimental` | Subject to change; do not depend on in production code. |
| `unimplemented` | Documented as planned; not yet available. |

---

## Valid Kind Values

Enforced by `VALID_KIND_VALUES` in `scripts/generate-docs.py`:

| Kind | Meaning |
|------|---------|
| `builtin` | Direct builtin; no `prelude.ark` wrapper. |
| `intrinsic` | Low-level `__intrinsic_*` name only; no public wrapper. |
| `prelude_wrapper` | Public function in `prelude.ark` that calls an `__intrinsic_*`. |
| `intrinsic_wrapper` | Module-level function that wraps an `__intrinsic_*`. |
| `host_stub` | Capability-gated host function (WASI/component model). |

---

## Kind-Specific Requirements

### `host_stub`

Entries with `kind = "host_stub"` must additionally provide:

- `module` — the `std::host::*` module they belong to.
- `target` — a **list** of target triples on which the capability is available
  (e.g., `["wasm32-wasi-p2"]`).

---

## CI Enforcement

Schema validation is integrated into the documentation generation pipeline:

```
python3 scripts/generate-docs.py          # validates schema, then generates
python3 scripts/generate-docs.py --check  # validates schema, then checks freshness
python3 scripts/check-docs-consistency.py # calls generate-docs.py --check internally
bash scripts/verify-harness.sh --quick    # runs check-docs-consistency.py as a bg check
```

A violation in `std/manifest.toml` will:
1. Print a descriptive error listing each violating entry and field.
2. Exit non-zero from `generate-docs.py`.
3. Cascade as a failure in `check-docs-consistency.py` and `verify-harness.sh --quick`.

---

## Schema Constants (source of truth)

The authoritative schema constants live in `scripts/generate-docs.py`:

```python
FUNCTION_REQUIRED_FIELDS = ("name", "params", "returns", "stability", "doc_category")

VALID_KIND_VALUES = frozenset(
    {"builtin", "intrinsic", "prelude_wrapper", "intrinsic_wrapper", "host_stub"}
)

FUNCTION_KIND_REQUIRED = {
    "host_stub": ("module", "target"),
}
```

This document is manually maintained alongside those constants. If the constants
change, update this file in the same commit.
