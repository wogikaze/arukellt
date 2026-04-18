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

```
Stage 0 (Rust compiler)
  └─ compiles src/compiler/main.ark → arukellt-s1.wasm   (trusted base)

Stage 1 (arukellt-s1.wasm under wasmtime)
  └─ compiles src/compiler/main.ark → arukellt-s2.wasm   (first self-compile)

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

# Stage 0 only — Rust compiles selfhost sources (partial smoke only)
scripts/run/verify-bootstrap.sh --stage1-only

# Full fixpoint verification (Stages 0 → 1 → 2)
scripts/run/verify-bootstrap.sh

# Run a single stage
scripts/run/verify-bootstrap.sh --stage 0
```

**Last audited (stage numbering and CLI flags):** 2026-04-16 against
`scripts/run/verify-bootstrap.sh`.

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

Compare `sha256(arukellt-s1.wasm)` with `sha256(arukellt-s2.wasm)`.
Identical checksums prove the compiler is a fixpoint.

### Determinism requirement

Fixpoint verification only works when compilation is **deterministic**.
The compiler must not embed timestamps, random nonces, or pointer-derived
values into the output binary.  `scripts/run/verify-harness.sh` already
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
`sha256(arukellt-s1.wasm) == sha256(arukellt-s2.wasm)`.  That gate shows a
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
  `bash scripts/check/check-selfhost-parity.sh --fixture` when
  `scripts/check/check-selfhost-parity.sh` exists **and** is executable by the
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

