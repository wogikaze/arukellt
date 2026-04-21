# 100% Self-Hosting Transition Plan (Operational Guide)

> **Status:** Implementation Guide — ready for execution with verification checkpoints
> **For agentic workers:** Execute phase-by-phase. Each phase has mandatory verification steps.

**Goal:** Transition the Arukellt compiler and IDE tooling from a dual-period Rust/Arukellt architecture to a 100% Arukellt-hosted architecture, culminating in the complete deletion of the Rust `crates/` directory.

**Work Streams (DO NOT MIX):**
1. Selfhost compiler: `src/compiler/*.ark`
2. Trusted base Rust compiler: `crates/arukellt`, `crates/ark-driver`, `crates/ark-mir`, `crates/ark-wasm`
3. Verification: `scripts/check/*`, `scripts/run/*`, `tests/fixtures/*`
4. IDE: `crates/ark-lsp`, `crates/ark-dap`, future `src/ide/*`

**Key Constraint:** First goal is NOT "delete Rust" but "selfhost compiler can reproduce itself".

---

## Execution Phases

### Phase 0: Baseline Establishment

**Purpose:** Fix current state. Observe only. Do not implement.

**Execution:**
```bash
cargo build -p arukellt
bash scripts/run/verify-harness.sh --quick
bash scripts/check/check-selfhost-fixpoint.sh
bash scripts/check/check-selfhost-fixture-parity.sh
bash scripts/check/check-selfhost-diagnostic-parity.sh
```

**Record:**
- `target/debug/arukellt` exists?
- `.build/selfhost/arukellt-s1.wasm` vs `s2.wasm` size difference
- Fixpoint failure root cause
- Fixture parity: pass/fail/skip counts
- Diagnostic parity: pass/fail/skip counts

**Known Root Cause (from `check-selfhost-fixpoint.sh`):**
1. Selfhost compiler does NOT follow `use` statements to load multiple files
2. Cross-module calls are stubbed (not lowered/emitted correctly)
3. Result: s2.wasm is extremely small

**First work targets:** `src/compiler/driver.ark`, `src/compiler/mir.ark`, `src/compiler/emitter.ark`

---

### Phase 1: Fixpoint Achievement (CRITICAL)

**Goal:** `sha256(s1) == sha256(s2)` — stable reproducible self-compilation

**Prerequisite:** All determinism issues resolved BEFORE feature work.

#### 1-1. Define Module Loading Specification (Minimal)

**Scope for fixpoint only:**
- `use foo` resolves to `foo.ark` in same directory as entry file
- Recursive resolution
- Duplicate module load = error
- Deterministic load order (sorted)
- Topological or stable source aggregation

**NOT in scope yet:** Package system, complex search paths, import cycle recovery

#### 1-2. Implement Multi-File Source Loading in Driver

**Target:** `src/compiler/driver.ark`

**Implementation:**
- Read entry file
- Extract `use` declarations
- Recursively read import targets
- Maintain visited set
- Stabilize module order
- Pass concatenated source or module sequence to lex/parse

**Recommendation:** Start simple. Build module graph → determine file order → compile in batch.

#### 1-3. Align Qualified Call Handling in MIR/Emitter

**Current failure:** Cross-module calls are unresolved/stubbed.

**Investigation:**
- Compare MIR dump from Rust compiler vs selfhost for same call site
- Check how `lexer::tokenize` style qualified names are handled
- Verify path: `qualified name → resolved function symbol → deterministic function index`

**Caution:** "Strip prefix only" may work temporarily but risks symbol collision. Acceptable for fixpoint if `src/compiler/*.ark` already assumes it. Full module namespace comes after parity.

#### 1-4. Enforce Determinism

**Requirements for fixpoint:**
- NO HashMap iteration in output-affecting paths
- NO filesystem enumeration order dependence
- NO import resolution order variation by source layout
- NO non-deterministic function index allocation

**Verification (mandatory):**
```bash
bash scripts/check/check-selfhost-fixpoint.sh
bash scripts/run/verify-bootstrap.sh --check
```

**Phase 1 Exit Condition:** `stage1-self-compile: reached` AND `stage2-fixpoint: reached`

---

### Phase 2: Fixture Parity Expansion

**Goal:** Selfhost compiler handles general fixtures same as trusted base.

**Execution:**
```bash
bash scripts/check/check-selfhost-fixture-parity.sh
bash scripts/check/check-selfhost-parity.sh --fixture
```

**Approach:**
1. Separate "fail" from "skip"
2. First: fixtures skipped due to compile errors
3. Then: fixtures failing due to output differences

**Priority:**
1. Parse failures
2. Resolve/typecheck failures
3. MIR/emitter output differences
4. Standard library/intrinsic differences
5. Host I/O differences
6. Target-specific differences

**Strategy:** Tackle by category, not all at once.

---

### Phase 3: Diagnostic Parity Expansion

**Goal:** Error detection and reporting matches Rust compiler.

**Execution:**
```bash
bash scripts/check/check-selfhost-diagnostic-parity.sh
bash scripts/check/check-selfhost-parity.sh --diag
```

**Internal Milestones (do not skip):**
- M1: Same error categories appear in both compilers
- M2: Error codes match
- M3: Primary messages match
- M4: Spans (line/column) match

**Policy:** Defer M4 until after Phase 4. Message/span exact matching is high-cost, low-value initially.

---

### Phase 4: Dual-Run Period (SAFETY)

**Purpose:** Maintain Rust as fallback while selfhost stabilizes.

**Duration:** Minimum several days to iterations.

**Execution:**
- CI runs both Rust and selfhost compilers
- Compare results for all major fixtures
- Investigate any mismatch
- Rust becomes read-only fallback

