# Bootstrap Verification

This document describes the staged bootstrap pipeline and fixpoint
verification methodology used to validate the Arukellt self-hosted compiler.

## Goal

Prove that the self-hosted compiler is correct by reaching a **fixpoint**:
compiling the compiler with itself produces a bit-identical binary.

## Stages

| Stage | Input              | Tool                   | Output            | Status      |
|-------|--------------------|------------------------|-------------------|-------------|
| 0     | `lexer.ark`        | Rust compiler          | `lexer.wasm`      | Implemented |
| 1     | `lexer.wasm`       | wasmtime               | exit 0            | Implemented |
| 2     | `parser.ark`       | Rust compiler          | `parser.wasm`     | Placeholder |
| 3     | `compiler.ark` × 3 | Stage-N compilers      | fixpoint diff     | Placeholder |

### Stage 0 — Compile with Rust compiler

The Rust-hosted compiler (`target/release/arukellt`) compiles
`src/compiler/lexer.ark` into `src/compiler/lexer.wasm`.  This is the
**trusted base**: if the Rust compiler is correct, the output is correct.

### Stage 1 — Execute under wasmtime

Run `lexer.wasm` under wasmtime.  A zero exit code confirms the compiled
module is structurally valid and executes without trapping.

### Stage 2 — Parser compilation (placeholder)

Once `src/compiler/parser.ark` exists, Stage 2 will compile and execute it
the same way Stages 0–1 handle the lexer.

### Stage 3 — Fixpoint verification (placeholder)

When a complete self-hosted compiler exists (`compiler.ark`), the fixpoint
check proceeds as follows:

```
  Rust compiler  ──compile──▶  compiler-s0.wasm   (Stage 0 output)
  compiler-s0    ──compile──▶  compiler-s1.wasm   (first self-compile)
  compiler-s1    ──compile──▶  compiler-s2.wasm   (second self-compile)

  diff compiler-s1.wasm compiler-s2.wasm  →  must be identical
```

If `compiler-s1.wasm` and `compiler-s2.wasm` are byte-identical, the
compiler is a **fixpoint**: it reproduces itself when it compiles itself.
This is the strongest practical proof of compiler correctness short of
formal verification.

### Determinism requirement

Fixpoint verification only works when compilation is **deterministic**.
The compiler must not embed timestamps, random nonces, or pointer-derived
values into the output binary.  Determinism is verified by `scripts/verify-harness.sh`
(which compiles fixtures twice and asserts byte-identical output) and will be extended to
cover the self-hosted compiler once it exists.

## Running

```bash
# All stages
scripts/verify-bootstrap.sh

# Single stage
scripts/verify-bootstrap.sh --stage=0
```

## Integration with verify-harness

`scripts/verify-harness.sh` is the top-level completion gate.  Once
bootstrap stages are stable, the harness will invoke `verify-bootstrap.sh`
as a sub-check so that self-hosting regressions block the build.

## Artifact naming convention

| Artifact            | Path                          | Producer         |
|---------------------|-------------------------------|------------------|
| `lexer.wasm`        | `src/compiler/lexer.wasm`     | Rust compiler    |
| `parser.wasm`       | `src/compiler/parser.wasm`    | Rust compiler    |
| `compiler-s0.wasm`  | (build dir, not committed)    | Rust compiler    |
| `compiler-s1.wasm`  | (build dir, not committed)    | compiler-s0.wasm |
| `compiler-s2.wasm`  | (build dir, not committed)    | compiler-s1.wasm |

Intermediate fixpoint artifacts live in a temporary build directory and are
cleaned up after each verification run.  Only the canonical `lexer.wasm`
and `parser.wasm` are committed to the repository.

## Failure policy

- Any non-zero exit from a stage marks the entire bootstrap check as **FAIL**.
- On failure the script exits with status 1 and prints which stage failed.
- When integrated with CI, a bootstrap failure blocks merge.
