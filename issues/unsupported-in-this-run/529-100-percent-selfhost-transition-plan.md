# 100% Self-Hosting Transition Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transition the Arukellt compiler and IDE tooling from a dual-period Rust/Arukellt architecture to a 100% Arukellt-hosted architecture, culminating in the complete deletion of the Rust `crates/` directory.

**Architecture:** This plan is executed in two major phases. Phase 1 focuses on achieving a compiler fixpoint by implementing multi-file module loading in the selfhost compiler, allowing it to accurately reproduce itself and pass all fixture parity checks. Phase 2 focuses on upgrading the Arukellt frontend (lexer, parser, resolver, typechecker) to support fast, incremental, error-resilient analysis, enabling the creation of an Arukellt-native LSP and DAP. Once the Arukellt tooling meets the IDE contract, the Rust crates are deleted.

**Tech Stack:** Arukellt, Wasmtime (for running the selfhost compiler), Bash (for verification scripts).

---

### Task 1: Unblock Compiler Fixpoint (Module Loading)

**Files:**
- Modify: `src/compiler/driver.ark` (or relevant module loader)
- Modify: `src/compiler/mir.ark` (if cross-module call lowering needs updates)
- Modify: `src/compiler/emitter.ark` (to ensure cross-module calls are emitted correctly instead of `i32.const 0`)

- [ ] **Step 1: Analyze current module loading failures**
  Run `scripts/check/check-selfhost-fixpoint.sh` and inspect the output of `arukellt-s1.wasm` to understand why `use` statements are currently ignored.

- [ ] **Step 2: Implement multi-file resolution in selfhost driver**
  Update the selfhost compiler's driver/resolver to correctly load, parse, and link multiple `.ark` files passed via the command line or discovered via `use` statements.

- [ ] **Step 3: Fix cross-module call lowering**
  Ensure that calls to functions in other modules are correctly lowered to MIR and emitted as valid Wasm `call` instructions, rather than stubbed out.

- [ ] **Step 4: Verify Stage 1 and Stage 2 Fixpoint**
  Run `scripts/run/verify-bootstrap.sh`.
  Expected: Both `stage1-self-compile` and `stage2-fixpoint` report `reached` (i.e., `sha256(s1) == sha256(s2)`).

- [ ] **Step 5: Commit**
  ```bash
  git add src/compiler/
  git commit -m "feat(compiler): implement multi-file module loading for selfhost fixpoint"
  ```

### Task 2: Achieve Core Compiler Parity

**Files:**
- Modify: `src/compiler/*.ark` (as needed to fix discrepancies)
- Test: `tests/fixtures/`

- [ ] **Step 1: Run fixture parity checks**
  Run `scripts/check/check-selfhost-parity.sh --fixture`. Identify failing tests.

- [ ] **Step 2: Fix compilation discrepancies**
  Iteratively fix the selfhost compiler until all fixtures compile and run with the exact same output as the Rust compiler.

- [ ] **Step 3: Run diagnostic parity checks**
  Run `scripts/check/check-selfhost-parity.sh --diag`. Identify failing error messages.

- [ ] **Step 4: Fix diagnostic discrepancies**
  Ensure error codes, messages, and line/column numbers match the Rust compiler's output exactly.

- [ ] **Step 5: Verify full parity**
  Run `scripts/check/check-selfhost-parity.sh`.
  Expected: Exit 0 (all parity checks pass).

- [ ] **Step 6: Commit**
  ```bash
  git add src/compiler/
  git commit -m "fix(compiler): achieve full fixture and diagnostic parity with Rust backend"
  ```

### Task 3: Delete Core Rust Compiler Crates

**Files:**
- Delete: `crates/ark-driver/`
- Delete: `crates/ark-mir/`
- Delete: `crates/ark-wasm/`
- Delete: `crates/ark-stdlib/` (if embedded logic is fully migrated)
- Delete: `crates/arukellt/`
- Modify: `Cargo.toml`
- Modify: `.github/workflows/` (or equivalent CI config)
- Modify: `scripts/run/verify-harness.sh`

