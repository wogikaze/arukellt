# Standard Library Stability Policy

## Stability Labels

Every public API in the Arukellt standard library carries one of four lifecycle labels. `std/manifest.toml` owns the current label for every entry; this document defines their semantics.

### Stable

- Backward-compatible within a major version.
- Breaking changes only occur on major version bumps.
- Compatibility commitment within the stated versioning policy.
- This label does not claim that the project or implementation is production-ready;
  release readiness is owned by [`../current-state.md`](../current-state.md).

### Experimental

- API may change in **minor** versions without notice.
- Functionality is available but the interface is not yet finalized.
- Marked with ⚠️ in documentation.

### Provisional

- API is broadly settled, but may change in minor versions with changelog notice.
- Promotion to Stable requires the readiness checks below.

### Deprecated

- API remains callable during its migration window but is no longer recommended.
- Every deprecated manifest entry names its replacement with `deprecated_by`.

---

## Current Classification

Current module and API classifications are generated from `std/manifest.toml`; see [`reference.md`](reference.md) and the generated module pages. This policy intentionally does not duplicate those mutable tables.

---

## Promotion Process

An Experimental module becomes Stable when:

1. The API has been unchanged for at least one minor release cycle.
2. Test coverage meets the project baseline.
3. At least one ADR documents the design rationale.
4. The change is recorded in the changelog.

## Error / Result Naming Conventions

The stdlib follows consistent naming patterns for functions that can fail:

### Return type conventions

| Pattern | Returns | Example |
|---------|---------|---------|
| `parse_*` | `Result<T, String>` | `parse_i32("42")` → `Ok(42)` |
| `try_*` | `Result<T, String>` | Reserved for fallible operations |
| `*_or` | `T` (with fallback) | `unwrap_or(result, default)` |

### Result / Option builtins (prelude)

> **Eradication scope (ADR-046):** these prelude free functions are
> classified in the [eradication scorecard](eradication.md) and migrate to
> `Result` / `Option` methods under issue #917. The table documents the
> current surface, not the target shape.

| Function | Signature | Description |
|----------|-----------|-------------|
| `is_ok` | `(Result<T, E>) -> bool` | Test if Result is Ok |
| `is_err` | `(Result<T, E>) -> bool` | Test if Result is Err |
| `unwrap` | `(Result<T, E>) -> T` | Extract Ok or panic |
| `unwrap_or` | `(Result<T, E>, T) -> T` | Extract Ok or return default |
| `is_some` | `(Option<T>) -> bool` | Test if Option is Some |
| `is_none` | `(Option<T>) -> bool` | Test if Option is None |
| `unwrap` | `(Option<T>) -> T` | Extract Some or panic |
| `unwrap_or` | `(Option<T>, T) -> T` | Extract Some or return default |

### Host vs pure family errors

- **Host family** (`std::host::*`): Functions that interact with the OS return `Result` when the operation can fail due to external state (e.g., `fs::read_file` returns `Result<String, String>`).
- **Pure family** (`std::collections`, `std::math`, etc.): Functions either return `Option` for partial operations or panic on precondition violations (e.g., `get` vs `get_unchecked`).

### Naming rules

1. Constructor variants use PascalCase: `Ok(v)`, `Err(e)`, `Some(v)`, `None`
2. Predicate builtins use snake_case: `is_ok`, `is_err`, `is_some`, `is_none`
3. Extraction builtins use snake_case: `unwrap`, `unwrap_or`, `expect`
4. Match arms use PascalCase constructors: `match r { Ok(v) => ..., Err(e) => ... }`

## Deprecation Process

A Stable API is deprecated before removal:

1. Mark with `@deprecated` annotation and document the replacement.
2. Keep the deprecated API available until the next major version, and for at least one complete release.
3. Remove no earlier than that major-version boundary, with a migration guide.

### 0.x eradication exception (#917)

Before 1.0.0 the project has no scheduled 1.0.0 release, so the
`remove_in = "1.0.0"` policy default is not by itself evidence that a
removal window closed. The #917 eradication wave may remove early **only**
these classes, and each wave must record the exception:

- `experimental` entries — outside the stability contract (ADR-014 D4);
- `deprecated` prelude bindings that the v2 prelude already gates with
  **E0100** plus a resolver migration hint, and whose replacement is
  shipped and tested.

