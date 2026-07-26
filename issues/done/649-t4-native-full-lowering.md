---
Status: done
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

Public `arukellt run --target native-cpp` is experimental on Linux x86-64
(ADR-050). Pathway: Ark → MIR → C99 → clang → native executable.

Canonical design: [ADR-050](../../docs/adr/ADR-050-experimental-public-native-c99-run.md)  
Executable plan: [native-cpp-public-run-promotion.md](../../docs/plans/native-cpp-public-run-promotion.md)  
Promotion receipt: [native-cpp-run-promotion-receipt.json](../../docs/data/native-cpp-run-promotion-receipt.json)

## Done evidence

- [x] ADR-050 + plan through Final checklist
- [x] Host launcher + `--emit c` + entry/args/stdio/exit/panic/trap
- [x] Public corpus + HOF zero-capture + PHI edge copies
- [x] Wasm/native parity + UBSan/`-Werror` + installed layout
- [x] CI public run gates + `run_native_cpp_experimental` guarantee
- [x] `docs/data/project-state.toml` `run_supported=true` (scaffold/partial/experimental)
- [x] Promotion + public coverage receipts committed
- [x] Capture closures documented as Known Limitation (ADR-050)

## Close note

Closed with experimental public run promoted. Capture closures, public C ABI/FFI,
and `support_tier=supported` remain out of scope.
