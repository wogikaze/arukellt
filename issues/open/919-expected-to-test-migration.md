---
Status: open
Created: 2026-10-11
Updated: 2026-10-15
ID: 919
Track: tooling
Depends on: "719"
Orchestration class: incremental
Orchestration upstream: "#719 execution harness"
Blocks v{N}: none
Priority: 2
Source: ADR-041 Phase 2d — .expected golden migration split from #719
---

# 919 — `.expected` to `test:` fixture migration (ADR-041 Phase 2d)

## Summary

With #719 executable, stdout-golden `.expected` fixtures can migrate to
in-file `test` / `test mod` blocks plus a `test:` manifest kind handled by
the fixture-parity harness. This issue owns that migration, tracked per
directory so progress is reviewable.

## Current state

- `arukellt test` executes in-file tests (gate-719 green).
- `tests/fixtures/manifest.txt` kinds: run, diag, fmt, module-run,
  module-diag, t3-compile, component-*, compile-error — no `test:` kind.
- ~609 empty `.expected` files (already assert-based bodies) and ~585
  non-empty `.expected` files (stdout goldens) per #719 audit.

## Required work

- [x] Add `test:` kind to the fixture-parity harness
      (`scripts/selfhost/fixture_parity.py` `run_fixture_parity`): run
      `arukellt test` and assert exit 0 instead of diffing stdout.
- [ ] Migrate the 609 empty-`.expected` fixtures to `test` / `test mod`
      blocks; delete the `.expected` file; flip manifest `run:` → `test:`.
- [ ] Convert the 585 non-empty `.expected` stdout goldens to
      `assert_eq_*` assertions inside test blocks (16 benchmark files keep
      `.expected` as correctness check alongside `bench` runner).
- [ ] Regenerate fixture-count docs per wave (project-state.toml +
      generated docs).

## Update 2026-10-15 — harness kind landed + pilot dir

- `test:` manifest kind implemented in
  `scripts/selfhost/fixture_parity.py::run_fixture_parity` (the live gate —
  the same-named legacy function in checks.py is rebound by
  selfhost.__init__ and was left untouched).  Each `test:` entry runs
  `arukellt test <file>` through `arukellt-selfhost.sh` with
  `ARUKELLT_SELFHOST_WASM=<current s2>`; exit 0 = pass, trap/non-zero =
  fixture failure.  Pinned bootstrap is not consulted (predates the test
  harness).  `--filter-dir` on a test-only directory no longer errors on
  the empty run: list.
- Pilot: `test_syntax/basic.ark` flipped `t3-compile:` → `test:`.
  Verified end-to-end:
  `selfhost fixture-parity --filter-dir test_syntax` →
  `test: PASS=1 FAIL=0`; `--filter-dir hello` (run-only) → PASS=5 FAIL=0
  (no regression on the run path).
- Remaining: the bulk .expected migration waves (609 empty + 585 golden).
  Mechanical recipe per file: wrap body asserts in `test "…" {}`/`test
  mod`, drop `.expected`, flip `run:`→`test:`, regenerate fixture
  counts.

## Update 2026-10-28 — Wave D: 35 fixtures migrated (verification blocked)

- Migrated 35 fixtures in 3 directories, each wrapping the original
  `fn main` asserts in a single behaviour-named
  `test "<name>" { … }` block (no test splitting), deleting the empty
  `.expected`, and flipping its manifest line `run:` → `test:` in place:
  - `stdlib_hashmap/` — 19 fixtures
  - `simd_conformance/` — 14 fixtures (7 base + 7 `_wasm32` twins)
  - `stdlib_sort/` — 2 fixtures
- Manifest: `test:` 1 → 36, `run:` 1913 → 1878 (measured 1880 after
  concurrent additions by other lanes). Empty `.expected` 634 → 599;
  non-empty `.expected` untouched at the time of this wave.
- Formatting: `python3 scripts/manager.py fmt --check` on all 35 changed
  `.ark` files → `checked=35 failed=0`.
- Verification status: **not completed — do not claim green for this wave.**
  - A combined `--filter-dir stdlib_hashmap --filter-dir simd_conformance
    --filter-dir stdlib_sort` run exceeded 10 minutes and was killed to
    release the shared `selfhost-runtime` lock (Lead instruction).
  - An isolated `ARUKELLT_BUILD_DIR=wt/fixture-migration/build` run for
    `stdlib_sort` also failed to finish within 5 minutes.
  - Retrying direct `arukellt test` then failed because the concurrent
    selfhost rebuild removed `.build/selfhost/arukellt-s2-runtime.wasm`
    and still held `.build/selfhost-runtime.lock`. The older
    `arukellt-s2-runtime-post891.wasm` fallback rejects these stdlib
    fixtures with a pre-existing `E0200 non-iterable source` error, so it
    is not a valid oracle for this wave.
  - No `test: PASS/FAIL` numbers were recorded. Next step: after the
    selfhost runtime rebuild completes, run fixture-parity per directory;
    roll back any failing fixture to `run:` (restore its `.expected`).
- The migration changes were swept into commits `cd5f1eb07` /
  `e378f0aaf` by another process; no commit was made by this lane.

## Verification after the rebuild window (Lead, 2026-10-28 21:10)

The blocker above was an artifact/lock state, not a fixture defect. `arukellt test`
does not need a stage-2 artifact: the wrapper desugars `test` blocks into
`__test_*` functions and the pinned bootstrap compiles them. Re-running the whole
`test:` manifest set with the default pinned compiler therefore verifies the wave:

```bash
export PATH=$PWD/wt/wasmtime-v46.0.1-x86_64-linux:$PATH
while read -r line; do f=${line#test:}; scripts/run/arukellt-selfhost.sh test "tests/fixtures/$f" || echo "FAIL $f"; done < <(grep '^test:' tests/fixtures/manifest.txt)
```

Result: **`pass=36 fail=0`**, i.e. the 1 pre-existing `test_syntax/basic.ark` plus all
35 migrated fixtures in `stdlib_hashmap/` (19), `simd_conformance/` (14) and
`stdlib_sort/` (2) execute and pass. The fixture-parity `test:` kind asserts exactly
this (`arukellt test` exit 0), so no fixture needs to be rolled back to `run:`.
`fmt --check` on the 35 files stays `checked=35 failed=0`.

Remaining for this issue: continue the waves for the other directories (empty
`.expected` 599 left; non-empty goldens 757), and regenerate fixture-count docs
per wave. The `.expected` count decreased monotonically (634 → 599).

## Acceptance

- [ ] `test:` manifest kind is handled by the parity harness and covered by
      at least one migrated directory end-to-end.
- [ ] `.expected` count decreases monotonically to the benchmark floor.
- [ ] `python3 scripts/manager.py verify fixtures` stays green per wave.
