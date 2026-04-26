# Bootstrap Guide

> Self-hosting verification for the Arukellt compiler.

## Overview

The Arukellt v5 compiler is written in Arukellt itself (`src/compiler/*.ark`).
Bootstrap verification proves correctness by reaching a **fixpoint**: the
self-hosted compiler produces a bit-identical binary when it compiles itself.

Process-level scaffold (stage slots, artifact names, failure/diff policy, and
future `verify-harness` integration notes) lives in
[`docs/process/bootstrap-verification.md`](../process/bootstrap-verification.md)
(issue #154, `issues/open/154-bootstrap-verification-scaffold.md`).

**Executable contract:** authoritative artifact paths, the Stage 2 `sha256sum`
comparison rule, and failure/diff behavior (including “no wasm binary diff in
this script”) are defined in the header comments of
[`scripts/run/verify-bootstrap.sh`](../../scripts/run/verify-bootstrap.sh).

```
Stage 0 (Rust compiler)
  └─ compiles src/compiler/main.ark → arukellt-s1.wasm   (trusted base)

Stage 1 (arukellt-s1.wasm under wasmtime)
  └─ compiles src/compiler/main.ark → arukellt-s2.wasm   (first self-compile)

Stage 2 (fixpoint check — standard bootstrap fixpoint)
  └─ arukellt-s2.wasm compiles main.ark → arukellt-s3.wasm
  └─ sha256(arukellt-s2.wasm) == sha256(arukellt-s3.wasm)
```

This is the **standard bootstrap fixpoint**: the selfhost compiler (`s2`) reproduces
itself (`s3`) byte-for-byte. Note that `s1 ≠ s2` is expected — the Rust emitter and
selfhost emitter produce different binary encodings for the same source (e.g. the Rust
emitter includes a function names section; the selfhost emitter uses index-only encoding).
The fixpoint `s2 == s3` proves the selfhost is deterministic and self-consistent.

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

# Stage 0 only — Rust compiles selfhost sources (partial smoke only)
scripts/run/verify-bootstrap.sh --stage1-only

# Full fixpoint verification (Stages 0 → 1 → 2)
scripts/run/verify-bootstrap.sh

# Run a single stage
scripts/run/verify-bootstrap.sh --stage 0
```

**Last audited (stage numbering and CLI flags):** 2026-04-18 against
`scripts/run/verify-bootstrap.sh`.

**Related progress:** Selfhost closure literal parsing and related slices are
tracked in issue **#499** (for example commit `6610945`). That is incremental
parser/front-end alignment work; it does **not** by itself mean Stage 2 fixpoint
(`sha256(s2) == sha256(s3)`)—see
[current-state.md — Self-Hosting Bootstrap Status](../current-state.md#self-hosting-bootstrap-status).

## Verification Stages

### Stage 0 — Compile with Rust compiler

The Rust-hosted `arukellt` binary (see `scripts/run/verify-bootstrap.sh` for
`ARUKELLT_BIN` / `target/debug` / `target/release` resolution) compiles
`src/compiler/main.ark` with `--target wasm32-wasi-p1` and writes
`.bootstrap-build/arukellt-s1.wasm`.  This is the **trusted base**: if the
Rust compiler is correct, the output is correct.

### Stage 1 — Self-compile with arukellt-s1.wasm

The Stage 0 artifact (`arukellt-s1.wasm`) is executed under `wasmtime` (repo
root mounted, per the script) to compile the same `src/compiler/main.ark`
again.  The output is `.bootstrap-build/arukellt-s2.wasm`.

If Stage 1 cannot produce `arukellt-s2.wasm`, `scripts/run/verify-bootstrap.sh`
fails the bootstrap gate explicitly. A partial command such as `--stage1-only`
remains available for smoke checks, but it does not report bootstrap attainment.

### Stage 2 — Fixpoint check

Run `arukellt-s2.wasm` to compile `main.ark` → `arukellt-s3.wasm`.
Compare `sha256(arukellt-s2.wasm)` with `sha256(arukellt-s3.wasm)`.
Identical checksums prove the selfhost compiler is a fixpoint.

### Determinism requirement

Fixpoint verification only works when compilation is **deterministic**.
The compiler must not embed timestamps, random nonces, or pointer-derived
values into the output binary.  `scripts/manager.py` already
checks determinism for fixtures and will be extended to the selfhost
compiler.

## Completion contract (draft)

This section describes **only** what `scripts/run/verify-bootstrap.sh`
implements today (commands, artifacts, exit behaviour).  It does not define
the full **language selfhost complete** bar; for that product-level
definition and checklist themes, see `issues/open/253-selfhost-completion-criteria.md`
and `issues/open/266-selfhost-completion-definition.md`.

**Repository bootstrap gate (this script).** A **full** invocation with no
partial flags runs Stage 0 → 1 → 2 in order.  The script exits **0** only if
every executed stage records success internally and Stage 2 proves
`sha256(arukellt-s2.wasm) == sha256(arukellt-s3.wasm)`.  That gate shows a
**fixpoint for the unified selfhost compiler** built from
`src/compiler/main.ark`; it does **not** by itself prove fixture parity, CLI
parity, diagnostic parity, or Rust compiler retirement.

**Language selfhost complete.** Broader than this script: additional checks
and governance live in the issue pointers above (for example
`issues/open/253-selfhost-completion-criteria.md` and
`issues/open/266-selfhost-completion-definition.md`) and elsewhere in this doc.

### Stages (as the script runs them)

The script runs with `set -euo pipefail` from the repo root (`REPO_ROOT`).
Build outputs go under `.bootstrap-build/` (`arukellt-s1.wasm`,
`arukellt-s2.wasm`, stderr logs); that directory is created before preflight
checks and removed on any exit (`trap 'rm -rf "$BUILD_DIR"' EXIT`).

Stage **0** compiles **only** `src/compiler/main.ark` into the unified
`arukellt-s1.wasm` (the script may print how many `*.ark` files exist under
`src/compiler/`, but it does not compile each file separately in Stage 0).

#### Preflight (exit **1** before Stage 0 runs)

- Unknown CLI flags: error, usage on stderr, exit **1**.
- **`--check`** together with any partial selector (`--stage …`,
  `--stage1-only`, **`--fixture-parity`**): `ERROR:` message on stderr, exit **1**.
- Missing selfhost tree: `src/compiler` directory not found, exit **1**.
- Rust `arukellt` resolution: if `ARUKELLT_BIN` is set, that path is used;
  otherwise the script picks `target/debug/arukellt` if that file exists, else
  `target/release/arukellt` if that file exists; if none of these yield a path,
  it prints `error: no arukellt binary found. Run cargo build -p arukellt.` and
  exits **1**.
- Chosen compiler path not executable: `ERROR:` on stderr, exit **1**.

| Stage | What runs | Artifact(s) | Failure modes (non‑exhaustive) |
|-------|-----------|-------------|----------------------------------|
| **0** | `"$COMPILER" compile "$MAIN_SRC" --target wasm32-wasi-p1 -o "$S1_WASM"` with `MAIN_SRC` = absolute `…/src/compiler/main.ark` and `S1_WASM` = absolute `…/.bootstrap-build/arukellt-s1.wasm` | `arukellt-s1.wasm` | Preflight failures above; compile non‑zero exit; stderr from `.bootstrap-build/stage0.stderr` indented onto stderr when that file is non‑empty. |
| **1** | `timeout 120 wasmtime run --dir="$REPO_ROOT" "$S1_WASM" -- compile "$rel_src" --target wasm32-wasi-p1 -o "$rel_out"` where `rel_src` / `rel_out` are `main.ark` and `.bootstrap-build/arukellt-s2.wasm` relative to `REPO_ROOT` | `arukellt-s2.wasm` | `arukellt-s1.wasm` missing; `command -v wasmtime` fails; wasmtime/compile non‑zero or timeout **124** from `timeout`; stderr from `.bootstrap-build/stage1.stderr` when non‑empty. |
| **2** | `sha256sum` on `$S1_WASM` and `$S2_WASM`; compare digests | (none new) | Either wasm missing; digests differ (script prints sizes and suggests `scripts/run/compare-outputs.sh`). |

### Partial modes, `--check`, and “attainment”

- **`--stage1-only`:** Runs Stage **0** only, then exits **0** on success while
  printing that **bootstrap attainment was not evaluated** (Stages 1–2 do
  not run).
- **`--fixture-parity`:** After Stage 0, if `arukellt-s1.wasm` exists, runs
  `python scripts/manager.py selfhost parity --mode --fixture` when
  `python scripts/manager.py selfhost parity` exists **and** is executable by the
  current user; otherwise prints `SKIP  check-selfhost-parity.sh not found` and
  continues. On success of Stage 0 (and parity, when run), exits like
  `--stage1-only` (no Stage 1–2). If parity exits non‑zero, the bootstrap
  script exits immediately (`set -e`), not with the partial “attainment not
  evaluated” success message.
- **`--stage N`:** Runs only stage *N*; other stages are marked not requested.
  If the requested stage succeeds and any partial-mode condition holds, the
  script may exit **0** while stating that **bootstrap attainment was not
  evaluated**.
- **`--check`:** Allowed only with the full Stage 0 → 1 → 2 run (no partial
  flags; mixing `--check` with `--stage`, `--stage1-only`, or `--fixture-parity`
  is a preflight error, exit **1**). Prints machine-readable lines under
  `bootstrap-check:`: `stage0-compile`, `stage1-self-compile`, and
  `stage2-fixpoint` each set to `reached`, `not-reached`, or `not-requested`,
  plus `attainment: reached` or `attainment: not-reached`, then exits **0** or **1**
  accordingly.

Any path that exits **0** without running the full pipeline is explicitly **not**
a claim of fixpoint attainment; the script states that in its success
message.

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

Use `scripts/run/compare-outputs.sh` to diff phase output between the two
compilers:

```bash
# Compare token output for hello.ark
scripts/run/compare-outputs.sh tokens tests/fixtures/hello/hello.ark

# Compare AST output
scripts/run/compare-outputs.sh ast tests/fixtures/hello/hello.ark

# Use a custom selfhost binary
scripts/run/compare-outputs.sh mir hello.ark --selfhost-wasm ./arukellt-s1.wasm
```

The script runs both compilers, captures stderr, and shows a unified diff.
Exit code 0 means identical output; exit code 1 means divergence.

### Investigate fixpoint failures

When Stage 2 reports that `arukellt-s2.wasm ≠ arukellt-s3.wasm` (non-determinism in selfhost):

1. **Find the divergent phase** — run `scripts/run/compare-outputs.sh` for
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
   scripts/run/verify-bootstrap.sh
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

The bootstrap check is available via `scripts/run/verify-bootstrap.sh`:

| Context | Command | Notes |
|---------|---------|-------|
| Fast smoke (local layered gates, etc.) | `scripts/run/verify-bootstrap.sh --stage1-only` | Stage 0 only; does not prove bootstrap attainment |
| CI `selfhost-bootstrap` job | `scripts/run/verify-bootstrap.sh --check` | Runs Stages 0 → 1 → 2; the workflow asserts Stage 0 and Stage 1 report `reached`; Stage 2 fixpoint is reached; `sha256(s2) == sha256(s3)` (see [current-state.md](../current-state.md#self-hosting-bootstrap-status)) |
| Local dev | `scripts/run/verify-bootstrap.sh --stage 0` | Single stage |

The `--stage1-only` flag is suitable for fast smoke checks. A **full**
invocation (no partial flags) runs Stage 0 → 1 → 2 and exits **0** only when
Stage 2 proves fixpoint (`sha256` match); that is the repository bootstrap
attainment gate. `--check` prints the same stages in machine-readable form
(`stage0-compile`, `stage1-self-compile`, `stage2-fixpoint`, `attainment`) and
rejects partial-mode flags.

## Verification Scripts

| Script | Purpose | Issue |
|--------|---------|-------|
| `scripts/run/verify-bootstrap.sh` | Stage 0→1→2 fixpoint verification | #154, #181, #182 |
| `python scripts/manager.py selfhost fixpoint` | sha256 fixpoint check (s2 vs s3) | #459 |
| `python scripts/manager.py selfhost fixture-parity` | Run "run:" fixtures through s1.wasm; compare stdout to Rust output | #459 |
| `python scripts/manager.py selfhost diag-parity` | Run "diag:" fixtures through s1.wasm; check expected error pattern appears | #459 |
| `scripts/run/compare-outputs.sh` | Phase output diff (Rust vs selfhost) | #174 |
| `scripts/manager.py` | Top-level verification gate | — |

### How to Run the Fixpoint Check

```bash
# Full bootstrap verification: Rust→s1, s1→s2, s2→s3, check sha256(s2)==sha256(s3)
bash scripts/run/verify-bootstrap.sh --check

# Machine-readable output
bash scripts/run/verify-bootstrap.sh --check

# Stage 0 only (smoke check)
bash scripts/run/verify-bootstrap.sh --stage1-only

# Run all selfhost checks via the full harness pass
bash scripts/manager.py --full
# Or individually:
bash scripts/manager.py --fixpoint
bash scripts/manager.py --selfhost-fixture-parity
bash scripts/manager.py --selfhost-diag-parity
```

**SKIP behaviour:** All three scripts exit 0 with a `SKIP:` message when
`arukellt-s1.wasm` is not present at `.build/selfhost/`, so CI does not
hard-fail when the bootstrap stage has not yet been run.  When wired into
`verify-harness.sh --full`, a SKIP counts as a passing check (not a failure).

## See Also

- [Bootstrap Verification Process](../process/bootstrap-verification.md)
- [Migration Guide: v4 → v5](../migration/v4-to-v5.md)
- [Compiler Pipeline](pipeline.md)
- [IR Specification](ir-spec.md)
- [ADR-0001: Harness Bootstrap](../adr/ADR-0001-harness-bootstrap.md)

## Selfhost Completion Criteria

The **Completion contract (draft)** section above states only what
`scripts/run/verify-bootstrap.sh` enforces (**repository bootstrap gate** /
self-compile fixpoint for `main.ark`). This section is the broader **language
selfhost complete** checklist; see also
`issues/open/253-selfhost-completion-criteria.md` and
`issues/open/266-selfhost-completion-definition.md`.

> **Language selfhost complete (product bar):** The selfhost compiler is
> complete when `scripts/run/verify-bootstrap.sh` exits 0 on the full
> Stage 0 → 1 → 2 path (fixpoint attained), **and**
> `python scripts/manager.py selfhost parity` exits 0 on the same commit (fixture /
> CLI / diagnostic parity as defined by that script). That bar is **not**
> implied by the repository bootstrap gate alone.

Passing the full bootstrap script is necessary for the fixpoint portion of
that product bar but does not substitute for parity checks or other governance
called out in the issue pointers above.

The criterion decomposes into these sub-conditions, all of which must hold
simultaneously and be verified by CI on every merge to `master`:

| Criterion | Description | Verification script/command |
|-----------|-------------|----------------------------|
| **Stage 0 compile** | Rust compiler compiles `src/compiler/main.ark` to `.bootstrap-build/arukellt-s1.wasm` with zero errors | `scripts/run/verify-bootstrap.sh --stage1-only` |
| **Stage 1 compile** | `arukellt-s1.wasm` compiles the same `main.ark` to `.bootstrap-build/arukellt-s2.wasm` with zero errors | `scripts/run/verify-bootstrap.sh` Stage 1 |
| **Stage 2 fixpoint** | `sha256(s2) == sha256(s3)` — selfhost compiler reproduces itself byte-for-byte | `scripts/run/verify-bootstrap.sh` Stage 2 |
| **Fixture parity** | Selfhost compiler passes all 575+ fixture tests identically to the Rust compiler | `python scripts/manager.py selfhost parity` |
| **CLI parity** | Rust and selfhost compilers agree on a representative set of CLI surface cases — exact output for `--version`/`--help`, and matching non-zero exit for unknown commands and no-arg `compile`/`check`/`run` | `python scripts/manager.py selfhost parity --mode --cli` |
| **Diagnostic parity** | Error message text, line/column positions, and exit codes match for all error fixtures | `python scripts/manager.py selfhost parity --mode --diag` |
| **Determinism** | Running Stage 0 twice on the same input produces identical bytes | part of `verify-bootstrap.sh` Stage 2 |

### Current Verification Status

See [docs/current-state.md — Self-Hosting Bootstrap Status](../current-state.md#self-hosting-bootstrap-status)
for the authoritative stage-by-stage verification status.

### What is *not* required for "complete"

- Performance parity with the Rust compiler (acceptable to be slower)
- LLVM backend support in the selfhost compiler
- LSP support in the selfhost compiler
- Identical binary output for *all possible* inputs (only the fixture set)

## Dual-Period Governance

During the dual period, **both** the Rust compiler (`crates/`) and the selfhost
sources (`src/compiler/`) are maintained in parallel. Every bug fix applied to
the Rust compiler must also be applied to the selfhost sources.

### 100% Self-Hosting Transition

The ultimate goal of the Arukellt project is **100% self-hosting**. All Rust code, including the compiler pipeline, IDE tooling (LSP, DAP), and foundational libraries (lexer, parser, typechecker), will be entirely rewritten in Arukellt.

There are no "permanent" Rust crates. The entire `crates/` directory is a deletion candidate.

### Deletion Candidates

All Rust crates currently in the workspace will be deleted once their Arukellt equivalents are complete and pass parity checks:

| Crate | Role | Deletion Condition |
|-------|------|--------------------|
| `ark-driver` | Pipeline orchestration | Selfhost `driver.ark` equivalent passes parity |
| MIR (removed in #561) | Mid-level IR and lowering | Selfhost `mir.ark` is now sole MIR/lowering authority |
| Wasm emitter (removed in #562) | Wasm binary emitter | Selfhost `emitter.ark` is now sole producer |
| `ark-stdlib` | Stdlib binary embedding | Selfhost equivalent passes parity |
| `arukellt` | CLI binary | Selfhost `main.ark` passes parity |
| `ark-lexer` | Tokenizer | Arukellt lexer supports IDE-grade error recovery |
| `ark-parser` | Parser | Arukellt parser supports IDE-grade error recovery |
| `ark-resolve` | Name resolution | Arukellt resolver supports IDE-grade incremental analysis |
| `ark-typecheck` | Type inference | Arukellt typechecker supports IDE-grade incremental analysis |
| `ark-hir` | High-level IR | Arukellt HIR supports IDE-grade analysis |
| `ark-diagnostics` | Shared diagnostics | Arukellt diagnostics system is complete |
| `ark-manifest` | Project manifest | Arukellt manifest parser is complete |
| `ark-target` | Compilation targets | Arukellt target definitions are complete |
| ~~`ark-lsp`~~ | ~~Language Server~~ | **Removed in #572 (Phase 7 of #529).** Selfhost `src/compiler/lsp.ark` invoked via `arukellt lsp` is the source of truth. |
| `ark-dap` | Debug Adapter | Arukellt DAP implementation is complete |

### When the Dual Period Ends

The dual period for the core compiler ends when:

> `python scripts/manager.py selfhost parity` exits 0 on `HEAD` of `master`.

Once `python scripts/manager.py selfhost parity` exits 0 in CI, the core compiler crates (driver, mir, wasm, CLI) can be deleted. The IDE and foundational crates will follow as their Arukellt counterparts achieve functional parity for incremental analysis and language server capabilities.

### IDE Tooling Positioning (100% Arukellt Architecture)

Unlike earlier plans that retained Rust for fast in-process IDE operations, the 100% self-hosting architecture demands that Arukellt itself provides the performance and error-recovery required for IDEs.

1. **Arukellt-Native Incrementality:** The Arukellt compiler frontend must be designed to support partial or incomplete source files and sub-millisecond response times.
2. **LSP/DAP in Arukellt:** The language server and debug adapter will be standard Arukellt programs compiled to Wasm and run via a Wasm runtime (e.g., Wasmtime or Node.js), communicating over standard I/O using JSON-RPC.
3. **No Rust dependency:** Once the Arukellt LSP/DAP and frontend are capable of fulfilling the editor contract, the remaining Rust crates will be purged, leaving a pure Arukellt codebase.