`stable` entries still require the ADR-014 deprecation-window addendum,
and the `std::seq` helpers still require W0009 qualified-call coverage.
Early removals must cite this exception explicitly in the migration guide.

---

## Free-Function Eradication (ADR-046)

ADR-046 removes every **user-reachable free function** from the stdlib
surface. The public API shape is method / associated function / trait
dispatch only (ADR-044); the sole exception is non-public `__intrinsic_*`
compiler bridges, which never appear in the public surface.

- The authoritative per-symbol classification lives in
  [`../data/stdlib-api-eradication.toml`](../data/stdlib-api-eradication.toml):
  every `std/manifest.toml` `[[functions]]` entry resolves to
  `must delete`, `associated-or-method`, or `intrinsic-only`, each with an
  owning issue and a replacement note.
- The generated scorecard and full inventory live in
  [eradication.md](eradication.md); the zero goal is **0 public free
  functions, 0 `prelude_wrapper` entries, 0 monomorphic helpers**.
- Coverage is enforced by `python3 scripts/check/check-stdlib-eradication.py`
  and by `docs regenerate` / `docs check`: any unclassified manifest symbol
  fails the gate.
- Retirement follows the deprecation process above; execution waves are
  tracked in issue #917 under policy issue #709.

---

## Stability Tier Change Checklist

Use this checklist when promoting, demoting, or deprecating a function or module's stability tier.
Every tier change must update **both** the manifest source of truth and the generated documentation.

### Promotion: Experimental → Provisional → Stable

- [ ] **Verify readiness criteria** (see [Promotion Process](#promotion-process))
  - API unchanged for at least one minor release cycle
  - Test coverage meets project baseline
  - ADR documents the design rationale (if new module)
- [ ] **Update `std/manifest.toml`**
  - Set `stability = "<new_tier>"` on each affected `[[functions]]` entry
  - If promoting an entire module, also update the `[[modules]]` entry's `stability`
  - Ensure no function stability exceeds its module stability
- [ ] **Verify fixture coverage**
  - Confirm at least one `tests/fixtures/*.ark` file exercises the promoted function(s)
  - For `host_stub` functions, confirm fixture tests the stub behavior (compile error, runtime trap, etc.)
- [ ] **Regenerate documentation**
  - Run `python3 scripts/gen/generate-docs.py`
  - Verify `docs/stdlib/reference.md` reflects the new tier in its canonical category table and overview counts
  - Verify `docs/stdlib/modules/*.md` pages show updated stability
- [ ] **Run CI checks**
  - `python3 scripts/gen/generate-docs.py --check` must pass
  - `python3 scripts/check/check-docs-consistency.py` must pass
  - `python3 scripts/manager.py verify quick` must pass
- [ ] **Review generated module pages** for the affected module

### Deprecation: Any → Deprecated

- [ ] **Add `deprecated_by` field** to the `[[functions]]` entry in `std/manifest.toml`
  - Value must be the canonical replacement function name (e.g., `"Vec::new<i32>"`)
- [ ] **Set `stability = "deprecated"`** on the same entry
- [ ] **Add migration example** to `docs/stdlib/migration-guidance.md`
- [ ] **Regenerate documentation**
  - Run `python3 scripts/gen/generate-docs.py`
  - Verify `~~strikethrough~~` and `⚠️ Deprecated` badge appear in reference.md
  - Verify the Deprecated APIs summary section lists the function
- [ ] **Run CI checks** (same as above)

### Demotion: Stable → Experimental (breaking change)

- [ ] **Confirm this is a major version boundary** — demoting a stable API is a breaking change
- [ ] **Update `std/manifest.toml`** — set `stability = "experimental"`
- [ ] **Document the reason** in a brief ADR note
- [ ] **Regenerate documentation and run CI checks** (same as above)

### Pre-commit Verification Summary

After any stability tier change, run these commands and confirm all pass:

```bash
python3 scripts/gen/generate-docs.py          # regenerate all docs
python3 scripts/gen/generate-docs.py --check  # verify freshness
python3 scripts/check/check-docs-consistency.py # verify metadata consistency
python3 scripts/manager.py verify quick    # verify test harness
```
