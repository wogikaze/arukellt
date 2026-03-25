# AGENTS.md

This document defines the repository boundary, verification contract, and agent harness for this project.

## Repository Boundary

This project is a language toolchain implementing [PROJECT_NAME]. It combines:
- Core compiler/interpreter implementation
- Standard library and documentation
- CLI tools and integration
- Testing and verification infrastructure

## Primary Source of Truth

- **Active queue**: `issues/open/`
- **Completed work**: `issues/done/`
- **Design decisions**: `docs/adr/`
- **Verification contract**: `scripts/verify-harness.sh`
- **Executable examples**: docs with embedded testable examples
- **Generated reference**: toolchain-specific output (e.g., syntax files, stdlib docs)

## Completion Criteria

Work is complete when:
1. `scripts/verify-harness.sh` exits with status 0
2. All newly generated artifacts are committed
3. Relevant ADR or verification docs are updated
4. Queue item is moved from `issues/open/` to `issues/done/`

## Agent Extension Points

When stable:
- `.claude/hooks/` - session initialization (e.g., environment setup)
- `.claude/skills/` - domain-specific workflow extensions
- `.claude/settings.json` - semantic tool configuration

## Focused Verification Loop

- **Quick check**: `scripts/verify-harness.sh --quick`
- **Full verification**: `scripts/verify-harness.sh`
- **Specific domains**: See task runner configuration

## Task Surface

Tasks are organized through:
- `mise.toml` (if polyglot tools needed) or `justfile`
- All external tools should be version-pinned
- Bootstrap must be self-service (no prior knowledge required)

## Tool Preferences

- **Code search**: Use `ig` (index-based grep) instead of `grep` / `rg`. Faster due to pre-built index.
  ```
  ig "async fn.*Result" src/ --stats
  ```

## Documentation Governance

- Behavioral changes require doc/example updates
- Generated docs must have explicit regeneration command and commit policy
- Executable examples must pass verification as part of harness check
