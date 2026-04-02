# Standard Library Stability Policy

## Stability Labels

Every module and public API in the Arukellt standard library carries one of three stability labels.

### Stable

- Backward-compatible within a major version.
- Breaking changes only occur on major version bumps.
- Suitable for production use.

### Experimental

- API may change in **minor** versions without notice.
- Functionality is available but the interface is not yet finalized.
- Marked with ⚠️ in documentation.

### Internal

- Used by the compiler and runtime only.
- No public API guarantee — may change or be removed at any time.
- Not intended for user code.

---

## Current Module Classification

### Stable

| Module | Description |
|--------|-------------|
| `prelude` | Auto-imported core functions |
| `std::core` | Option, Result, math, type conversion, panic/assert |
| `std::text` | String manipulation |
| `std::bytes` | Binary data, ByteBuf, ByteCursor, LEB128, encoding |
| `std::collections` | Vec, HashMap, HashSet, Deque (hash/linear/ordered) |
| `std::seq` | Lazy sequence combinators |
| `std::path` | Path manipulation (string-based) |
| `std::time` | Pure duration arithmetic |
| `std::random` | Deterministic seeded helpers |
| `std::test` | Assertions and test utilities |
| `std::host::stdio` | Standard output/error access |
| `std::host::fs` | File read/write |
| `std::host::process` | Process exit/abort |
| `std::host::env` | Environment and CLI arguments |
| `std::host::clock` | Monotonic host clock |
| `std::host::random` | Host entropy |

### Experimental

| Module | Description |
|--------|-------------|
| `std::wasm` | Wasm binary types and builder |
| `std::wit` | WIT type constants and naming |
| `std::component` | Component Model ABI metadata |
| `std::json` | JSON primitive stringify/parse |
| `std::toml` | TOML line parser |
| `std::csv` | CSV line splitter |
| `std::collections::compiler` | Compiler-internal collection variants |

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
2. Keep the deprecated API available for at least one major version.
3. Remove in the next major version with a migration guide.

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
  - Run `python3 scripts/generate-docs.py`
  - Verify `docs/stdlib/reference.md` reflects the new tier in both the per-category and by-stability sections
  - Verify `docs/stdlib/modules/*.md` pages show updated stability
- [ ] **Run CI checks**
  - `python3 scripts/generate-docs.py --check` must pass
  - `python3 scripts/check-docs-consistency.py` must pass
  - `bash scripts/verify-harness.sh --quick` must pass
- [ ] **Update this file** if the module classification table changes
- [ ] **Record the change in CHANGELOG.md**

### Deprecation: Any → Deprecated

- [ ] **Add `deprecated_by` field** to the `[[functions]]` entry in `std/manifest.toml`
  - Value must be the canonical replacement function name (e.g., `"Vec::new<i32>"`)
- [ ] **Set `stability = "deprecated"`** on the same entry
- [ ] **Add migration example** to `docs/stdlib/migration-guidance.md`
- [ ] **Regenerate documentation**
  - Run `python3 scripts/generate-docs.py`
  - Verify `~~strikethrough~~` and `⚠️ Deprecated` badge appear in reference.md
  - Verify the Deprecated APIs summary section lists the function
- [ ] **Run CI checks** (same as above)
- [ ] **Record the change in CHANGELOG.md**

### Demotion: Stable → Experimental (breaking change)

- [ ] **Confirm this is a major version boundary** — demoting a stable API is a breaking change
- [ ] **Update `std/manifest.toml`** — set `stability = "experimental"`
- [ ] **Document the reason** in CHANGELOG.md and a brief ADR note
- [ ] **Regenerate documentation and run CI checks** (same as above)

### Pre-commit Verification Summary

After any stability tier change, run these commands and confirm all pass:

```bash
python3 scripts/generate-docs.py          # regenerate all docs
python3 scripts/generate-docs.py --check  # verify freshness
python3 scripts/check-docs-consistency.py # verify metadata consistency
bash scripts/verify-harness.sh --quick    # verify test harness
```
