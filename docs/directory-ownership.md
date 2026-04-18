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
| `docs/stdlib/scoreboard.md` | generated | `scripts/gen/generate-scoreboard.sh` | Module maturity scoreboard |
| `docs/data/` | generated | `scripts/gen/generate-docs.py` | Project state data |
| `docs/spec/` | archive | — | Previous version specs |
| `docs/migration/` | archive | — | Version migration guides |
| `docs/adr/` | product | — | Architecture Decision Records |
| `extensions/arukellt-all-in-one/` | product | — | VS Code extension |
| `harness/` | internal | — | Test harness configuration |
| `issues/open/` | internal | `scripts/gen/generate-issue-index.sh` | Active issue tracking |
| `issues/done/` | internal | `scripts/gen/generate-issue-index.sh` | Completed issue archive |

## Generated Files

These files are auto-generated. Run the generator instead of editing manually:

| File | Generator |
|------|-----------|
| `docs/stdlib/reference.md` | `python3 scripts/gen/generate-docs.py` |
| `docs/stdlib/modules/*.md` | `python3 scripts/gen/generate-docs.py` |
| `docs/stdlib/scoreboard.md` | `bash scripts/gen/generate-scoreboard.sh` |
| `issues/open/index.md` | `bash scripts/gen/generate-issue-index.sh` |
| `issues/open/index-meta.json` | `bash scripts/gen/generate-issue-index.sh` |
| `issues/done/index.md` | `bash scripts/gen/generate-issue-index.sh` |
| `issues/open/dependency-graph.md` | `bash scripts/gen/generate-issue-index.sh` |

## Excluded from Default Build

| Directory | Reason | How to Include |
|-----------|--------|----------------|
| `crates/ark-llvm/` | Requires LLVM 18 | `cargo build -p ark-llvm` |
| `crates/ark-lsp/` | Scaffold, not CI-tested | `cargo build -p ark-lsp` |
