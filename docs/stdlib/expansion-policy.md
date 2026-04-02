# Standard Library Expansion Policy

> Defines which module families accept new APIs, under what conditions,
> and which are frozen or maintenance-only.

## Family Classification

| Family | Label | Description |
|--------|-------|-------------|
| `prelude` | maintenance | Core builtins. Changes require ADR. |
| `std::core` | maintenance | Error, hash, ordering primitives. Frozen surface. |
| `std::collections` | expansion | Vec/HashMap/linear/ordered operations. Accepts new generic ops. |
| `std::text` | expansion | String processing. Accepts Unicode-aware additions. |
| `std::seq` | expansion | Iterator/sequence combinators. Accepts new combinators. |
| `std::path` | maintenance | Path manipulation. Stable surface. |
| `std::json` | expansion | JSON stringify/parse. Accepts builder/query additions. |
| `std::toml` | expansion | TOML line parsing. Accepts structured parse additions. |
| `std::csv` | maintenance | CSV line splitting. Minimal surface. |
| `std::bytes` | expansion | Binary data helpers. Accepts encoding/protocol additions. |
| `std::test` | expansion | Test assertions. Accepts new matchers/helpers. |
| `std::time` | maintenance | Time formatting. Stable surface. |
| `std::random` | maintenance | Pure random utilities. Stable surface. |
| `std::wasm` | expansion | Wasm binary format helpers. Active development. |
| `std::wit` | expansion | WIT type constants. Active development. |
| `std::component` | expansion | Component model ABI helpers. Active development. |
| `std::host::stdio` | maintenance | Console I/O. Stable surface. |
| `std::host::fs` | expansion | File system operations. Accepts path/metadata additions. |
| `std::host::env` | expansion | Environment variable access. Accepts new env operations. |
| `std::host::process` | maintenance | Process exit. Minimal surface. |
| `std::host::clock` | maintenance | Clock/time access. Stable surface. |
| `std::host::random` | maintenance | OS random source. Minimal surface. |
| `std::host::http` | expansion | HTTP client. WASI P2 dependent. Experimental. |
| `std::host::sockets` | expansion | TCP/UDP sockets. WASI P2 dependent. Experimental. |

## Label Definitions

### Expansion

- Actively accepts new function additions
- Must meet admission gate criteria (see below)
- New APIs start as `experimental` stability

### Maintenance

- Surface is considered stable and complete for current use
- Bug fixes and documentation improvements accepted
- New functions require ADR justification

### Frozen

- No changes accepted except critical bug fixes
- API is stable and backward-compatible
- Currently no families are frozen

## API Addition Gate Conditions

Every new stdlib function must satisfy ALL of:

1. **Fixture**: At least one test fixture in `tests/fixtures/` exercises the function
2. **Manifest**: Entry in `std/manifest.toml` with all required fields
3. **Docs**: Generated reference includes the function (via `generate-docs.py`)
4. **Target**: Function works on all declared targets (check `target` field)
5. **Naming**: Follows naming conventions in `docs/stdlib/stability-policy.md`
6. **Stability**: Marked as `experimental` for new additions

## See Also

- [Stability Policy](stability-policy.md)
- [Monomorphic Deprecation](monomorphic-deprecation.md)
- [API Reference](reference.md)
