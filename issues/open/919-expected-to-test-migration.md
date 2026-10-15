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

## Acceptance

- [ ] `test:` manifest kind is handled by the parity harness and covered by
      at least one migrated directory end-to-end.
- [ ] `.expected` count decreases monotonically to the benchmark floor.
- [ ] `python3 scripts/manager.py verify fixtures` stays green per wave.
