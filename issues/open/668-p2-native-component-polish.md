---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 668
Track: wasi-feature
Parent: 074
Depends on: 074, 510, 714
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: P0 WASI P2 native checklist audit 2026-06-17 — post-#074 polish gaps
---

# 668 — P2 native component polish (post-#074)

## Summary

Issue #074 closed the minimum P2 native command path (`gate_074`: validate + wasmtime
`hello p2`). Several P0 items from the 2026-06-17 audit remain open: stderr,
fixture coverage, bootstrap honesty, docs alignment, and export hygiene.

This issue is the **parent polish gate** for P2 native command components. Do not
re-open #074 for individual slices; land evidence here and in focused child slices
if the queue grows.

## Background

- Stdout uses `p2_component_wrap.py` + canonical ABI patch; stderr and direct
  `wasi:io/streams` guest imports are not wired. #714 tracks the architecture
  correction: emit wrapper-free WASI 0.2 Component Model output directly from the
  compiler instead of repairing pseudo core imports after compilation.
- `tests/fixtures/wasi_p2_native/` contains only `hello.ark`.
- `BOOTSTRAP_COMPONENT_STUB` remains in `scripts/selfhost/checks.py` (FD-07 risk).
- `docs/target-contract.md` still says P2 native is "deferred to v5+" while
  `docs/current-state.md` documents `gate_074` as green.
- `wasi:cli` version strings mix `@0.2.0` (imports) and `@0.2.6` (exports).

## Acceptance

- [ ] P2 `eprintln` routes through `wasi:cli/stderr` (or documented equivalent
      streams path) with wasmtime stderr proof fixture
- [ ] Guest print path follows #714's coherent architecture: wrapper-free
      emitter-native WASI 0.2 component output using `wasi:cli/stdout.get-stdout`
      plus `wasi:io/streams` resource methods, not pseudo direct
      `wasi:cli/stdout::write` core imports
- [ ] `tests/fixtures/wasi_p2_native/` gains runnable gates for at least:
      `eprintln_stderr.ark`, `exit_code.ark`, `args.ark`, `env_var.ark`
      (rename or alias `hello.ark` → `hello_stdout.ark` if desired)
- [ ] P2 native component size regression gate (no P1 adapter blob) lands in
      `scripts/check/` with a checked-in threshold (~80KB savings vs adapter path)
- [ ] `docs/target-contract.md` P2 native tier matches `current-state.md` and
      `gate_074` reality
- [ ] Normalize generated `wasi:cli/*` version strings across import/export sections
- [ ] Optional: P2 command-world WIT golden snapshot gate under
      `tests/fixtures/wasi_p2_native/` or `tests/fixtures/component/`
- [ ] Optional: component output metadata dump gate for P2 native artifacts
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Close gate

New script `scripts/check/gate-668-p2-native-polish.py` (or extend `gate_074`) that:

1. Compiles and runs all `wasi_p2_native/*` manifest entries under wasmtime
2. Asserts `docs/target-contract.md` does not claim P2 native is deferred-only
3. Fails if `BOOTSTRAP_COMPONENT_STUB` is still required for the proof path
   (document exception if bootstrap-only; proof must use s2/pinned non-stub path)

## Out of scope

- Library component routing (#667)
- Full WASI P2 filesystem/HTTP/sockets capability facades (#076, #077, #139)
- `arukellt_host` bridge retirement and HTTP/sockets import migration
  to standard WASI P2 (**#727**)
- Removing `BOOTSTRAP_COMPONENT_STUB` entirely (tracked separately if memory-budget
  work is needed; this issue only requires non-stub proof for gates)

## References

- `issues/done/074-wasi-p2-native-component.md`
- `issues/open/714-wasi-p2-emitter-native-component-output.md`
- `scripts/selfhost/p2_component_wrap.py`, `p2_guest_stdio_patch.py`
- `scripts/check/check-false-done-close-gates.py` (`gate_074`)
- `docs/process/false-done-prevention.md` (FD-07)
