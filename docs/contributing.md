# Contributing to Arukellt

## Prerequisites

- Rust stable toolchain (install via [rustup](https://rustup.rs/))
- `cargo`, `clippy`, `rustfmt` components

## Quick Start

```bash
# Clone and build
git clone <repo-url>
cd arukellt
cargo build --release -p arukellt

# Run verification
bash scripts/verify-harness.sh

# Run a program
target/release/arukellt run tests/fixtures/hello/hello.ark
```

## Raw Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --exclude ark-llvm -- -D warnings
cargo build --workspace --exclude ark-llvm
cargo test --workspace --exclude ark-llvm
bash scripts/verify-harness.sh
python3 scripts/collect-baseline.py
python3 scripts/check-docs-consistency.py
```

## Project Structure

```text
crates/
  ark-lexer/        # Tokenizer
  ark-parser/       # Parser / AST
  ark-resolve/      # Bind/Load/Analyze/Resolve refactor target area
  ark-typecheck/    # Type checking (+ planned CoreHIR build)
  ark-hir/          # Planned shared CoreHIR crate for this refactor
  ark-mir/          # MIR lowering / validation / optimization boundary
  ark-wasm/         # Wasm backend emit + backend validation
  ark-target/       # Target registry + backend planning boundary
  ark-diagnostics/  # Canonical diagnostic registry + rendering
  ark-driver/       # Session / orchestration
  arukellt/         # CLI entry point
```

## Fixture / Baseline Contract

- `tests/fixtures/manifest.txt` is the single source of truth for fixture entry points
- Current manifest size: **346** entries
- Harness kinds:
  - `run`
  - `diag`
  - `module-run`
  - `module-diag`
- Baselines live under `tests/baselines/`
  - `perf-baseline.json`
  - `fixture-baseline.json`
  - `api-baseline.json`

## Verification Contract

All PRs must pass `scripts/verify-harness.sh` with exit code 0.

It includes:

- docs structure checks
- docs consistency drift checks
- formatting (`cargo fmt`)
- lint (`cargo clippy`)
- workspace build/tests
- fixture harness execution
- All fixture tests (346 pass, 0 fail)
- stdlib manifest check
- baseline collection smoke

## Perf Gate Policy

Baseline compile-time cases:

- `docs/examples/hello.ark`
- `docs/examples/vec.ark`
- `docs/examples/closure.ark`
- `docs/sample/parser.ark`

Thresholds:

- `arukellt check`: median compile time regression must stay within 10%
- `arukellt compile`: median compile time regression must stay within 20%

Heavy perf comparison belongs in a separate CI job, not the default correctness gate.

## Diagnostics / Snapshot Tooling

Hidden developer support only:

- `ARUKELLT_DUMP_PHASES=parse,resolve,corehir,mir,optimized-mir,backend-plan`
- `ARUKELLT_DUMP_DIAGNOSTICS=1`

These are for snapshot/debug work and are not stable public CLI options.

## Compatibility Notes

Internal migration docs should keep the following distinction explicit:

- old Session API / direct pipeline calls
- new artifact/query-oriented pipeline surface

Intentional behavior change in this refactor track:

- `W0004` is now a hard error instead of warning-only
