# Contributing to Arukellt

## Prerequisites

- Rust stable toolchain (install via [rustup](https://rustup.rs/))
- `cargo`, `clippy`, `rustfmt` components

## Quick Start

```bash
# Clone and build
git clone <repo-url>
cd arukellt
cargo build --release

# Run verification
bash scripts/verify-harness.sh

# Run a program
target/release/arukellt run tests/fixtures/hello/hello.ark
```

## Task Runner (mise)

If you have [mise](https://mise.jdx.dev/) installed:

```bash
mise run build:release    # Build release binary
mise run test:unit        # Run unit tests
mise run test:t1          # Run T1 fixture suite
mise run verify           # Full verification harness
```

## Raw Commands

```bash
cargo fmt --all --check       # Check formatting
cargo clippy --workspace -- -D warnings  # Lint
cargo build --workspace       # Build
cargo test --workspace        # Unit tests
bash scripts/verify-harness.sh  # Full verification (includes above)
```

## Project Structure

```
crates/
  ark-lexer/       # Tokenizer
  ark-parser/      # Recursive descent parser
  ark-resolve/     # Name resolution + module loading
  ark-typecheck/   # Type checking + trait resolution
  ark-mir/         # MIR lowering
  ark-wasm/        # Wasm code generation
    src/emit/      # Per-target emitters (T1, T3)
    src/component/ # WIT generation
  ark-target/      # Target registry
  ark-diagnostics/ # Error codes + rendering
  ark-stdlib/      # Standard library definitions
  arukellt/        # CLI entry point
```

## Adding a Test Fixture

1. Create `tests/fixtures/<category>/<name>.ark`
2. Create `tests/fixtures/<category>/<name>.expected` with expected stdout
3. For diagnostic tests, create `tests/fixtures/<category>/<name>.diag`
4. Run `bash scripts/verify-harness.sh` to verify

## Verification Contract

All PRs must pass `scripts/verify-harness.sh` with exit code 0. This includes:
- Code formatting (`cargo fmt`)
- Linting (`cargo clippy`)
- All unit tests (`cargo test`)
- All fixture tests (169+ pass, 0 fail)
- Documentation structure checks