- [ ] **Step 1: Delete core compiler crates**
  Remove the `ark-driver`, `ark-mir`, `ark-wasm`, `ark-stdlib`, and `arukellt` directories.

- [ ] **Step 2: Update Cargo workspace**
  Remove the deleted crates from the `members` array in the root `Cargo.toml`. Remove dependencies like `wasm-encoder` and `wasmtime` if they are no longer needed by the remaining IDE crates.

- [ ] **Step 3: Update CI and test scripts**
  Modify CI workflows and `verify-harness.sh` to use `wasmtime run .bootstrap-build/arukellt-s1.wasm` (or the promoted selfhost binary) instead of `cargo run -p arukellt`.

- [ ] **Step 4: Verify build and tests**
  Run `cargo build --workspace` to ensure the remaining crates compile. Run `scripts/run/verify-harness.sh` to ensure the Arukellt compiler runs the test suite successfully.

- [ ] **Step 5: Commit**
  ```bash
  git rm -r crates/ark-driver crates/ark-mir crates/ark-wasm crates/ark-stdlib crates/arukellt
  git add Cargo.toml scripts/run/verify-harness.sh
  git commit -m "chore: remove Rust compiler backend after selfhost promotion"
  ```

### Task 4: Implement Arukellt-Native LSP and DAP

**Files:**
- Create: `src/ide/lsp.ark`
- Create: `src/ide/dap.ark`
- Modify: `src/compiler/lexer.ark`, `src/compiler/parser.ark` (for error recovery)

- [ ] **Step 1: Enhance parser for incremental/error-resilient analysis**
  Modify the Arukellt parser to recover from syntax errors gracefully and produce a partial AST, which is required for IDE features.

- [ ] **Step 2: Implement Language Server Protocol in Arukellt**
  Create `src/ide/lsp.ark` that reads JSON-RPC from standard input, uses the Arukellt compiler frontend to analyze code, and responds with hover, definition, and diagnostic information.

- [ ] **Step 3: Implement Debug Adapter Protocol in Arukellt**
  Create `src/ide/dap.ark` to handle debugging requests.

- [ ] **Step 4: Test LSP/DAP functionality**
  Verify that the newly compiled Arukellt LSP can connect to an editor (e.g., VS Code extension) and provide basic features.

- [ ] **Step 5: Commit**
  ```bash
  git add src/ide/ src/compiler/
  git commit -m "feat(ide): implement native Arukellt LSP and DAP"
  ```

### Task 5: Delete Rust IDE Crates

**Files:**
- Delete: `crates/ark-lsp/`, `crates/ark-dap/`, `crates/ark-lexer/`, `crates/ark-parser/`, `crates/ark-resolve/`, `crates/ark-typecheck/`, `crates/ark-hir/`, `crates/ark-diagnostics/`, `crates/ark-manifest/`, `crates/ark-target/`
- Delete: `Cargo.toml`, `Cargo.lock`

- [ ] **Step 1: Remove all remaining Rust crates**
  Delete the entire `crates/` directory.

- [ ] **Step 2: Remove Cargo configuration**
  Delete `Cargo.toml` and `Cargo.lock` from the repository root, as Rust is no longer used.

- [ ] **Step 3: Update tooling and documentation**
  Update any remaining scripts or documentation that reference `cargo` or the Rust toolchain.

- [ ] **Step 4: Verify clean state**
  Ensure the repository contains no Rust code and all Arukellt tests still pass using the selfhost compiler.

- [ ] **Step 5: Commit**
  ```bash
  git rm -r crates/ Cargo.toml Cargo.lock
  git commit -m "chore: remove all remaining Rust IDE crates, completing 100% self-hosting transition"
  ```
