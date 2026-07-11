# Stdlib Docs Generation Schema

> **This schema is enforced at generation time.**
> `python3 scripts/gen/generate-docs.py` validates every `[[functions]]` entry in
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
| `target` | list of strings | **Legacy single axis.** Prefer `targets` + `host_profiles` + `requires` + `[implementation]` below. When present alone, means language targets (canonical ids: `wasm32`, `wasm32-gc`). Not a target triple. Required for `host_stub` until the multi-axis fields are mandatory. |
| `targets` | list of strings | Language targets (`wasm32`, `wasm32-gc`, …). Preferred over singular `target`. |
| `host_profiles` | list of strings | Host profiles (`wasi-p1`, `wasi-p2`, …). Orthogonal to language target (ADR-007). |
| `requires` | list of strings | Required host capabilities (e.g. `host.stdout`, `host.env`). |
| `implementation` | table | Per-target coverage: `implemented` / `missing-adapter` / `unimplemented`. |
| `implementation_status` | string | Semantic completeness: `functional`, `limited`, `stub`, or `unreachable`. If omitted, the generator derives a conservative status from kind and documented limitations. This axis is displayed separately from lifecycle stability for every public API. |
| `semantic_id` | string | Reference into `docs/data/core-ops.toml` `[[operations]]` (ADR-042). Public path stays in manifest only. |
| `type_id` | string | Reference into `docs/data/core-ops.toml` `[[types]]`. |
| `deprecated_by` | string | Replacement identifier. Signals that this entry is superseded. |
| `deprecated_since` | string | Release that started deprecation (W0008). |
| `remove_in` | string | Earliest release that may delete the entry. |
| `doc` | string | Inline documentation string for the function (currently unused by generator; reserved). |

### Availability axes (design)

Language target, host profile, required capability, and implementation coverage are
**separate**. Do not encode “wasm32-gc + WASI P2” as a single fake target name.

```toml
# Example: stdout on both Wasm targets under either host profile
targets = ["wasm32", "wasm32-gc"]
host_profiles = ["wasi-p1", "wasi-p2"]
requires = ["host.stdout"]

[implementation]
wasm32 = "implemented"
wasm32-gc = "implemented"
```

```toml
# Example: env::var — adapter missing on wasm32
targets = ["wasm32", "wasm32-gc"]
host_profiles = ["wasi-p1", "wasi-p2"]
requires = ["host.env"]

[implementation]
wasm32 = "missing-adapter"
wasm32-gc = "implemented"
```

Generated module pages should not emit a single Target badge for mixed modules.
Use `Availability: mixed — see individual symbols` when symbols differ.

---

## Valid Stability Labels

Defined by ADR-014 and enforced by `PUBLIC_API_STABILITY_LABELS` in
`scripts/gen/generate-docs.py`:

| Label | Meaning |
|-------|---------|
| `stable` | Public API; breaking changes require a deprecation cycle. |
| `provisional` | API shape is mostly settled but may change without a full deprecation. |
| `experimental` | Subject to change; do not depend on in production code. |
| `deprecated` | Still callable during its removal window; a replacement is named by `deprecated_by`. |

`unimplemented` is a language-feature maturity value, not a valid lifecycle
value for a public `std/manifest.toml` entry. Planned APIs must not be published
as public entries merely to reserve a name.

Deprecated entries require `deprecated_by`. Their lifecycle dates resolve from
per-entry `deprecated_since` / `remove_in` fields or the manifest's required
`[deprecation_policy]` defaults. The policy also requires at least one complete
release before removal; generation fails if that minimum is less than one.

---

## Valid Kind Values

Enforced by `VALID_KIND_VALUES` in `scripts/gen/generate-docs.py`:

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
- `target` **or** `targets` — a **list** of canonical language target ids
  (e.g. `["wasm32-gc"]`). Prefer the multi-axis fields above when documenting
  host-profile / capability differences; `target` alone is insufficient for
  mixed modules.

---

## CI Enforcement

Schema validation is integrated into the documentation generation pipeline:

```
python3 scripts/gen/generate-docs.py          # validates schema, then generates
python3 scripts/gen/generate-docs.py --check  # validates schema, then checks freshness
python3 scripts/check/check-docs-consistency.py # calls generate-docs.py --check internally
python scripts/manager.py verify quick    # runs check-docs-consistency.py as a bg check
```

A violation in `std/manifest.toml` will:
1. Print a descriptive error listing each violating entry and field.
2. Exit non-zero from `generate-docs.py`.
3. Cascade as a failure in `check-docs-consistency.py` and `verify-harness.sh --quick`.

---

## Schema Constants (source of truth)

The authoritative schema constants live in `scripts/gen/generate-docs.py`:

```python
PUBLIC_API_STABILITY_LABELS = ("stable", "provisional", "experimental", "deprecated")
LANGUAGE_FEATURE_STABILITY_LABELS = ("stable", "provisional", "experimental", "unimplemented")

FUNCTION_REQUIRED_FIELDS = ("name", "params", "returns", "stability", "doc_category")

VALID_KIND_VALUES = frozenset(
    {"builtin", "intrinsic", "prelude_wrapper", "intrinsic_wrapper", "host_stub"}
)

FUNCTION_KIND_REQUIRED = {
    "host_stub": ("module", "target"),
}
```

The consistency gate verifies this table against the generator constants; a
label-set mismatch fails documentation verification.
