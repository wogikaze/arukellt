---
Status: open
Created: 2026-06-15
Updated: 2026-07-25
ID: 649
Track: native-cpp
Depends on: "641,834"
Orchestration class: design-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: ADR-050 experimental public native C99 run
---

# 649 — Experimental public `run --target native-cpp` (`run_supported=true`)

## Summary

`native-cpp` already has a C99 emitter, runtime/GC, root liveness, and an
**internal** selfhost executor lane (ADR-049, #834). This issue tracks the
**public** CLI path:

```text
arukellt run program.ark --target native-cpp -- arg1 arg2
```

Canonical design: [ADR-050](../../docs/adr/ADR-050-experimental-public-native-c99-run.md)  
Executable plan: [native-cpp-public-run-promotion.md](../../docs/plans/native-cpp-public-run-promotion.md)

`run_supported=true` means a documented capability subset runs on Linux x86-64 via
public CLI. It does **not** mean full MIR/CoreOp coverage or `support_tier=supported`.

## Current baseline (already true)

- [x] `compile --target native-cpp` generates C99
- [x] C runtime + mark-sweep GC + production root clears
- [x] Internal executor strict 3× PASS (`docs/data/native-cpp-executor-promotion-receipt.json`)
- [x] ADR-050 accepted; ADR-049 remains owner of the internal executor lane

## Gaps

- [x] Host launcher (`scripts/run/native-cpp-runner.py`) + wrapper dispatch
- [x] Shared clang 14+ toolchain resolver
- [x] Formal `--emit c` + default `.c` output + emit matrix diagnostics
- [x] Entry signature + args argv[0] parity
- [x] Public run GC default ON (runner + runtime unset default; executor keeps explicit 0/1)
- [x] stdio / cwd / exit / signal contracts + fixtures (env CoreOp still planned)
- [x] User-facing trap/panic diagnostics (`ark_rt_trap_kind` / `ark_rt_panic`)
- [x] Supported fs I/O returns Result (write open/write failures do not abort)
- [ ] `tests/fixtures/native_cpp_public/` corpus + Wasm/native parity
- [ ] HOF/indirect call or explicit Known Limitation + blocker
- [ ] PHI / de-SSA for user programs
- [ ] Installed runtime layout + smoke
- [ ] Release guarantee + `native-cpp-run-promotion-receipt.json`
- [ ] Final commit sets `run_supported=true` (never earlier)

## Non-goals

- `support_tier=supported`
- Windows / macOS / cross compile
- Public stable C ABI / external FFI
- `native-llvm` productization
- Replacing ADR-029 Wasmtime fixpoint
- `--emit exe` inside the compiler (host launcher owns clang)

## Acceptance

- [ ] ADR-050 + plan checkboxes through Final checklist
- [ ] `arukellt run <fixture> --target native-cpp` executes a native binary end-to-end
- [ ] `compile --target native-cpp --emit c` is the formal contract; default output `.c`
- [ ] Public positive/negative fixtures PASS; parity + ASan/UBSan PASS
- [ ] Installed-layout smoke PASS (no accidental source-tree dependency)
- [ ] `docs/data/project-state.toml` has `run_supported=true` with scaffold/partial/experimental
- [ ] Release guarantee `run_native_cpp_experimental` PASS; promotion receipt fresh
- [ ] Internal executor strict lane and ADR-029 fixpoint still PASS
- [ ] `python3 scripts/manager.py verify quick` exits 0
- [ ] `python3 scripts/manager.py docs check` exits 0 (or project-accepted exceptions only)

## Required verification

```bash
python3 scripts/manager.py verify lane
# after public runner exists:
arukellt run tests/fixtures/native_cpp_public/<hello>.ark --target native-cpp
python3 scripts/manager.py verify quick
python3 scripts/manager.py docs check
```

## Close gate

Do not close until the plan's Final checklist is complete and `run_supported=true`
is set only in the final promotion commit with a fresh
`docs/data/native-cpp-run-promotion-receipt.json`.
