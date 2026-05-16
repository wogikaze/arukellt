---
Track: main
Orchestration class: implementation-ready
Depends on: none
Sub-issues: 624, 594, 625, 564, 626, 627, 628, 571, 574, 575, 576, 577, 578, 579, 580, 581, 582
---

# 100% selfhost transition plan

## Sub-issue Index

| Phase | Sub-issue | File | Status |
|-------|-----------|------|--------|
| Phase 1: Fixpoint Achievement | #624 | `529-phase1-fixpoint-achievement.md` | Done (fixpoint reached) |
| Phase 2+3: Fixture & Diagnostic Parity | #594 | `594-selfhost-phase2-fixture-diag-parity.md` | Open |
| Phase 4: Dual-Run Period (SAFETY) | #625 | `529-phase4-dual-run-period.md` | Open |
| Phase 5: Delete `crates/arukellt` | #564 | `564-phase5-delete-arukellt.md` | Open |
| Phase 6/A: IDE-Ready Frontend | #626 | `529-phase6a-ide-ready-frontend.md` | Open |
| Phase 6/B: Analysis API | #627 | `529-phase6b-analysis-api.md` | Open |
| Phase 6/C: LSP Minimum Viable | #628 | `529-phase6c-lsp-minimum-viable.md` | Open |
| Phase 6/D: DAP Scaffold | #571 | `571-phase6-debug-adapter-scaffold-deferred-priority.md` | Open |
| Phase 7: Delete `crates/ark-lexer` | #574 | `574-phase7-delete-ark-lexer.md` | Open |
| Phase 7: Delete `crates/ark-parser` | #575 | `575-phase7-delete-ark-parser.md` | Open |
| Phase 7: Delete `crates/ark-resolve` | #576 | `576-phase7-delete-ark-resolve.md` | Open |
| Phase 7: Delete `crates/ark-typecheck` | #577 | `577-phase7-delete-ark-typecheck.md` | Open |
| Phase 7: Delete `crates/ark-hir` | #578 | `578-phase7-delete-ark-hir.md` | Open |
| Phase 7: Delete `crates/ark-diagnostics` | #579 | `579-phase7-delete-ark-diagnostics.md` | Open |
| Phase 7: Delete `crates/ark-manifest` | #580 | `580-phase7-delete-ark-manifest.md` | Open |
| Phase 7: Delete `crates/ark-target` | #581 | `581-phase7-delete-ark-target.md` | Open |
| Phase 7: Delete workspace (Cargo.toml/Cargo.lock) | #582 | `582-phase7-remove-cargo-workspace.md` | Open |

### Phase 1: "Fixpoint Achievement (CRITICAL)" — see `529-phase1-fixpoint-achievement.md` (#624)

Goal: Error detection and reporting matches Rust compiler.
Prerequisite: All determinism issues resolved BEFORE feature work.
Scope for fixpoint only:
NOT in scope yet: Package system, complex search paths, import cycle recovery
Target: `src/compiler/driver.ark`
Implementation:
Recommendation: Start simple. Build module graph → determine file order → compile in batch.
Current failure: Cross-module calls are unresolved/stubbed.
Investigation:
- Check how `lexer: ":tokenize` style qualified names are handled"
- Verify path: `qualified name → resolved function symbol → deterministic function index`
Caution: "Strip prefix only" may work temporarily but risks symbol collision. Acceptable for fixpoint if `src/compiler/*.ark` already assumes it. Full module namespace comes after parity.
Requirements for fixpoint:
Verification (mandatory):
Phase 1 Exit Condition: "`stage1-self-compile: reached` AND `stage2-fixpoint: reached`"

### Phase 2: Fixture Parity Expansion — see `594-selfhost-phase2-fixture-diag-parity.md` (#594, also covers Phase 3)

Approach:
2. First: fixtures skipped due to compile errors
3. Then: fixtures failing due to output differences
Priority:
Strategy: Tackle by category, not all at once.

### Phase 3: Diagnostic Parity Expansion — see `594-selfhost-phase2-fixture-diag-parity.md` (#594, combined with Phase 2)

Internal Milestones (do not skip):
- M1: Same error categories appear in both compilers
- M2: Error codes match
- M3: Primary messages match
- M4: "Spans (line/column) match"
Policy: Defer M4 until after Phase 4. Message/span exact matching is high-cost, low-value initially.

### Phase 4: "Dual-Run Period (SAFETY)" — see `529-phase4-dual-run-period.md` (#625)

Duration: Minimum several days to iterations.
Exit Conditions:

### Phase 5: Core Compiler Crates Deletion — see `564-phase5-delete-arukellt.md` (#564)

When: After 2+ weeks of clean dual-run results.
Targets: `src/compiler/lexer.ark`, `src/compiler/parser.ark`, possibly `resolver.ark`, `typechecker.ark`
Options:
- Development: `wasmtime run .build/selfhost/arukellt-s1.wasm -- ...`
- Or: promoted selfhost binary wrapper
Changes:

### Phase 6: IDE Frontend/LSP/DAP Migration — see sub-issues below

Scope: Separate project-level effort. Not a compiler subtask.

