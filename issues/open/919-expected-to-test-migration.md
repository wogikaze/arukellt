---
Status: open
Created: 2026-10-11
Updated: 2026-10-11
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

- [ ] Add `test:` kind to the fixture-parity harness
      (`scripts/selfhost/checks.py` `run_fixture_parity`): run
      `arukellt test` and assert exit 0 instead of diffing stdout.
- [ ] Migrate the 609 empty-`.expected` fixtures to `test` / `test mod`
      blocks; delete the `.expected` file; flip manifest `run:` → `test:`.
- [ ] Convert the 585 non-empty `.expected` stdout goldens to
      `assert_eq_*` assertions inside test blocks (16 benchmark files keep
      `.expected` as correctness check alongside `bench` runner).
- [ ] Regenerate fixture-count docs per wave (project-state.toml +
      generated docs).

## Acceptance

- [ ] `test:` manifest kind is handled by the parity harness and covered by
      at least one migrated directory end-to-end.
- [ ] `.expected` count decreases monotonically to the benchmark floor.
- [ ] `python3 scripts/manager.py verify fixtures` stays green per wave.
