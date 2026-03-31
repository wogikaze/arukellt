# Repository Retention Policy

> Defines what is kept, archived, or removed as the repository grows.

## Scope

This policy covers:
- Completed issues (`issues/done/`)
- Historical documentation (`docs/spec/`, `docs/migration/`)
- Architecture Decision Records (`docs/adr/`)
- Build artifacts and generated files

## Rules

### Completed Issues (`issues/done/`)

- **Retained permanently**: All completed issue files remain in `issues/done/`.
- **Rationale**: They serve as audit trail and design history.
- **Format**: Each file must have `Status: done` and all acceptance criteria checked.

### Architecture Decision Records (`docs/adr/`)

- **Retained permanently**: ADRs are append-only.
- **Superseded ADRs**: Keep in place with `Status: superseded` and link to replacement.
- **Never delete**: Even rejected ADRs provide decision context.

### Historical Documentation (`docs/spec/`)

- **Archive boundary**: `docs/spec/` is the archive zone.
- **Marking**: Files must have an archive banner at the top:
  ```
  > **Archive**: This document describes a previous version. See [current-state](current-state.md).
  ```
- **No deletion**: Historical docs remain for reference.

### Migration Documentation (`docs/migration/`)

- **Retained for 2 major versions**: Migration guides from version N are kept until version N+2.
- **After retention period**: Move to `docs/spec/archive/` with archive banner.
- **Active guides**: Must link to current `docs/current-state.md`.

### Generated Files

- **Regenerate, don't hand-edit**: Files marked `Do not edit manually` must be regenerated via their generator script.
- **Not committed to history**: Generated files can be regenerated from source at any time.
- **Generator scripts**: Must be documented and runnable.

### Build Artifacts

- **Never committed**: `target/`, `node_modules/`, `.vscode-test/` must be in `.gitignore`.
- **Benchmark results**: Stored in `benchmarks/results/` with schema versioning.
- **CI artifacts**: Ephemeral, not stored in repository.

### Large Files

- **Budget**: No single file should exceed 1MB without justification.
- **Binary assets**: Prefer external hosting or build-time download.
- **Monitoring**: `scripts/check-orphan-inventory.sh` reports files > 500KB.

## Archive Workflow

When moving content to archive:

1. Add archive banner to the document header
2. Update any inbound links to point to the current version
3. Move file to `docs/spec/archive/` or appropriate archive location
4. Record the move date and reason in a commit message

## Review Schedule

- Review `issues/done/` count at each milestone
- Review `docs/migration/` retention at each major version
- Run `scripts/check-orphan-inventory.sh` monthly
