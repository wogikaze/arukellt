# ADR-016: Breaking Change Process — Two-Piece Set

ステータス: **DECIDED** — Breaking Change Process（2要素セット）を採用  
**Created**: 2026-04-09  
**Updated**: 2026-04-28 — CHANGELOG要件を削除（v0.1段階のため不要）
**Scope**: Language, stdlib, CLI, LSP, extension

## Context

Without a defined process, breaking changes enter the codebase without notice.
Users discover breakage by accident. Migration paths are undocumented.

## Definition of Breaking Change

A change is **breaking** if it causes any of the following for code that worked in
the prior stable release:

- **Syntax**: valid source code no longer parses or compiles
- **Semantics**: the runtime behavior of existing code changes
- **API**: a stable or provisional stdlib function, method, or type is removed or
  renamed; its signature changes incompatibly; or its behavior changes in a
  user-observable way
- **CLI**: a flag that existed in the prior release is removed, renamed, or changes
  its default value
- **LSP protocol**: a capability that the prior release advertised is removed
- **ABI/Wasm output**: the Wasm binary layout changes in a way incompatible with
  previously compiled modules (component boundary ABI)

The following are **NOT** breaking:
- Adding new stdlib functions, new CLI flags, new LSP capabilities
- Bug fixes that make incorrect behavior correct (even if someone relied on the bug)
- Performance changes without observable behavior differences
- Changes to `experimental` or unimplemented features (see ADR-014)

## The Two-Piece Set Rule

Every breaking change (to a `stable` or `provisional` feature) **must** ship with:

### 1. Issue

- An issue must exist documenting the breaking change *before* the PR lands
- Issue title must include `[breaking]`
- Issue must describe: what breaks, why it's necessary, the migration path

### 2. Migration Guide

For breaking changes that affect user code:
- If a single file change is sufficient: add a `## Migration` section to the issue
- If multiple files need updating: create `docs/migration/<from>-to-<to>.md`
- The migration guide must include: before/after code examples, automated migration
  steps if possible, and a timeline for the deprecation period

## Deprecation Period

For `stable` features, the minimum deprecation period is **one minor version**:
1. Version N: feature works, deprecation warning added
2. Version N+1: feature is removed

For `provisional` features, no deprecation period is required (but a migration guide is still required).

## Enforcement

- PR reviewers must reject PRs that introduce breaking changes without the two-piece set
- The issue template includes a `[breaking]` checkbox

## References

- `docs/release-criteria.md`
- `docs/adr/ADR-014-stability-labels.md`
- `issues/done/230-breaking-change-process-three-piece-set.md`