**Sub-issues:**
- Phase 6/A (IDE-Ready Frontend): `529-phase6a-ide-ready-frontend.md` (#626)
- Phase 6/B (Analysis API): `529-phase6b-analysis-api.md` (#627)
- Phase 6/C (LSP Minimum Viable): `529-phase6c-lsp-minimum-viable.md` (#628)
- Phase 6/D (DAP Scaffold): `571-phase6-debug-adapter-scaffold-deferred-priority.md` (#571)

Requirements (different from batch compiler):
Goals:
New entry point: ""document text → AST / symbols / diagnostics" (not CLI subprocess)"
Create: `src/ide/lsp.ark`
Handlers (in order):

### Phase 7: Full Rust Deletion — see individual crate deletion sub-issues

**Crate deletion sub-issues:** #574 (ark-lexer), #575 (ark-parser), #576 (ark-resolve), #577 (ark-typecheck), #578 (ark-hir), #579 (ark-diagnostics), #580 (ark-manifest), #581 (ark-target), #582 (Cargo.toml/Cargo.lock)
Final targets:
Last to delete: "`Cargo.toml`, `Cargo.lock` — only when ALL Rust dependencies are gone (including VS Code extension and other tools)."
Per work unit (single concern only):
- Example: driver module loading only, OR emitter qualified call only, OR parser error recovery only

### Criterion A: Selfhost Bootstrap

- [ ] `verify-bootstrap.sh --check` reports `stage2-fixpoint: reached`

### Criterion B: Compiler Parity

- [ ] Fixture parity: 0 fails
- [ ] Diagnostic parity: 0 critical fails
- [ ] Skip count: temporarily acceptable, target 0 eventually

### Criterion C: Rust Core Compiler Retirement

### Criterion D: Full Rust Retirement

### STOP before LSP Rust deletion or other phases until these complete

Rule: Do not proceed to Phase 2+ until Phase 1 fixpoint is stable. Root cause isolation becomes impossible if you mix phases

---

## 100% Self-Hosting Transition Plan (Operational Guide)

## Responsibility split — 2026-04-22

\#529 owns the **legacy removal / selfhost transition** lane, including #285 and
\#508 after ADR-028. Keep this lane separate from:

- #125/#126: trusted-base compiler default-path correction and double-lowering
  cleanup.
- #099: selfhost frontend incremental-parse design.

Do not collapse these into one generic "compiler blocked" bucket. The selfhost
transition lane can proceed or wait on fixpoint/parity evidence independently of
the trusted-base default-route decision.

---

## Execution Phases

### Phase 0: Baseline Establishment

**Purpose:** Fix current state. Observe only. Do not implement.

**Execution:**

```bash
cargo build -p arukellt
python scripts/manager.py verify quick
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
```

**Record:**
- `target/debug/arukellt` exists?
- `.build/selfhost/arukellt-s1.wasm` vs `s2.wasm` size difference
- Fixpoint failure root cause
- Fixture parity: pass/fail/skip counts
- Diagnostic parity: pass/fail/skip counts

**Known Root Cause (from `python scripts/manager.py selfhost fixpoint`):**
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
python scripts/manager.py selfhost fixpoint
bash scripts/run/verify-bootstrap.sh --check
```

**Phase 1 Exit Condition:** `stage1-self-compile: reached` AND `stage2-fixpoint: reached`

---

### Phase 2: Fixture Parity Expansion

**Goal:** Selfhost compiler handles general fixtures same as trusted base.

**Execution:**

```bash
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost parity --mode --fixture
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
python scripts/manager.py selfhost diag-parity
python scripts/manager.py selfhost parity --mode --diag
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
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
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
   python scripts/manager.py selfhost fixpoint
   python scripts/manager.py selfhost fixture-parity
   ```

3. **Implement**

4. **Minimal verification**

   ```bash
   python scripts/manager.py verify quick
   python scripts/manager.py verify fixtures
   ```

5. **Selfhost verification**

   ```bash
   python scripts/manager.py selfhost fixpoint
   python scripts/manager.py selfhost fixture-parity
   python scripts/manager.py selfhost diag-parity
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

### Criterion A: Selfhost Bootstrap (Phase 1 — #624)

- [x] `python scripts/manager.py selfhost fixpoint` passes — ACHIEVED
- [x] `verify-bootstrap.sh --check` reports `stage2-fixpoint: reached` — ACHIEVED

### Criterion B: Compiler Parity (Phase 2+3 — #594)

- [ ] Fixture parity: 0 fails
- [ ] Diagnostic parity: 0 critical fails
- [ ] Skip count: temporarily acceptable, target 0 eventually

### Criterion C: Rust Core Compiler Retirement (Phase 4+5 — #625, #564)

- [ ] Execution path switched to selfhost — ACHIEVED per ADR-029
- [ ] Dual-run period stable for 2+ weeks (#625)
- [ ] `crates/arukellt` deleted (#564)

### Criterion D: Full Rust Retirement (Phase 6+7 — #626, #627, #628, #571, #574-#582)

- [ ] IDE fully selfhost/native (Phase 6/A-C + #571)
- [ ] No Cargo workspace needed (#582)
- [ ] No Rust code in repository (#574-#582)

---

## Immediate Next Steps (First 3 Tasks)

**STOP before LSP, Rust deletion, or other phases until these complete:**

1. **Implement recursive module loading** in `src/compiler/driver.ark`
2. **Align qualified cross-module call resolution** in `src/compiler/mir.ark` and `src/compiler/emitter.ark`
3. **Verify** `python scripts/manager.py selfhost fixpoint` passes

**Rule:** Do not proceed to Phase 2+ until Phase 1 fixpoint is stable. Root cause isolation becomes impossible if you mix phases.
