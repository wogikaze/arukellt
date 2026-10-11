---
Status: open
Created: 2026-10-11
Updated: 2026-10-11
ID: 918
Track: tooling
Depends on: "719"
Orchestration class: implementation
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: ADR-041 Phase 2c — test runner UX split from #719 execution harness
---

# 918 — `arukellt test` runner UX: filter, no-fail-fast, directory discovery

## Summary

#719 landed the B1 execution harness: `arukellt test <file>` desugars test
decls into `__test_<n>` functions plus a synthesized `__test_main` entry,
emits a wasm32 module, and the wrapper executes it under wasmtime — trap is
FAIL, clean exit is PASS (fail-fast). This issue owns the remaining Phase-2c
runner UX described in ADR-041 D7.

## Current state

- `arukellt test <file>`: compile + exec works, verified by
  `scripts/check/gate-719-test-execution.py` (PASS lines per test, exit 0).
- Failure semantics: assertion trap aborts the module — first failure ends
  the run (fail-fast is the only mode).
- Execution target defaults to wasm32 (WASI p1 imports run under plain
  `wasmtime run`); `--target wasm32-gc` emits a module needing component
  packaging before execution.

## Required work

- [ ] `arukellt test <file> --filter <pattern>`: run only tests whose
      display name matches the pattern (drives synthesized `__test_main`
      call list or a runner-side filter).
- [ ] `arukellt test <file> --no-fail-fast`: continue after a trap. Requires
      per-test isolation — the B2 option in ADR-041 (compile each test as a
      separate module and exec independently), since a Wasm trap unwinds
      the whole module.
- [ ] `arukellt test <dir>/`: discover `*.ark` files under a directory and
      run each file`s tests.
- [ ] Output polish toward the ADR-041 nextest-style format: per-test
      timing and a `N passed, M failed, T total` summary line.

## Acceptance

- [ ] `--filter` narrows execution and reports which tests were skipped.
- [ ] `--no-fail-fast` reports every failure in one run (B2 isolation or an
      equivalent trap-boundary mechanism).
- [ ] Directory mode aggregates results across files.
- [ ] Gate coverage extended (gate-719 or a new gate) for each new flag.

## Notes

- Keep B1 fail-fast as the default; B2 is opt-in for isolation/parallelism.
- A gc-target exec path would need the component packaging step
  (`--emit component` + WASI adapter) — out of scope unless it unblocks B2.
