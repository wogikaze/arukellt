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
| `crates/arukellt/` | product | — | CLI binary and entry point |
| `crates/ark-parser/` | product | — | Lexer, parser, AST, formatter |
| `crates/ark-resolve/` | product | — | Name resolution and module loading |
| `crates/ark-typecheck/` | product | — | Type checking and inference |
| `crates/ark-mir/` | product | — | MIR lowering and optimization |
| `crates/ark-wasm/` | product | — | Wasm code generation (T1, T3) |
| `crates/ark-diagnostics/` | product | — | Diagnostic codes and messages |
| `crates/ark-driver/` | product | — | Compilation driver and session |
| `crates/ark-manifest/` | product | — | `ark.toml` manifest parsing |
| `crates/ark-stdlib/` | product | — | Stdlib manifest metadata API |
| `crates/ark-lsp/` | internal | — | LSP server (scaffold, not shipped) |
| `crates/ark-llvm/` | internal | — | LLVM backend (requires LLVM 18, excluded from default build) |
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
| `docs/stdlib/scoreboard.md` | hand-maintained | — | Module maturity scoreboard (auto-generator not yet implemented) |
| `docs/data/` | generated | `scripts/gen/generate-docs.py` | Project state data |
| `docs/spec/` | archive | — | Previous version specs |
| `docs/migration/` | archive | — | Version migration guides |
| `docs/adr/` | product | — | Architecture Decision Records |
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
| `docs/stdlib/scoreboard.md` | manual edit (auto-generator not yet implemented) |
| `issues/open/index.md` | `python3 scripts/gen/generate-issue-index.py` |
| `issues/open/index-meta.json` | `python3 scripts/gen/generate-issue-index.py` |
| `issues/done/index.md` | `python3 scripts/gen/generate-issue-index.py` |
| `issues/open/dependency-graph.md` | `python3 scripts/gen/generate-issue-index.py` |

## Excluded from Default Build

| Directory | Reason | How to Include |
|-----------|--------|----------------|
| `crates/ark-llvm/` | Requires LLVM 18 | `cargo build -p ark-llvm` |
| `crates/ark-lsp/` | Scaffold, not CI-tested | `cargo build -p ark-lsp` |
