---
Status: open
Created: 2026-06-17
Updated: 2026-07-26
ID: 668
Track: wasi-feature
Parent: 074
Depends on: 074, 510, 714
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: P0 WASI P2 native checklist audit 2026-06-17 — post-#074 polish gaps
Status note: Parent #074 re-closed 2026-07-25; stderr + args/env bridges landed (gate-668-p2-stderr / gate-668-p2-args-env).
---

# 668 — P2 native component polish (post-#074)

## Summary

Issue #074 closed the minimum P2 native command path (`gate_074`: validate + wasmtime
`hello p2`; re-closed 2026-07-25). Several P0 items from the 2026-06-17 audit remain open: stderr,
fixture coverage, bootstrap honesty, docs alignment, and export hygiene.

This issue is the **parent polish gate** for P2 native command components. Do not
re-open #074 for individual slices; land evidence here and in focused child slices
if the queue grows.

## Background

- **#714 bridged close (2026-07-25):** `p2_component_wrap.py` deleted; in-tree
  bridged emit prints `hello p2` via component imports of `wasi:cli/stdout` +
  `wasi:io/streams` + stdout bridge. Guest core may still import a bridge
  `write` symbol; **guest-native** `get-stdout` + stream method call sites
  remain this issue's acceptance item.
- `tests/fixtures/wasi_p2_native/` has hello / exit_code / eprintln / args / env_var
  fixtures with slice gates. Guest-native stdout/stderr remains.
- `BOOTSTRAP_COMPONENT_STUB` remains in `scripts/selfhost/checks.py` (FD-07 risk;
  proof gates must use s2/s3, not the stub overlay).
- Live `wasi:cli/*` emit uses `@0.2.0` (dead `@0.2.6` path cleaned in this issue).

## Acceptance

- [x] P2 `eprintln` routes through `wasi:cli/stderr` (or documented equivalent
      streams path) with wasmtime stderr proof fixture
      (`tests/fixtures/wasi_p2_native/eprintln_stderr.ark`,
      `scripts/check/gate-668-p2-stderr.py`)
- [ ] Guest print path follows #714's coherent architecture: wrapper-free
      emitter-native WASI 0.2 component output using `wasi:cli/stdout.get-stdout`
      plus `wasi:io/streams` resource methods, not pseudo direct
      `wasi:cli/stdout::write` core imports
- [x] `tests/fixtures/wasi_p2_native/` gains runnable gates for at least:
      `eprintln_stderr.ark` ✅, `exit_code.ark` ✅, `args.ark` ✅, `env_var.ark` ✅
      Evidence: `scripts/check/gate-668-p2-args-env.py` — real
      `wasi:cli/environment` host + env bridge + GC Vec/Option assembly.
- [x] P2 native component size regression gate (no P1 adapter blob) lands in
      `scripts/check/gate-668-p2-size.py` with
      `docs/data/p2-native-component-size-baseline.toml` (~80KB savings vs
      historical adapter reference size)
- [x] Platform / current-state P2 native tier matches `gate_074` reality
      (`docs/platform/target-runtime-and-surfaces.md`, `docs/current-state.md`,
      `docs/state/component-model.md`; legacy `docs/target-contract.md` removed)
- [x] Normalize generated `wasi:cli/*` version strings across import/export sections
      (live path `@0.2.0`; dead `p2_command_run` `@0.2.6` fixed)
- [ ] Optional: P2 command-world WIT golden snapshot gate under
      `tests/fixtures/wasi_p2_native/` or `tests/fixtures/component/`
- [ ] Optional: component output metadata dump gate for P2 native artifacts
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Close gate

New script `scripts/check/gate-668-p2-native-polish.py` (or extend `gate_074`) that:

1. Compiles and runs all `wasi_p2_native/*` manifest entries under wasmtime
2. Asserts platform / current-state docs do not claim P2 native is deferred-only
3. Fails unless proof uses current s2/s3 (`lib.selfhost_s2`); bootstrap stub
   overlay is not required for the proof path

## Out of scope

- Library component routing (#667)
- Full WASI P2 filesystem/HTTP/sockets capability facades (#076, #077, #139)
- `arukellt_host` bridge retirement and HTTP/sockets import migration
  to standard WASI P2 (**#727**)
- Removing `BOOTSTRAP_COMPONENT_STUB` entirely (tracked separately if memory-budget
  work is needed; this issue only requires non-stub proof for gates)

## References

- `issues/done/074-wasi-p2-native-component.md`
- `issues/done/714-wasi-p2-emitter-native-component-output.md`
- `scripts/selfhost/p2_component_wrap.py`, `p2_guest_stdio_patch.py`
- `scripts/check/check-false-done-close-gates.py` (`gate_074`)
- `docs/process/false-done-prevention.md` (FD-07)