When Stage 2 reports that `arukellt-s1.wasm ≠ arukellt-s2.wasm`:

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
| CI `selfhost-bootstrap` job | `scripts/run/verify-bootstrap.sh --check` | Runs Stages 0 → 1 → 2; the workflow asserts Stage 0 and Stage 1 report `reached`; Stage 2 fixpoint may still fail while `sha256(s1) ≠ sha256(s2)` (see [current-state.md](../current-state.md#self-hosting-bootstrap-status)) |
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
| `scripts/check/check-selfhost-fixpoint.sh` | sha256 fixpoint check (s1 vs s2); `--no-build` compares cached artifacts | #459 |
| `scripts/check/check-selfhost-fixture-parity.sh` | Run "run:" fixtures through s1.wasm; compare stdout to Rust output | #459 |
| `scripts/check/check-selfhost-diagnostic-parity.sh` | Run "diag:" fixtures through s1.wasm; check expected error pattern appears | #459 |
| `scripts/run/compare-outputs.sh` | Phase output diff (Rust vs selfhost) | #174 |
| `scripts/run/verify-harness.sh` | Top-level verification gate | — |

### How to Run the Fixpoint Check

```bash
# Build s1.wasm and s2.wasm then compare sha256
bash scripts/check/check-selfhost-fixpoint.sh

# Compare pre-built cached artifacts (faster, suitable for CI)
bash scripts/check/check-selfhost-fixpoint.sh --no-build

# Run fixture output parity (requires .build/selfhost/arukellt-s1.wasm)
bash scripts/check/check-selfhost-fixture-parity.sh

# Run diagnostic parity (requires .build/selfhost/arukellt-s1.wasm)
bash scripts/check/check-selfhost-diagnostic-parity.sh

# Run all selfhost checks via the full harness pass
bash scripts/run/verify-harness.sh --full
# Or individually:
bash scripts/run/verify-harness.sh --fixpoint
bash scripts/run/verify-harness.sh --selfhost-fixture-parity
bash scripts/run/verify-harness.sh --selfhost-diag-parity
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
> `scripts/check/check-selfhost-parity.sh` exits 0 on the same commit (fixture /
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
| **Stage 2 fixpoint** | `sha256(s1) == sha256(s2)` — compiler reproduces itself byte-for-byte | `scripts/run/verify-bootstrap.sh` Stage 2 |
| **Fixture parity** | Selfhost compiler passes all 575+ fixture tests identically to the Rust compiler | `scripts/check/check-selfhost-parity.sh` |
| **CLI parity** | `arukellt-s1.wasm compile <file>` stdout/stderr matches `arukellt compile <file>` for all fixture inputs | `scripts/check/check-selfhost-parity.sh --cli` |
| **Diagnostic parity** | Error message text, line/column positions, and exit codes match for all error fixtures | `scripts/check/check-selfhost-parity.sh --diag` |
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
| `ark-driver` | Pipeline orchestration (lex→parse→…→emit) | Selfhost `driver.ark` equivalent passes `scripts/check/check-selfhost-parity.sh` |
| `ark-mir` | Mid-level IR and HIR→MIR lowering | Selfhost `mir.ark` equivalent passes `scripts/check/check-selfhost-parity.sh` |
| `ark-wasm` | Wasm binary emitter | Selfhost `emitter.ark` equivalent passes `scripts/check/check-selfhost-parity.sh` |
| `arukellt` | Top-level CLI binary | Selfhost `main.ark` (as `arukellt.wasm`) passes `scripts/check/check-selfhost-parity.sh` |

**Parity check definition:** "passes `scripts/check/check-selfhost-parity.sh`" means
the script exits 0 for every fixture in `tests/fixtures/` on the current `HEAD`
of `master`.

### When the Dual Period Ends

**One observable condition:** The dual period ends when:

> `scripts/check/check-selfhost-parity.sh` exits 0 on `HEAD` of `master`.

All other gates (fixpoint check, CLI parity, diagnostic parity, determinism)
are prerequisites that must be satisfied before this command can exit 0. Once
`scripts/check/check-selfhost-parity.sh` exits 0 in CI, the dual period is over and
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
8. Update `scripts/run/verify-harness.sh` to invoke the selfhost binary
9. Update `docs/current-state.md` to remove the dual-period sections
10. Archive `issues/done/` for all selfhost promotion issues

## Workspace Restructuring Plan

> **Issue:** #332 — Design for selfhost-primary migration.
> **Status:** Planning only — no `Cargo.toml` or code changes.

This section documents the workspace restructuring that accompanies the
selfhost-primary transition. After the dual period ends and the selfhost
compiler is promoted, the Rust crates shift from "full compiler" to "IDE
tooling only." This plan defines the complete crate classification, feature
gate design, minimal post-selfhost workspace, migration procedure, and IDE
tooling architecture.

### Complete Crate Classification

The dual-period governance section above categorizes crates into "retained"
and "deletion candidates," but six crates are unclassified. This section
provides the complete three-tier classification for all 16 workspace crates.

**Tier 1 — Permanent (IDE tooling and shared foundations)**

These crates are retained permanently. They provide IDE integration, shared
analysis infrastructure, and foundational types used by both the Rust compiler
and future tooling.

| Crate | Purpose | Workspace Deps |
|-------|---------|----------------|
| `ark-diagnostics` | Shared diagnostic types | — |
| `ark-lexer` | Tokenizer | ark-diagnostics |
| `ark-parser` | Parser | ark-lexer, ark-diagnostics |
| `ark-hir` | High-level IR types | ark-diagnostics |
| `ark-resolve` | Name resolution and scopes | ark-parser, ark-diagnostics, ark-lexer |
| `ark-typecheck` | Type inference and unification | ark-parser, ark-resolve, ark-hir, ark-diagnostics |
| `ark-manifest` | Project manifest parsing | — |
| `ark-target` | Compilation target definitions | — |
| `ark-dap` | Debug Adapter Protocol | — |
| `ark-lsp` | Language Server Protocol | ark-lexer, ark-parser, ark-resolve, ark-typecheck, ark-diagnostics, ark-manifest, ark-driver\*, ark-stdlib\* |

\*Dependencies marked with `*` must be decoupled before deletion candidates
can be removed. See [ark-lsp Decoupling](#ark-lsp-decoupling-pre-deletion-prerequisite)
below.

**Rationale for Tier 1 additions** (crates not listed in dual-period governance):

- **ark-resolve, ark-typecheck, ark-hir**: Required by `ark-lsp` for
  go-to-definition, find-references, hover types, and inline diagnostics.
  These are fast, incremental, in-process operations that cannot be replaced
  by a subprocess call to the selfhost compiler. All three depend only on
  other Tier 1 crates.

- **ark-target**: Required by `ark-lsp` for stdlib path resolution and
  conditional compilation hints. No workspace dependencies.

**Tier 2 — Deletion Candidates (compiler pipeline)**

These crates implement the Rust compiler pipeline. Each is deleted after its
selfhost equivalent passes the parity check, as defined in the
[Rust Compiler Deletion Procedure](#rust-compiler-deletion-procedure) above.

| Crate | Role | Workspace Deps |
|-------|------|----------------|
| `ark-driver` | Pipeline orchestration | ark-lexer, ark-parser, ark-resolve, ark-typecheck, ark-hir, ark-mir, ark-wasm, ark-diagnostics, ark-target |
| `ark-mir` | MIR and HIR→MIR lowering | ark-parser, ark-typecheck, ark-diagnostics, ark-hir |
| `ark-wasm` | Wasm binary emitter | ark-mir, ark-typecheck, ark-diagnostics, ark-parser, ark-target |
| `ark-stdlib` | Stdlib binary embedding | ark-wasm |
| `ark-llvm` | LLVM backend (already optional) | ark-mir, ark-diagnostics, ark-typecheck, ark-target |
| `arukellt` | CLI binary | all crates |

**Note on `ark-stdlib`**: Although it provides stdlib definitions, its current
implementation depends on `ark-wasm` for binary embedding. After selfhost
promotion, stdlib embedding moves to the selfhost compiler. The LSP needs
stdlib *metadata* (names, signatures) but not binary embedding; this
decoupling is part of the migration (see Phase 2 below).

### Dependency Graph

```
LEAF (no workspace deps)
├── ark-diagnostics
├── ark-manifest
├── ark-target
└── ark-dap

FOUNDATION → diagnostics only
├── ark-lexer ──→ ark-diagnostics
└── ark-hir ────→ ark-diagnostics

PARSING → lexer + diagnostics
└── ark-parser ─→ ark-lexer, ark-diagnostics

SEMANTIC ANALYSIS → parser + diagnostics
├── ark-resolve ───→ ark-parser, ark-diagnostics, ark-lexer
└── ark-typecheck ─→ ark-parser, ark-resolve, ark-hir, ark-diagnostics

─── DELETION BOUNDARY ─────────────────────────────

COMPILER BACKEND (deletion candidates below this line)
├── ark-mir ───→ ark-parser, ark-typecheck, ark-diagnostics, ark-hir
├── ark-wasm ──→ ark-mir, ark-typecheck, ark-diagnostics, ark-parser, ark-target
├── ark-llvm ──→ ark-mir, ark-diagnostics, ark-typecheck, ark-target
└── ark-stdlib → ark-wasm

ORCHESTRATION
├── ark-driver → (9 crates: lexer, parser, resolve, typecheck, hir, mir, wasm, diagnostics, target)
└── arukellt ──→ (14 crates: all)

IDE (crosses the boundary — requires decoupling)
└── ark-lsp ───→ lexer, parser, resolve, typecheck, diagnostics, manifest, driver*, stdlib*
```

The deletion boundary cleanly separates Tier 1 (above) from Tier 2 (below),
except for `ark-lsp`'s dependencies on `ark-driver` and `ark-stdlib`, which
must be decoupled before deletion.

### Feature Gate Design

During the transition period between dual-period governance and full deletion,
a `rust-compiler` Cargo feature controls whether compiler-pipeline crates are
built. This allows the IDE-only workspace to be tested before pipeline crates
are permanently removed.

**Per-crate feature configuration:**

```toml
# crates/ark-lsp/Cargo.toml (planned change)
[features]
default = ["rust-compiler"]
rust-compiler = ["dep:ark-driver", "dep:ark-stdlib"]

[dependencies]
ark-driver = { workspace = true, optional = true }
ark-stdlib = { workspace = true, optional = true }
# ... other deps remain non-optional ...
```

```toml
# crates/arukellt/Cargo.toml (planned change)
[features]
default = ["rust-compiler"]
rust-compiler = ["ark-lsp/rust-compiler"]
```

**Crates gated by `rust-compiler`:**

| Crate | Gate mechanism |
|-------|---------------|
| `ark-driver` | Made optional dep in ark-lsp via feature |
| `ark-mir` | Only reached transitively through ark-driver |
| `ark-wasm` | Only reached transitively through ark-driver and ark-stdlib |
| `ark-stdlib` | Made optional dep in ark-lsp via feature |
| `ark-llvm` | Already gated by `llvm` feature in arukellt |
| `arukellt` | Removed from `default-members` when feature disabled |

**Build commands during transition:**

```bash
# Full build (default — dual period, all crates)
cargo build --workspace

# IDE-only build (test the minimal workspace)
cargo build -p ark-lsp --no-default-features
cargo build -p ark-dap

# Verify IDE-only build doesn't pull pipeline crates
cargo tree -p ark-lsp --no-default-features | grep -E 'ark-(driver|mir|wasm|stdlib)'
# Expected: no output (no pipeline crates in dependency tree)
```

### Minimal Post-Selfhost Cargo.toml

After all deletion candidates are removed, the workspace contains only the
10 permanent Tier 1 crates. The selfhost compiler (`arukellt.wasm`) is the
primary compiler; Rust crates provide IDE tooling only.

```toml
[workspace]
resolver = "2"
members = [
    "crates/ark-lexer",
    "crates/ark-parser",
    "crates/ark-resolve",
    "crates/ark-typecheck",
    "crates/ark-hir",
    "crates/ark-diagnostics",
    "crates/ark-target",
    "crates/ark-manifest",
    "crates/ark-lsp",
    "crates/ark-dap",
]

[workspace.package]
version = "0.2.0"
edition = "2024"
license = "MIT"
rust-version = "1.85"

[workspace.dependencies]
ark-lexer = { path = "crates/ark-lexer" }
ark-parser = { path = "crates/ark-parser" }
ark-resolve = { path = "crates/ark-resolve" }
ark-typecheck = { path = "crates/ark-typecheck" }
ark-hir = { path = "crates/ark-hir" }
ark-diagnostics = { path = "crates/ark-diagnostics" }
ark-target = { path = "crates/ark-target" }
ark-manifest = { path = "crates/ark-manifest" }
ark-dap = { path = "crates/ark-dap" }

# Shared dependencies (IDE tooling only)
ariadne = "0.5"
clap = { version = "4", features = ["derive"] }
thiserror = "2"
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
tower-lsp = "0.20"
tokio = { version = "1", features = ["full"] }
serde_json = "1.0"
```

**Removed crates:** `ark-driver`, `ark-mir`, `ark-wasm`, `ark-stdlib`,
`ark-llvm`, `arukellt`.

**Removed workspace dependencies:** `wasm-encoder`, `wasmparser`, `wasmtime`,
`inkwell` — these belong to the compiler pipeline and are no longer needed.

**Version bump:** `0.1.0` → `0.2.0` to mark the post-selfhost transition.

### ark-lsp Decoupling (Pre-deletion Prerequisite)

Before compiler-pipeline crates can be deleted, `ark-lsp` must be decoupled
from `ark-driver` and `ark-stdlib`. This is a prerequisite for Phase 3 of
the migration.

**Current problematic dependency chains:**

```
ark-lsp → ark-driver → ark-mir, ark-wasm, ...  (9 transitive deps)
ark-lsp → ark-stdlib → ark-wasm                (2 transitive deps)
```

**Decoupling strategy:**

1. **`ark-driver` → subprocess invocation.** The LSP currently uses ark-driver
   to run the full compilation pipeline and collect diagnostics. After selfhost
   promotion, replace this with a subprocess call:

   ```
   wasmtime arukellt.wasm compile --check --diagnostics-format=json <file>
   ```

   The selfhost compiler emits structured JSON diagnostics on stdout; the LSP
   parses this output. This eliminates the in-process dependency on the entire
   Rust compilation pipeline.

   **Trade-off:** subprocess invocation adds ~50-100ms latency per full-file
   diagnostic check. This is acceptable because:
   - Fast operations (tokenization, parsing, name resolution, type hover)
     remain in-process via Tier 1 crates
   - Full-file diagnostics are debounced (typically 300ms+ after last keystroke)
   - The subprocess model matches how other LSP servers work (e.g., rust-analyzer
     invokes `cargo check`)

2. **`ark-stdlib` → metadata extraction.** The LSP uses ark-stdlib for
   function signatures, type information, and documentation strings for
   completions and hover. Replace the direct dependency with a static metadata
   file:

   - Generate `std/stdlib-metadata.json` from stdlib manifests (signatures,
     types, doc strings) as a build step
   - The LSP reads this file at startup and caches it in memory
   - No dependency on `ark-wasm` or binary embedding

   **Format sketch:**

   ```json
   {
     "modules": {
       "io": {
         "functions": {
           "println": {
             "signature": "fn println(s: String)",
             "doc": "Print a string followed by a newline."
           }
         }
       }
     }
   }
   ```

### Step-by-Step Migration Procedure

Execute the following phases **in order**. Each step within a phase must leave
the repository in a buildable, test-passing state (`verify-harness.sh --quick`
passes all checks).

**Phase 1 — Feature gate introduction** (while dual period is active)

| Step | Action | Verification |
|------|--------|--------------|
| 1.1 | Add `rust-compiler` feature to `ark-lsp/Cargo.toml`; make `ark-driver` and `ark-stdlib` optional behind this feature | `cargo build --workspace` passes |
| 1.2 | Guard `ark-lsp` code that uses `ark-driver` / `ark-stdlib` with `#[cfg(feature = "rust-compiler")]` | `cargo build -p ark-lsp --no-default-features` passes |
| 1.3 | Add `rust-compiler` feature to `arukellt/Cargo.toml` that enables it transitively in ark-lsp | `cargo build --workspace` passes (default features) |
| 1.4 | Verify IDE-only build produces a functional LSP binary (syntax highlighting, basic diagnostics from in-process analysis) | Manual LSP test with `--no-default-features` |

**Phase 2 — ark-lsp decoupling** (after selfhost promotion confirmed)

| Step | Action | Verification |
|------|--------|--------------|
| 2.1 | Generate `std/stdlib-metadata.json` from stdlib manifests | File exists and is valid JSON |
| 2.2 | Replace `ark-lsp`'s direct `ark-stdlib` usage with metadata file reading | `cargo build -p ark-lsp --no-default-features` passes |
| 2.3 | Add `--diagnostics-format=json` flag to selfhost compiler | `wasmtime arukellt.wasm compile --check --diagnostics-format=json <file>` emits JSON |
| 2.4 | Replace `ark-lsp`'s direct `ark-driver` usage with selfhost subprocess invocation | `cargo build -p ark-lsp --no-default-features` passes |
| 2.5 | Remove `rust-compiler` feature from `ark-lsp` (zero pipeline deps remain) | `cargo build -p ark-lsp` passes; `cargo tree -p ark-lsp \| grep ark-driver` outputs nothing |

**Phase 3 — Compiler pipeline deletion** (follows [Rust Compiler Deletion Procedure](#rust-compiler-deletion-procedure) above)

| Step | Action | Verification |
|------|--------|--------------|
| 3.1 | Delete `crates/ark-driver/` and remove from `Cargo.toml` | `cargo build --workspace` passes |
| 3.2 | Delete `crates/ark-mir/` and remove from `Cargo.toml` | `cargo build --workspace` passes |
| 3.3 | Delete `crates/ark-wasm/` and remove from `Cargo.toml` | `cargo build --workspace` passes |
| 3.4 | Delete `crates/ark-stdlib/` and remove from `Cargo.toml` | `cargo build --workspace` passes |
| 3.5 | Delete `crates/ark-llvm/` and remove from `Cargo.toml` | `cargo build --workspace` passes |
| 3.6 | Delete `crates/arukellt/` and remove from `Cargo.toml` | `cargo build --workspace` passes |
| 3.7 | Remove `wasm-encoder`, `wasmparser`, `wasmtime`, `inkwell` from workspace deps | `cargo build --workspace` passes |
| 3.8 | Apply minimal post-selfhost `Cargo.toml` (see above); bump version to `0.2.0` | `cargo build --workspace` passes |
| 3.9 | Update CI: replace `cargo build --workspace` compile step with `wasmtime run arukellt.wasm` for compiler tests | CI green |
| 3.10 | Update `scripts/run/verify-harness.sh` to invoke the selfhost binary | `verify-harness.sh --quick` passes |

### IDE Tooling Positioning (Post-Selfhost Architecture)

After selfhost promotion, the Rust crates serve a single purpose: **IDE
tooling.** The compilation workflow shifts entirely to the selfhost compiler.

```
┌──────────────────────────────────────────────────────────┐
│  Editor (VS Code / Neovim / etc.)                        │
│  └─ LSP client                                           │
└───────────────┬──────────────────────────────────────────┘
                │ LSP protocol (JSON-RPC over stdio)
┌───────────────▼──────────────────────────────────────────┐
│  ark-lsp (Rust binary)                                   │
│                                                          │
│  In-process (fast, incremental):                         │
│  ├─ ark-lexer      → syntax highlighting, token stream   │
│  ├─ ark-parser     → AST, structural navigation          │
│  ├─ ark-resolve    → go-to-definition, find-references   │
│  ├─ ark-typecheck  → hover types, type diagnostics       │
│  ├─ ark-hir        → high-level IR for analysis          │
│  ├─ ark-manifest   → project config, stdlib paths        │
│  ├─ ark-target     → target info, conditional hints      │
│  └─ ark-diagnostics→ diagnostic formatting               │
│                                                          │
│  Subprocess (full-file diagnostics):                     │
│  └─ wasmtime arukellt.wasm compile --check --diag=json   │
└──────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────┐
│  ark-dap (Rust binary)                                   │
│  └─ Debug Adapter Protocol for Wasm debugging            │
└──────────────────────────────────────────────────────────┘
```

**Key design decisions:**

1. **Rust crates handle fast, incremental IDE operations.** Tokenization,
   parsing, name resolution, and type hover run in-process for sub-millisecond
   response times. These operations benefit from Rust's speed and can operate
   on partial or incomplete source files.

2. **Selfhost compiler handles full compilation diagnostics** via subprocess
   invocation. The LSP invokes
   `wasmtime arukellt.wasm compile --check --diagnostics-format=json <file>`
   and parses structured JSON output. This model matches how `rust-analyzer`
   delegates to `cargo check`.

3. **Semantic analysis crates are retained** (`ark-resolve`, `ark-typecheck`,
   `ark-hir`) because IDE features like go-to-definition, find-references,
   and hover-type require fast in-process analysis that subprocess latency
   cannot support.

4. **No duplication of compiler logic.** The Tier 1 semantic analysis crates
   analyze source code for IDE purposes. They do *not* need to match the
   selfhost compiler's output bit-for-bit — they only need to provide
   correct-enough analysis for IDE features. The selfhost compiler is the
   single source of truth for compilation correctness.