**Exit Conditions:**
- Fixpoint stable
- Fixture parity stable
- No critical diagnostic differences
- No selfhost regressions in recent changes

---

### Phase 5: Core Compiler Crates Deletion

**When:** After 2+ weeks of clean dual-run results.

**Targets:**
- `crates/ark-driver`
- `crates/ark-mir`
- `crates/ark-wasm`
- `crates/ark-stdlib` (if fully migrated)
- `crates/arukellt`

#### 5-1. Fix Selfhost Compiler Launch Path

**Options:**
- Development: `wasmtime run .build/selfhost/arukellt-s1.wasm -- ...`
- Or: promoted selfhost binary wrapper

#### 5-2. Update Scripts to Selfhost-First

**Targets:**
- `python scripts/manager.py verify`
- `.github/workflows/*.yml`

**Changes:**
- Remove `cargo run -p arukellt` assumptions
- Use selfhost wasm execution
- Keep Cargo workspace if IDE crates still need it

#### 5-3. Final Verification Before Deletion

```bash
bash scripts/run/verify-harness.sh --quick
bash scripts/run/verify-harness.sh --fixtures
bash scripts/run/verify-harness.sh --fixpoint
bash scripts/run/verify-harness.sh --selfhost-fixture-parity
bash scripts/run/verify-harness.sh --selfhost-diag-parity
```

**Then delete.**

---

### Phase 6: IDE Frontend/LSP/DAP Migration

**Scope:** Separate project-level effort. Not a compiler subtask.

**Requirements (different from batch compiler):**
- Error recovery (continue past syntax errors)
- Partial AST preservation
- Incremental update support
- Stable symbol tables for hover/definition
- Cancellation / latency management

#### 6-A. IDE-Ready Frontend

**Targets:** `src/compiler/lexer.ark`, `src/compiler/parser.ark`, possibly `resolver.ark`, `typechecker.ark`

**Goals:**
- Don't discard entire tree on syntax error
- Return partial AST
- Accumulate diagnostics incrementally

#### 6-B. Separate Analysis API from CLI

**New entry point:** "document text → AST / symbols / diagnostics" (not CLI subprocess)

#### 6-C. LSP Minimum Viable

**Create:** `src/ide/lsp.ark`

**Handlers (in order):**
1. `initialize`
2. `textDocument/didOpen`
3. `textDocument/didChange`
4. `textDocument/hover`
5. `textDocument/definition`
6. `textDocument/publishDiagnostics`

#### 6-D. DAP (Deferred)

DAP involves runtime/debug info/stepping. Separate issue, lower priority than LSP.

---

### Phase 7: Full Rust Deletion

**Final targets:**
- `crates/ark-lsp`
- `crates/ark-dap`
- `crates/ark-lexer`
- `crates/ark-parser`
- `crates/ark-resolve`
- `crates/ark-typecheck`
- `crates/ark-hir`
- `crates/ark-diagnostics`
- `crates/ark-manifest`
- `crates/ark-target`

**Last to delete:** `Cargo.toml`, `Cargo.lock` — only when ALL Rust dependencies are gone (including VS Code extension and other tools).

---

## Daily Operational Procedure

**Per work unit (single concern only):**

1. **Select target**
   - Example: driver module loading only, OR emitter qualified call only, OR parser error recovery only

2. **Observe before change**
   ```bash
   bash scripts/check/check-selfhost-fixpoint.sh --no-build
   bash scripts/check/check-selfhost-fixture-parity.sh
   ```

3. **Implement**

4. **Minimal verification**
   ```bash
   python scripts/manager.py verify quick
   python scripts/manager.py verify cargo
   python scripts/manager.py verify fixtures
   ```

5. **Selfhost verification**
   ```bash
   bash scripts/check/check-selfhost-fixpoint.sh
   bash scripts/check/check-selfhost-fixture-parity.sh
   bash scripts/check/check-selfhost-diagnostic-parity.sh
   ```

6. **Record deltas**
   - s1/s2 sizes
   - Parity pass/fail/skip counts
   - Newly passing fixtures
   - Broken fixtures

7. **Stop there** — don't expand to adjacent issues

---

## Branch Naming Convention

One branch per concern:

- `feat/selfhost-module-loading`
- `fix/selfhost-qualified-call-resolution`
- `fix/selfhost-function-index-determinism`
- `fix/selfhost-fixture-parity-literals`
- `fix/selfhost-diagnostic-span-alignment`
- `chore/remove-rust-core-compiler`
- `feat/selfhost-lsp-minimal`

---

## Completion Criteria

### Criterion A: Selfhost Bootstrap
- [ ] `check-selfhost-fixpoint.sh` passes
- [ ] `verify-bootstrap.sh --check` reports `stage2-fixpoint: reached`

### Criterion B: Compiler Parity
- [ ] Fixture parity: 0 fails
- [ ] Diagnostic parity: 0 critical fails
- [ ] Skip count: temporarily acceptable, target 0 eventually

### Criterion C: Rust Core Compiler Retirement
- [ ] Execution path switched to selfhost
- [ ] Harness passes after core compiler crate deletion

### Criterion D: Full Rust Retirement
- [ ] IDE fully selfhost/native
- [ ] No Cargo workspace needed
- [ ] No Rust code in repository

---

## Immediate Next Steps (First 3 Tasks)

**STOP before LSP, Rust deletion, or other phases until these complete:**

1. **Implement recursive module loading** in `src/compiler/driver.ark`
2. **Align qualified cross-module call resolution** in `src/compiler/mir.ark` and `src/compiler/emitter.ark`
3. **Verify** `bash scripts/check/check-selfhost-fixpoint.sh` passes

**Rule:** Do not proceed to Phase 2+ until Phase 1 fixpoint is stable. Root cause isolation becomes impossible if you mix phases.
