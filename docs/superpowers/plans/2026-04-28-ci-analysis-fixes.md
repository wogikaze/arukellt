# CI Analysis Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all identified CI failures from the local CI analysis, with priority on merge-blocking issues.

**Architecture:** The CI failures stem from 3 independent root causes: (1) missing `use` imports in the selfhost compiler's `main.ark` which prevents Stage 0 bootstrap and causes stage-2 wasm corruption, (2) stale/broken cross-references in documentation, (3) orphaned integration test file outside any Cargo package. Each task is independent.

**Tech Stack:** Rust (reference compiler), Arukellt (selfhost compiler), Python (verification scripts), Markdown (docs)

**Root cause dependency tree:**

```
main.ark missing imports (Task 1)
  └→ Stage 0 bootstrap fails
  └→ Stage-2 wasm corrupt (pre-existing emitter bug exposed)
  └→ CLI cannot start (loads corrupt stage-2 wasm first)
      └→ integration/packaging/determinism all blocked

Broken links (Task 2) — independent
Docs freshness (Task 3) — independent  
Test harness location (Task 4) — independent
```

---

## Task 1: Fix `main.ark` — add missing `use lsp` and `use analysis` imports

**Files:**
- Modify: `src/compiler/main.ark:6-7`

**Root cause:** `main.ark` references `lsp::run_session()` (line 411) and `analysis::analyze()` (line 425) as qualified names, but neither `use lsp` nor `use analysis` is present in the imports section. The Rust reference compiler correctly rejects this as `undefined module: lsp` and `undefined module: analysis`.

- [ ] **Step 1: Add the missing imports**

Insert `use lsp` and `use analysis` after line 6 (`use driver`).

File: `src/compiler/main.ark`, lines 1-8 (current):

```ark
use std::host::env
use std::host::fs
use std::host::process
use std::host::stdio
use diagnostics
use driver
```

Need to add after `use driver`:

```ark
use analysis
use lsp
```

- [ ] **Step 2: Verify Stage 0 compiles**

Run the bootstrap Stage 0 compilation:

```bash
cd /home/wogikaze/arukellt/.worktrees/ci-fixes
cargo build -p arukellt --release 2>&1 | tail -5
ARUKELLT_BIN=./target/release/arukellt bash scripts/run/verify-bootstrap.sh --check 2>&1
```

Expected: Stage 0 reaches "stage0-compile: reached" (the `lsp` and `analysis` modules compile successfully).

- [ ] **Step 3: Commit**

```bash
git add src/compiler/main.ark
git commit -m "fix(selfhost): add missing 'use lsp' and 'use analysis' imports in main.ark

main.ark references lsp::run_session() and analysis::analyze() via qualified
names but lacked the corresponding 'use' imports. This caused Stage 0 bootstrap
to fail with 'undefined module: lsp' and 'undefined module: analysis'.

Adding these imports resolves the bootstrap failure and allows the stage-2
selfhost compiler wasm to be generated correctly."
```

---

## Task 2: Fix broken links (7 broken cross-references)

**Files:**
- Modify: `docs/compiler/legacy-path-migration.md:119`
- Modify: `docs/compiler/legacy-path-status.md:14`
- Modify: `docs/current-state.md:224`
- Modify: `docs/migration/v4-to-v5.md:111`
- Modify: `issues/done/285-legacy-path-deprecation.md:42`
- Modify: `issues/open/index.md:35,131`
- (the `docs/playground/wasm/README.md` link is handled separately)

**Context:** After issue tracking was reorganized (some issues moved from `issues/open/` to `issues/done/` and vice versa), several markdown links became stale. The link checker (`scripts/check/check-links.sh`) reports 7 broken links. Each fix updates a relative path to point to the correct location.

Breakdown:

| # | Source file | Current target | Correct target | Issue |
|---|---|---|---|---|
| 1 | `docs/compiler/legacy-path-migration.md:119` | `../../issues/open/285-legacy-path-deprecation.md` | `../../issues/done/285-legacy-path-deprecation.md` | Issue 285 is done |
| 2 | `docs/compiler/legacy-path-status.md:14` | `../../issues/open/285-legacy-path-deprecation.md` | `../../issues/done/285-legacy-path-deprecation.md` | Same |
| 3 | `docs/current-state.md:224` | `../issues/open/510-t3-p2-import-table-switch.md` | `../issues/done/510-t3-p2-import-table-switch.md` | Issue 510 is done |
| 4 | `docs/migration/v4-to-v5.md:111` | `../../CHANGELOG.md` | Remove the link (CHANGELOG.md never existed in this repo) | No changelog file |
| 5 | `issues/done/285-legacy-path-deprecation.md:42` | `508-legacy-path-removal-unblocked-by.md` | `../open/508-legacy-path-removal-unblocked-by.md` | Relative from `issues/done/` → `issues/open/` |
| 6 | `issues/open/index.md:35` | `510-t3-p2-import-table-switch.md` | `../done/510-t3-p2-import-table-switch.md` | Issue 510 moved to done |
| 7 | `issues/open/index.md:131` | `510-t3-p2-import-table-switch.md` | `../done/510-t3-p2-import-table-switch.md` | Same |

