# Bootstrap Guide

> Self-hosting verification for the Arukellt compiler.

## Overview

The Arukellt v5 compiler is written in Arukellt itself (`src/compiler/*.ark`).
Bootstrap verification proves correctness by reaching a **fixpoint**: the
self-hosted compiler produces a bit-identical binary when it compiles itself.

```
Stage 0 (Rust compiler)
  └─ compiles src/compiler/*.ark → arukellt-s1.wasm   (trusted base)

Stage 1 (arukellt-s1.wasm under wasmtime)
  └─ compiles src/compiler/*.ark → arukellt-s2.wasm   (first self-compile)

Stage 2 (fixpoint check)
  └─ sha256(arukellt-s1.wasm) == sha256(arukellt-s2.wasm)
```

If `arukellt-s1.wasm` and `arukellt-s2.wasm` are byte-identical, the compiler
is a **fixpoint**: it reproduces itself when it compiles itself.  This is the
strongest practical proof of compiler correctness short of formal verification.

## Prerequisites

| Tool | Purpose | Install |
|------|---------|---------|
| Rust toolchain (rustc 1.85+) | Build the Stage 0 Rust compiler | `rustup` or `mise` |
| wasmtime | Execute `.wasm` artifacts | `mise` installs automatically |
| sha256sum | Compare fixpoint checksums | coreutils (pre-installed on Linux) |

## Quick Start

```bash
# Build the Rust compiler first
cargo build --release

# Stage 0 only — Rust compiles selfhost sources
scripts/verify-bootstrap.sh --stage1-only

# Full fixpoint verification (Stages 0 → 1 → 2)
scripts/verify-bootstrap.sh

# Run a single stage
scripts/verify-bootstrap.sh --stage 0
```

## Verification Stages

### Stage 0 — Compile with Rust compiler

The Rust-hosted compiler (`target/release/arukellt`) compiles each
`src/compiler/*.ark` file.  This is the **trusted base**: if the Rust
compiler is correct, the output is correct.

When `main.ark` produces a unified `main.wasm`, it is copied to
`.bootstrap-build/arukellt-s1.wasm` for the next stage.

### Stage 1 — Self-compile with arukellt-s1.wasm

The Stage 0 artifact (`arukellt-s1.wasm`) is executed under wasmtime to
compile the same selfhost sources again.  The output is
`.bootstrap-build/arukellt-s2.wasm`.

> **Note:** Stage 1 is skipped automatically when the selfhost compiler
> is not yet mature enough to produce a unified binary.

### Stage 2 — Fixpoint check

Compare `sha256(arukellt-s1.wasm)` with `sha256(arukellt-s2.wasm)`.
Identical checksums prove the compiler is a fixpoint.

### Determinism requirement

Fixpoint verification only works when compilation is **deterministic**.
The compiler must not embed timestamps, random nonces, or pointer-derived
values into the output binary.  `scripts/verify-harness.sh` already
checks determinism for fixtures and will be extended to the selfhost
compiler.

## Selfhost Compiler Components

| File | Role |
|------|------|
| `lexer.ark` | Tokenizer — character stream → token stream |
| `parser.ark` | Recursive descent + Pratt parser → AST |
| `resolver.ark` | Name resolution and scope management |
| `typechecker.ark` | Type inference and unification |
| `hir.ark` | High-level IR data structures |
| `mir.ark` | Mid-level IR and HIR→MIR lowering |
| `emitter.ark` | MIR → Wasm binary emission |
| `driver.ark` | Pipeline orchestration (lex→parse→…→emit) |
| `main.ark` | CLI entry point and argument parsing |

## Debug Procedures

### Dump compiler phases

Both compilers support phase dumping for debugging.

**Rust compiler** — uses the `ARUKELLT_DUMP_PHASES` environment variable
(output on stderr):

```bash
# Available phases: parse, resolve, corehir, mir, optimized-mir, backend-plan
ARUKELLT_DUMP_PHASES=parse cargo run -- compile hello.ark
ARUKELLT_DUMP_PHASES=mir,optimized-mir cargo run -- compile hello.ark
```

**Selfhost compiler** — uses the `--dump-phases` CLI flag
(output on stderr):

```bash
# Available phases: tokens, ast, hir, mir, wasm
wasmtime arukellt-s1.wasm -- compile --dump-phases tokens hello.ark
wasmtime arukellt-s1.wasm -- compile --dump-phases ast hello.ark
```

### Compare Rust vs selfhost output

Use `scripts/compare-outputs.sh` to diff phase output between the two
compilers:

