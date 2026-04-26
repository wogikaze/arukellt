# Error Handling Convergence: Panic / ICE Policy

**Status**: done
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 615
**Parent**: #592
**Depends on**: —
**Track**: compiler / runtime / cli
**Orchestration class**: implementation-ready

---

## Summary

Child issue for #592 Stream 3 — Panic / ICE Policy.

The current codebase conflates: user errors (Result), assertions that should never fire
(ICE), runtime panics (unreachable/divide/OOB), and CLI error reporting. This issue
defines and enforces a clear policy separating those four cases.

---

## Scope

**In scope:**
- Write the panic/ICE policy as a doc in `docs/compiler/panic-ice-policy.md`:
  - ICE: internal compiler error that must not fire on user input; always produces a
    `[BUG] internal compiler error:` message; crashes with status 101
  - User panic: user `panic!` / unreachable — emitted as a runtime trap or a user-visible
    error; never a compiler crash
  - Result: recoverable failure always returned via `Result<T, E>`, never panics
  - CLI error reporting: user-facing error output from `arukellt` CLI — structured, not a
    Rust panic
- Audit compiler internals for assertions that would fire on user input and convert to
  structured diagnostics
- Audit CLI error paths for Rust `.unwrap()` / `.expect()` that would display a raw Rust
  panic traceback to the user, and replace with proper CLI error output

**Out of scope:**
- Full migration of compiler diagnostic model (that is #614)
- stdlib Result surface (that is #613)

---

## Primary paths

- `docs/compiler/panic-ice-policy.md` (new)
- `crates/arukellt/src/` (CLI error paths)
- `src/compiler/` (compiler ICE/assert audit)
- `crates/ark-diagnostics/` (ICE reporting)

## Allowed adjacent paths

- `crates/ark-runtime/` (runtime panic / trap behavior)

---

## Upstream / Depends on

None.

## Blocks

- Closes the panic/ICE stream of #592

---

## Acceptance

1. `docs/compiler/panic-ice-policy.md` defines the four categories
2. At least 3 instances of `.unwrap()` / `.expect()` in CLI paths are replaced with structured errors
3. At least 1 compiler assertion that fires on valid user input is converted to a structured diagnostic
4. ICE output format uses `[BUG]` prefix and exits status 101

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py docs check
```

Manual ICE behavior test:
```bash
# ICE must not produce raw Rust panic trace on user input
echo "invalid input that triggers an assertion" | ./target/release/arukellt compile - \
  && echo "should have failed" || echo "expected failure"
```

---

## STOP_IF

- Do not implement a new runtime exception model
- Do not change stdlib API error types (that is #613)
- Do not implement IDE-level diagnostics here

---

## Close gate

paths, at least one compiler assertion is demoted to a diagnostic, and ICE format is `[BUG]` + 101.

## Close note (2026-04-26)
Implementation complete in prior PR. Verified `docs/compiler/panic-ice-policy.md` and `crates/arukellt/src/main.rs`.