- [ ] **Step 1: Fix `docs/compiler/legacy-path-migration.md`**

Line 119: Change `../../issues/open/285-legacy-path-deprecation.md` to `../../issues/done/285-legacy-path-deprecation.md`

- [ ] **Step 2: Fix `docs/compiler/legacy-path-status.md`**

Line 14: Change `../../issues/open/285-legacy-path-deprecation.md` to `../../issues/done/285-legacy-path-deprecation.md`

- [ ] **Step 3: Fix `docs/current-state.md`**

Line 224: Change `../issues/open/510-t3-p2-import-table-switch.md` to `../issues/done/510-t3-p2-import-table-switch.md`

- [ ] **Step 4: Fix `docs/migration/v4-to-v5.md`**

Line 111: Remove the `[CHANGELOG](../../CHANGELOG.md)` link line entirely (CHANGELOG.md does not exist in this repository).

- [ ] **Step 5: Fix `issues/done/285-legacy-path-deprecation.md`**

Line 42: Change `508-legacy-path-removal-unblocked-by.md` to `../open/508-legacy-path-removal-unblocked-by.md`

- [ ] **Step 6: Fix `issues/open/index.md`**

Line 35: Change `510-t3-p2-import-table-switch.md` to `../done/510-t3-p2-import-table-switch.md`
Line 131: Change `510-t3-p2-import-table-switch.md` to `../done/510-t3-p2-import-table-switch.md`

- [ ] **Step 7: Verify all links are fixed**

```bash
cd /home/wogikaze/arukellt/.worktrees/ci-fixes
bash scripts/check/check-links.sh 2>&1
```

Expected output: `FAILED: 0 broken link(s) in NNN files` (or no FAILED line at all). Note: the playground README link may still be broken independently.

- [ ] **Step 8: Commit**

```bash
git add docs/compiler/legacy-path-migration.md docs/compiler/legacy-path-status.md docs/current-state.md docs/migration/v4-to-v5.md issues/done/285-legacy-path-deprecation.md issues/open/index.md
git commit -m "fix(docs): repair 7 broken cross-reference links

- Point issue 285 references from issues/open/ to issues/done/
- Point issue 510 references from issues/open/ to issues/done/
- Fix relative path for 508 reference in done/285 from open/
- Remove dead CHANGELOG.md link (never existed in this repo)"
```

---

## Task 3: Regenerate generated docs

**Files:**
- Modify (generated): various files under `docs/` (produced by `scripts/gen/generate-docs.py`)

**Root cause:** The verification harness reports `✗ docs consistency` with "generated docs are out of date". This means source documentation has changed but the generated artifacts (manifest-backed reference pages, etc.) haven't been regenerated.

- [ ] **Step 1: Regenerate docs**

```bash
cd /home/wogikaze/arukellt/.worktrees/ci-fixes
python3 scripts/gen/generate-docs.py --check 2>&1
```

If this says "generated docs are out of date", run:

```bash
python3 scripts/gen/generate-docs.py 2>&1
```

Then verify:

```bash
python3 scripts/gen/generate-docs.py --check 2>&1
```

Expected: "generated docs are up to date"

- [ ] **Step 2: Check if there are changes**

```bash
git diff --stat 2>&1
```

If there are documentation file changes, commit them. If not (already up to date despite the verify check — this can happen due to timing), just proceed.

- [ ] **Step 3: Commit (only if docs changed)**

```bash
git add docs/
git commit -m "chore(docs): regenerate generated documentation"
```

---

## Task 4: Fix test harness location

**Files:**
- Move: `tests/harness.rs` → `crates/arukellt/tests/harness.rs`
- The `crates/arukellt/tests/` directory may need to be created

**Root cause:** The CI job runs `cargo test -p arukellt --test harness`, which looks for `crates/arukellt/tests/harness.rs`. But the file is at `tests/harness.rs` (workspace root). Cargo does not recognize root-level `tests/` as belonging to any workspace member, so the test can never be found.

The test uses `env!("CARGO_MANIFEST_DIR")` which would resolve correctly:
- If moved to `crates/arukellt/tests/harness.rs` → `CARGO_MANIFEST_DIR` = `crates/arukellt` → `.parent()` = workspace root → `.parent()` again would go above workspace root... Actually need to verify.

