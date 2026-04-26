# ADR-023: Package Registry Resolution Design

ステータス: **DECIDED** — Registry lookupモデル（local > workspace > registry）を採用
**Date**: 2026-04-14
**Decided by**: Module-system track (issue #487)

## Context

Arukellt's dependency resolution (documented in `docs/module-resolution.md` §5)
currently supports only local path dependencies (`{ path = "..." }`). The
manifest format already reserves a version-string syntax for registry
dependencies (`some-pkg = "1.2.3"`), but no resolution logic exists for it.

Issue #487 tracks this gap. This ADR defines the lookup model, failure
diagnostics, and explicit non-goals so that implementation can proceed from a
stable design.

### Current state

- `ark.toml` `[dependencies]` accepts `path` keys only.
- Resolution priority is documented as: local path > workspace member > registry.
- No registry endpoint, protocol, or cache layout is defined.
- ADR-009 separates source-level `use` from Component Model `import`; registry
  resolution operates within the source-level (`use`) layer.

## Decision

### 1. Registry lookup model

Registry resolution activates when a dependency entry is a bare version string:

```toml
[dependencies]
foo = "1.2.3"          # registry dependency
bar = { path = "../bar" }  # local — unchanged
```

Resolution proceeds in this order (unchanged from the documented priority):

1. **Local path** — if `path` key is present, resolve relative to package root.
2. **Workspace member** — if `workspace = true`, resolve within workspace.
3. **Registry** — if the value is a version string, query the registry.

#### Registry query contract

The resolver issues a lookup for `(package-name, version-constraint)` to a
single configured registry endpoint. The endpoint URL is read from the
project-level or user-level configuration:

```toml
# ark.toml (project) or ~/.config/arukellt/config.toml (user)
[registry]
url = "https://registry.arukellt.dev/v1"
```

If no `[registry]` section exists, the resolver uses a compiled-in default URL.

The query is an HTTP GET returning a JSON manifest that includes:

- `name` — package name (must match query)
- `version` — resolved version
- `checksum` — integrity hash (SHA-256)
- `archive_url` — download URL for the package tarball

The resolver downloads and unpacks the archive into a per-user cache directory
(`~/.cache/arukellt/registry/<name>/<version>/`). Subsequent builds reuse the
cache unless the checksum changes.

#### Version constraint syntax

v1 supports exact version strings only (`"1.2.3"`). Semver ranges and
pre-release handling are deferred to a follow-up.

### 2. Failure diagnostics

| Scenario | Error Code | Message pattern |
|----------|-----------|-----------------|
| Registry unreachable (network / timeout) | E0120 | `registry unreachable: {url} ({reason})` |
| Package not found in registry | E0121 | `package '{name}' not found in registry` |
| Version not found | E0122 | `version '{version}' of '{name}' not found in registry` |
| Checksum mismatch after download | E0123 | `integrity check failed for '{name}@{version}'` |
| Registry not configured and no default | E0124 | `no registry configured; add [registry] to ark.toml` |

All errors are compile-time diagnostics (the resolver runs before codegen).
They follow the existing E01xx numbering block used by resolution errors.

When the registry is unreachable but a cached version matching the constraint
exists, the resolver emits a warning and uses the cache (offline-first
fallback).

### 3. Non-goals

The following are explicitly **out of scope** for this design and the initial
implementation:

- **Hosting a registry service** — the registry API contract is defined here,
  but standing up a service is a separate infrastructure concern.
- **Authentication and private registries** — follow-up work; the endpoint
  contract is extensible but v1 assumes public unauthenticated access.
- **Semver range resolution** — v1 accepts exact versions only.
- **Lock file format** — dependency pinning and reproducible builds via a lock
  file are tracked separately.
- **Publishing workflow** — `arukellt publish` and package upload are not part
  of this ADR.
- **Mirroring and fallback registries** — single-endpoint only in v1.

## Rationale

1. **Minimal surface**: Exact-version-only avoids semver solver complexity in
   the initial implementation while still proving the registry path end-to-end.
2. **Offline-first**: Caching with checksum gives deterministic builds when
   the network is unavailable, matching user expectations from Cargo/npm.
3. **Distinct error codes**: Each failure mode gets its own E01xx code so
   diagnostics are actionable without ambiguity.
4. **Config layering**: Project-level `ark.toml` overrides user-level config,
   following the same precedence as other Arukellt settings.
5. **ADR-009 alignment**: Registry packages are imported via `use`, consistent
   with the Layer S (Source) / Layer C (Component) separation.

## Alternatives Considered

### A. Embed registry URL in each dependency entry

```toml
[dependencies]
foo = { version = "1.2.3", registry = "https://custom.example.com" }
```

Rejected for v1 — adds per-dependency complexity. Can be revisited for
multi-registry support later.

### B. Git-based dependency resolution (no registry)

Allow `{ git = "https://...", tag = "v1.2.3" }` as the only remote source.

Rejected as the sole mechanism — git dependencies don't provide namespace
governance or integrity guarantees. May be added as a complementary source
alongside registry.

### C. Vendoring as the only offline story

Require `arukellt vendor` to copy sources into the project tree.

Rejected as the primary mechanism — the checksum-verified cache provides
equivalent reproducibility with less source-tree pollution.

## Implementation Notes

- Error codes E0120–E0124 must be added to the error catalog.
- `docs/module-resolution.md` §5 and §9 should be updated once the resolver
  is wired.
- The cache directory follows XDG conventions on Linux/macOS.
- This ADR does not claim that implementation is complete; it defines the
  target design for issue #487.
