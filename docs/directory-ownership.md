# Directory Ownership Map

> Defines the role, ownership, and maintenance tier of each top-level
> directory in the repository.

## Tier Definitions

| Tier | Meaning |
|------|---------|
| **product** | User-facing, part of the released toolchain |
| **generated** | Auto-generated from source; regenerate, don't hand-edit |
| **internal** | Development infrastructure, not shipped to users |
| **archive** | Historical, retained for reference only |

## Directory Map

| Directory | Tier | Owner/Generator | Description |
|-----------|------|-----------------|-------------|
| `src/compiler/parser.ark` | product | — | Lexer, parser, AST, formatter |
| `src/compiler/resolver.ark` | product | — | Name resolution and module loading |
| `src/compiler/typechecker.ark` | product | — | Type checking and inference |
| `src/compiler/diagnostics.ark` | product | — | Diagnostic codes and messages |
| `std/` | product | — | Standard library source and manifest |
| `std/manifest.toml` | product | — | Canonical stdlib API definition |
| `src/compiler/` | product | — | Selfhost compiler sources (`.ark`) |
| `tests/` | internal | — | Test fixtures and harness |
| `tests/fixtures/` | internal | — | Fixture `.ark` files and `.expected` outputs |
| `tests/fixtures/manifest.txt` | internal | — | Fixture registry |
| `benchmarks/` | internal | — | Performance benchmarks and results |
| `scripts/` | internal | — | Build, test, generation scripts |
| `docs/` | product | — | User and developer documentation |
| `docs/stdlib/reference.md` | generated | `scripts/gen/generate-docs.py` | Stdlib API reference |
| `docs/stdlib/modules/*.md` | generated | `scripts/gen/generate-docs.py` | Per-module reference pages |
| `docs/stdlib/scoreboard.md` | generated | `scripts/gen/generate-docs.py` | Module maturity scoreboard |
| `docs/data/project-state.toml` | product (SSOT input) | hand-maintained | Structured current-state numbers / target profiles |
| `docs/data/sections.toml` | product (SSOT input) | hand-maintained | Docs section registry |
| `docs/data/language-doc-classifications.toml` | product (SSOT input) | hand-maintained | Language doc classifications |
| `docs/data/cli-surface.toml` | product (SSOT input) | hand-maintained | CLI surface SSOT (#770) |
| `docs/data/bootstrap-contract.toml` | product (SSOT input) | hand-maintained | Bootstrap contract SSOT (ADR-029) |
| `docs/data/capabilities.toml` | product (SSOT input) | hand-maintained | Host capability matrix SSOT |
| `docs/data/component-availability.toml` | product (SSOT input) | hand-maintained | Component availability axes |
| `docs/data/release-guarantees.toml` | product (SSOT input) | hand-maintained | Release guarantees SSOT |
| `docs/data/warnings.toml` | product (SSOT input) | hand-maintained | Diagnostic identity and lifecycle SSOT |
| `docs/data/verification-commands.toml` | product (SSOT input) | hand-maintained | Canonical verification command registry |
| `docs/data/docs-gate-config.toml` | internal (SSOT input) | hand-maintained | Documentation hard-gate policy and ratchets |
| `docs/capability-surface.md` | generated | `scripts/gen/generate-structured-state-docs.py` | From capabilities.toml |
| `docs/data/cli-surface.md` | generated | `scripts/gen/generate-structured-state-docs.py` | From cli-surface.toml |
| `docs/data/bootstrap-contract.md` | generated | `scripts/gen/generate-structured-state-docs.py` | From bootstrap-contract.toml |
| `docs/data/component-availability.md` | generated | `scripts/gen/generate-structured-state-docs.py` | From component-availability.toml |
| `docs/data/release-guarantees.md` | generated | `scripts/gen/generate-structured-state-docs.py` | From release-guarantees.toml |
| `docs/data/target-contract-summary.md` | generated | `scripts/gen/generate-docs.py` | Generated from project-state.toml |
| `docs/data/core-ops.toml` | product (SSOT input) | hand-maintained | Core ops registry input |
| `docs/spec/` | archive | — | Previous version specs |
| `docs/adr/` | product | — | Architecture Decision Records |
| `docs/rfcs/` | product | — | 詳細設計提案・仕様草案（ADR の長文側） |
| `docs/plans/` | product | — | 実装フェーズ・PR 計画 |
| `docs/design/` | archive | — | 分類前の設計メモ・比較検討 |
| `extensions/arukellt-all-in-one/` | product | — | VS Code extension |
| `harness/` | internal | — | Test harness configuration |
| `issues/open/` | internal | `python3 scripts/gen/generate-issue-index.py` | Active issue tracking |
| `issues/done/` | internal | `python3 scripts/gen/generate-issue-index.py` | Completed issue archive |

## Generated Files

These files are auto-generated. Run the generator instead of editing manually:

| File | Generator |
|------|-----------|
| `docs/stdlib/reference.md` | `python3 scripts/gen/generate-docs.py` |
| `docs/stdlib/modules/*.md` | `python3 scripts/gen/generate-docs.py` |
| `docs/stdlib/scoreboard.md` | `python3 scripts/gen/generate-docs.py` |
| `issues/open/index.md` | `python3 scripts/gen/generate-issue-index.py` |
| `issues/open/index-meta.json` | `python3 scripts/gen/generate-issue-index.py` |
| `issues/done/index.md` | `python3 scripts/gen/generate-issue-index.py` |
| `issues/open/dependency-graph.md` | `python3 scripts/gen/generate-issue-index.py` |

## Excluded from Default Build

| Directory | Reason | How to Include |
|-----------|--------|----------------|
| _(none)_ | Selfhost LSP via `arukellt lsp` is the source of truth. | — |