Let me check the path logic:

```rust
let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .parent()
    .unwrap();
```

If moved to `crates/arukellt/tests/harness.rs`:
- `CARGO_MANIFEST_DIR` = `/home/wogikaze/arukellt/.worktrees/ci-fixes/crates/arukellt`
- 1st `.parent()` = `/home/wogikaze/arukellt/.worktrees/ci-fixes/crates`
- 2nd `.parent()` = `/home/wogikaze/arukellt/.worktrees/ci-fixes`

But the fixture directory it looks for is `tests/fixtures` under workspace root, so `/home/wogikaze/arukellt/.worktrees/ci-fixes/tests/fixtures` — that's correct!

- [ ] **Step 1: Create the crate test directory**

```bash
mkdir -p crates/arukellt/tests
```

- [ ] **Step 2: Move the harness file**

```bash
git mv tests/harness.rs crates/arukellt/tests/harness.rs
```

- [ ] **Step 3: Verify the test compiles and runs**

```bash
cd /home/wogikaze/arukellt/.worktrees/ci-fixes
cargo test -p arukellt --test harness -- --nocapture 2>&1 | tail -20
```

Expected: The test compiles and starts running fixtures (may produce fixture failures, but the test target should be found).

- [ ] **Step 4: Run the CI fixture command to verify**

```bash
ARUKELLT_SELFHOST_WASM=/home/wogikaze/arukellt/.worktrees/ci-fixes/bootstrap/arukellt-selfhost.wasm \
ARUKELLT_BIN=./target/release/arukellt \
python3 scripts/manager.py verify fixtures 2>&1
```

Expected: The fixture harness is found and runs (results may vary, but no "no test target" error).

- [ ] **Step 5: Commit**

```bash
git add crates/arukellt/tests/harness.rs
git commit -m "fix(ci): move tests/harness.rs into arukellt crate test directory

The integration test harness was at the workspace root (tests/harness.rs)
which is not recognized by Cargo as belonging to any workspace member.
Moved to crates/arukellt/tests/harness.rs so that CI's
'cargo test -p arukellt --test harness' can find and execute it."
```

---

## Task 5: Run full CI verification to confirm fixes

**Files:** n/a (verification only)

- [ ] **Step 1: Run unit tests**

```bash
cd /home/wogikaze/arukellt/.worktrees/ci-fixes
cargo clippy --workspace -- -D warnings 2>&1 | tail -3
cargo fmt --all -- --check 2>&1
cargo test --workspace --lib --bins -- --nocapture 2>&1 | tail -10
```

Expected: clippy OK, rustfmt OK, all tests pass.

- [ ] **Step 2: Run verification harness quick**

```bash
ARUKELLT_SELFHOST_WASM=/home/wogikaze/arukellt/.worktrees/ci-fixes/bootstrap/arukellt-selfhost.wasm \
ARUKELLT_BIN=./target/release/arukellt \
python3 scripts/manager.py verify --quick 2>&1
```

Expected: Fewer failures than the original run. Specifically:
- `selfhost LSP lifecycle gate` and `selfhost analysis API gate` — should still fail until stage-2 wasm is rebuilt (requires bootstrap pipeline)
- `docs consistency` — should now pass (fixed in Task 3)
- `doc example check` — may still fail (stage2 wasm issue)
- `broken internal links` — should now pass (fixed in Task 2)

- [ ] **Step 3: Run integration smoke tests**

```bash
ARUKELLT_SELFHOST_WASM=/home/wogikaze/arukellt/.worktrees/ci-fixes/bootstrap/arukellt-selfhost.wasm \
./target/release/arukellt --help > /dev/null 2>&1 && echo "PASS: --help"
ARUKELLT_SELFHOST_WASM=/home/wogikaze/arukellt/.worktrees/ci-fixes/bootstrap/arukellt-selfhost.wasm \
./target/release/arukellt --version > /dev/null 2>&1 && echo "PASS: --version"
```

Expected: Both pass.

- [ ] **Step 4: Run selfhost bootstrap (Stage 0 verification)**

```bash
cd /home/wogikaze/arukellt/.worktrees/ci-fixes
cargo build --release -p arukellt 2>&1 | tail -3
ARUKELLT_BIN=./target/release/arukellt bash scripts/run/verify-bootstrap.sh --check 2>&1 | head -30
```

Expected: Stage 0 now reaches "stage0-compile: reached". Stage 1 and Stage 2 may still fail due to the pre-existing wasm emitter bug.

- [ ] **Step 5: Report final results**

Provide a summary of which issues were fixed and which remain (pre-existing).
