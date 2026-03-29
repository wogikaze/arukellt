# Contributing to Arukellt

## Prerequisites

- Rust stable toolchain (install via [rustup](https://rustup.rs/))
- `cargo`, `clippy`, `rustfmt`
- `python3`
- `npx` / `markdownlint-cli2` for the markdown check used by the harness

## Start Here

Before changing behavior, read:

- [`current-state.md`](current-state.md) — current verified project behavior
- [`process/policy.md`](process/policy.md) — operational / verification policy
- [`README.md`](README.md) — docs index and quick links

## Quick Start

```bash
# Build the CLI
cargo build --release -p arukellt

# Run the repository verification contract
bash scripts/verify-harness.sh

# Run a sample program
./target/release/arukellt run docs/examples/hello.ark
```

## Common Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --exclude ark-llvm -- -D warnings
cargo test --workspace --exclude ark-llvm
bash scripts/verify-harness.sh --quick
bash scripts/verify-harness.sh
python3 scripts/generate-docs.py
python3 scripts/check-docs-consistency.py
python3 scripts/collect-baseline.py
```

## Project Structure

```text
crates/
  ark-lexer/        # tokenizer
  ark-parser/       # parser / AST
  ark-resolve/      # name resolution, imports, module loading
  ark-typecheck/    # type checking
  ark-hir/          # shared HIR crate
  ark-mir/          # MIR lowering / validation / optimization
  ark-wasm/         # Wasm backend + component / WIT emit
  ark-target/       # target registry + backend planning
  ark-diagnostics/  # diagnostics registry + rendering
  ark-driver/       # session / orchestration
  ark-stdlib/       # stdlib support crate
  ark-lsp/          # LSP scaffold
  ark-llvm/         # LLVM backend scaffold (excluded from default verification)
  arukellt/         # CLI entry point
std/                # source-backed stdlib wrappers and manifest
tests/fixtures/     # manifest-driven fixtures
benchmarks/         # perf cases
scripts/            # verification / generation utilities
docs/               # user-facing and design docs
```

## Fixtures and Baselines

- `tests/fixtures/manifest.txt` is the single source of truth for fixture entry points.
- The harness is manifest-driven; do not assume globbing.
- Current totals are derived dynamically by `scripts/verify-harness.sh` and surfaced in [`current-state.md`](current-state.md).
- Fixture kinds currently include:
  - `run`
  - `diag`
  - `module-run`
  - `module-diag`
  - `t3-compile`
  - `t3-run`
  - `component-compile`
  - `compile-error`
  - `bench`
- Baselines live under `tests/baselines/`.

## Documentation Workflow

Some docs are generated and should be regenerated rather than hand-edited.

Use:

```bash
python3 scripts/generate-docs.py
python3 scripts/check-docs-consistency.py
```

This updates / validates generated landing pages, README status blocks, sidebar content, and manifest-backed stdlib reference material.

## Verification Contract

All behavior changes should pass:

```bash
bash scripts/verify-harness.sh
```

The harness covers, among other checks:

- docs structure and docs drift
- formatting (`cargo fmt`)
- lint (`cargo clippy`)
- workspace tests
- manifest-driven fixture execution
- stdlib manifest checks
- baseline collection smoke

Use `--quick` while iterating, then run the full harness before finishing.

## Perf Policy

Baseline compile-time cases are:

- `docs/examples/hello.ark`
- `docs/examples/vec.ark`
- `docs/examples/closure.ark`
- `docs/sample/parser.ark`

Thresholds:

- `arukellt check`: median regression must stay within 10%
- `arukellt compile`: median regression must stay within 20%

Heavy perf comparison belongs outside the default correctness gate.

## Hidden Developer Tooling

These are developer/debug aids, not stable public CLI options:

- `ARUKELLT_DUMP_PHASES=parse,resolve,corehir,mir,optimized-mir,backend-plan`
- `ARUKELLT_DUMP_DIAGNOSTICS=1`

## Compatibility Notes

- `docs/current-state.md` is the current behavior contract.
- Historical roadmap / migration / completion docs should not override current-state.
- `W0004` is a hard error: backend validation failure is build-breaking, not warning-only.