```bash
# Compare token output for hello.ark
scripts/compare-outputs.sh tokens tests/fixtures/hello/hello.ark

# Compare AST output
scripts/compare-outputs.sh ast tests/fixtures/hello/hello.ark

# Use a custom selfhost binary
scripts/compare-outputs.sh mir hello.ark --selfhost-wasm ./arukellt-s1.wasm
```

The script runs both compilers, captures stderr, and shows a unified diff.
Exit code 0 means identical output; exit code 1 means divergence.

### Investigate fixpoint failures

When Stage 2 reports that `arukellt-s1.wasm ≠ arukellt-s2.wasm`:

1. **Find the divergent phase** — run `scripts/compare-outputs.sh` for
   each phase (`tokens`, `ast`, `hir`, `mir`, `wasm`) against a small
   fixture to locate the first phase where output differs.

2. **Narrow the fixture** — use the smallest fixture that reproduces the
   divergence (start with `tests/fixtures/hello/hello.ark`).

3. **Compare phase output** — dump the divergent phase from both Stage 0
   and Stage 1 compilers:

   ```bash
   # Stage 0 (Rust)
   ARUKELLT_DUMP_PHASES=mir target/release/arukellt compile fixture.ark 2>s0.mir
   # Stage 1 (selfhost)
   wasmtime .bootstrap-build/arukellt-s1.wasm -- compile --dump-phases mir fixture.ark 2>s1.mir
   diff -u s0.mir s1.mir
   ```

4. **Fix and verify** — correct the selfhost source and re-run:

   ```bash
   scripts/verify-bootstrap.sh
   ```

## Artifact Naming Convention

| Artifact | Path | Producer |
|----------|------|----------|
| `lexer.wasm` | `src/compiler/lexer.wasm` | Rust compiler |
| `arukellt-s1.wasm` | `.bootstrap-build/arukellt-s1.wasm` | Rust compiler (Stage 0) |
| `arukellt-s2.wasm` | `.bootstrap-build/arukellt-s2.wasm` | arukellt-s1.wasm (Stage 1) |

Intermediate fixpoint artifacts live in `.bootstrap-build/` and are cleaned
up after each verification run.  Only component `.wasm` files (e.g.
`lexer.wasm`) are committed to the repository.

## CI Integration

The bootstrap check is available via `scripts/verify-bootstrap.sh`:

| Context | Command | Notes |
|---------|---------|-------|
| PR checks | `scripts/verify-bootstrap.sh --stage1-only` | Fast — Rust compilation only |
| Merge to main | `scripts/verify-bootstrap.sh` | Full fixpoint when available |
| Local dev | `scripts/verify-bootstrap.sh --stage 0` | Single stage |

The `--stage1-only` flag is suitable for PR checks (faster), while full
fixpoint verification runs on merge to main.

## Verification Scripts

| Script | Purpose | Issue |
|--------|---------|-------|
| `scripts/verify-bootstrap.sh` | Stage 0→1→2 fixpoint verification | #181, #182 |
| `scripts/compare-outputs.sh` | Phase output diff (Rust vs selfhost) | #174 |
| `scripts/verify-harness.sh` | Top-level verification gate | — |

## See Also

- [Bootstrap Verification Process](../process/bootstrap-verification.md)
- [Migration Guide: v4 → v5](../migration/v4-to-v5.md)
- [Compiler Pipeline](pipeline.md)
- [IR Specification](ir-spec.md)
- [ADR-0001: Harness Bootstrap](../adr/ADR-0001-harness-bootstrap.md)

## Selfhost Completion Criteria

The selfhost compiler is **complete** when **all** of the following conditions
are satisfied simultaneously and verified by CI on every merge to `master`:

| Criterion | Description | Verification script/command |
|-----------|-------------|----------------------------|
| **Stage 0 compile** | Rust compiler compiles all `src/compiler/*.ark` files with zero errors | `scripts/verify-bootstrap.sh --stage1-only` |
| **Stage 1 compile** | `arukellt-s1.wasm` compiles all `src/compiler/*.ark` files with zero errors | `scripts/verify-bootstrap.sh` Stage 1 |
| **Stage 2 fixpoint** | `sha256(s1) == sha256(s2)` — compiler reproduces itself byte-for-byte | `scripts/verify-bootstrap.sh` Stage 2 |
| **Fixture parity** | Selfhost compiler passes all 575+ fixture tests identically to the Rust compiler | `scripts/check-selfhost-parity.sh` |
| **CLI parity** | `arukellt-s1.wasm compile <file>` stdout/stderr matches `arukellt compile <file>` for all fixture inputs | `scripts/check-selfhost-parity.sh --cli` |
| **Diagnostic parity** | Error message text, line/column positions, and exit codes match for all error fixtures | `scripts/check-selfhost-parity.sh --diag` |
| **Determinism** | Running Stage 0 twice on the same input produces identical bytes | part of `verify-bootstrap.sh` Stage 2 |

**One-line definition:** The selfhost compiler is complete when
`scripts/verify-bootstrap.sh` exits 0 with all stages passing (no SKIP),
**and** `scripts/check-selfhost-parity.sh` exits 0.

### Current Status (Updated Automatically)

See [docs/current-state.md — Self-Hosting Bootstrap Status](../current-state.md#self-hosting-bootstrap-status)
for the latest stage-by-stage status. That section is the authoritative source.

### What is *not* required for "complete"

- Performance parity with the Rust compiler (acceptable to be slower)
- LLVM backend support in the selfhost compiler
- LSP support in the selfhost compiler
- Identical binary output for *all possible* inputs (only the fixture set)

## Dual-Period Governance

During the dual period, **both** the Rust compiler (`crates/`) and the selfhost
sources (`src/compiler/`) are maintained in parallel. Every bug fix applied to
the Rust compiler must also be applied to the selfhost sources.

### Retained Crates (IDE tooling — never deleted)

The following crates provide IDE and editor integration and remain in
`Cargo.toml` after the dual period ends.

| Crate | Purpose |
|-------|---------|
| `ark-lsp` | Language Server Protocol integration |
| `ark-dap` | Debug Adapter Protocol integration |
| `ark-diagnostics` | Shared diagnostic types (used by LSP/DAP) |
| `ark-manifest` | Project manifest parsing (used by LSP and CLI) |
| `ark-lexer` | Tokenizer — shared between IDE tooling and the compiler |
| `ark-parser` | Parser — shared between IDE tooling and the compiler |

Test infrastructure in `tests/` and `scripts/` is also never removed.

### Deletion Candidates (compiler pipeline)

The following crates implement the Rust compiler pipeline. Each is deleted once
its selfhost equivalent passes the parity check.

| Crate | Role | Deletion Condition |
|-------|------|--------------------|
| `ark-driver` | Pipeline orchestration (lex→parse→…→emit) | Selfhost `driver.ark` equivalent passes `scripts/check-selfhost-parity.sh` |
| `ark-mir` | Mid-level IR and HIR→MIR lowering | Selfhost `mir.ark` equivalent passes `scripts/check-selfhost-parity.sh` |
| `ark-wasm` | Wasm binary emitter | Selfhost `emitter.ark` equivalent passes `scripts/check-selfhost-parity.sh` |
| `arukellt` | Top-level CLI binary | Selfhost `main.ark` (as `arukellt.wasm`) passes `scripts/check-selfhost-parity.sh` |

**Parity check definition:** "passes `scripts/check-selfhost-parity.sh`" means
the script exits 0 for every fixture in `tests/fixtures/` on the current `HEAD`
of `master`.

### When the Dual Period Ends

**One observable condition:** The dual period ends when:

> `scripts/check-selfhost-parity.sh` exits 0 on `HEAD` of `master`.

All other gates (fixpoint check, CLI parity, diagnostic parity, determinism)
are prerequisites that must be satisfied before this command can exit 0. Once
`scripts/check-selfhost-parity.sh` exits 0 in CI, the dual period is over and
the deletion procedure below may begin.

### Rust Compiler Deletion Procedure

Execute the following steps **in order** after the dual-period end condition is
confirmed. Each step must leave the repository in a buildable, test-passing
state before the next step begins.

1. Open a tracking issue: `"chore: remove Rust compiler backend after selfhost promotion"`
2. Delete `crates/ark-driver/` (precondition: selfhost `driver.ark` passes parity check)
3. Delete `crates/ark-mir/` (precondition: selfhost `mir.ark` passes parity check)
4. Delete `crates/ark-wasm/` (precondition: selfhost `emitter.ark` passes parity check)
5. Delete `crates/arukellt/` (precondition: selfhost `main.ark` as `arukellt.wasm` passes parity check)
6. Remove the deleted crates from `Cargo.toml` workspace `members` and `default-members`
7. Update CI: replace `cargo build --workspace` compile step with `wasmtime run arukellt.wasm`
8. Update `scripts/verify-harness.sh` to invoke the selfhost binary
9. Update `docs/current-state.md` to remove the dual-period sections
10. Archive `issues/done/` for all selfhost promotion issues
