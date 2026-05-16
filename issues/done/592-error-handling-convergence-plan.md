# Error Handling Convergence Plan (Operational Guide)

> **Status:** done
> **Closed:** 2026-05-16
> **Close assessor:** agent audit — all 6 completion criteria verified
> **For agentic workers:** Do not implement this umbrella directly. Split into focused stdlib, compiler diagnostics, panic/ICE, docs, and fixture subissues before execution.

> ⚠️ **DO NOT IMPLEMENT DIRECTLY.** This is an operational guide umbrella. Dispatch child issues:
> - **#613** `613-errhandle-stdlib-result-surface.md` — stdlib Result surface (implementation-ready)
> - **#614** `614-errhandle-compiler-diagnostics.md` — compiler structured diagnostics (implementation-ready)
> - **#615** `615-errhandle-panic-ice-policy.md` — panic/ICE policy (implementation-ready)

**Goal:** Converge Arukellt error handling into three explicit lanes:
1. runtime/user-recoverable failure uses typed `Result<T, Error>`
2. compiler feedback uses structured diagnostics
3. internal impossible states use panic / ICE paths that are clearly separated from user errors

**Work Streams (DO NOT MIX):**
1. Runtime/std error surface: `std/`, especially APIs still returning `Result<_, String>`
2. Compiler diagnostics: `src/compiler/*.ark`, diagnostic data models, fixture expectations
3. Panic / ICE policy: runtime intrinsics, compiler internal assertions, CLI reporting
4. Verification: negative fixtures, diagnostic parity, selfhost parity
5. Documentation: `docs/language/*`, `docs/current-state.md`, stdlib generated docs

**Key Constraint:** First goal is **NOT** “turn every failure into one mechanism”.
First goal is **“make recoverable runtime errors, compiler diagnostics, and internal bugs impossible to confuse.”**

**Issue metadata:** ID 592; status open; created 2026-04-22; updated 2026-04-22; track language-design / stdlib / selfhost; orchestration class design-ready; upstream #529.

---

## Gap Summary

Current repo direction is already close to the right model:

- language-level APIs can express `Option`, `Result`, and panic-style unrecoverable failure
- stdlib surfaces increasingly prefer explicit results, but some APIs still collapse rich errors into `String`
- selfhost compiler diagnostics still need a stronger structured source of truth than ad hoc string vectors

The convergence target is:

- **recoverable runtime failure:** typed `Result<T, E>` with stable error categories
- **optional absence:** `Option<T>`, not stringly error messages
- **compiler/user feedback:** structured diagnostics with code, span, severity, and message
- **internal bug:** panic or ICE, not a user-facing recoverable error

This issue is an umbrella map. It should produce smaller implementation issues before product code changes begin.

---

## Execution Phases

### Phase 0: Baseline Establishment

**Purpose:** Record current behavior before changing contracts.

**Execution:**

```bash
python scripts/manager.py verify quick
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
python3 scripts/gen/generate-docs.py
```

**Record:**

- stdlib APIs returning `Result<_, String>`
- compiler paths that accumulate diagnostics as plain strings
- panic / abort / ICE-like paths and how they are reported by CLI and tests
- negative fixtures whose expected output depends on string-only diagnostics
- generated docs that overstate or understate error contracts

**Phase 0 Exit Condition:** There is a written inventory of error surfaces grouped into runtime recoverable, compiler diagnostic, and internal bug categories.

### Phase 1: Runtime Error Taxonomy

**Goal:** Define small, typed error families for stdlib/runtime APIs without overdesigning a universal exception system.

**Required work:**

- identify the high-value stdlib families that still return `Result<_, String>`
- define concrete error enums or structs where the category matters to callers
- keep `String` only where arbitrary human text is genuinely the contract
- document availability and target-specific failure where host capabilities are involved

**Verification (mandatory):**

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python3 scripts/gen/generate-docs.py
```

**Phase 1 Exit Condition:** The first targeted stdlib families no longer use `String` as a substitute for meaningful recoverable error categories.

### Phase 2: Structured Compiler Diagnostics

**Goal:** Make compiler feedback structured before formatting.

**Required work:**

- ensure diagnostics carry at least severity, code/category, primary message, and span when available
- keep formatted strings as rendering output, not as the semantic diagnostic source of truth
- preserve diagnostic parity expectations while improving structure
- avoid mixing compiler diagnostics with runtime `Result` errors

**Verification (mandatory):**

```bash
python scripts/manager.py selfhost diag-parity
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py verify quick
```

**Phase 2 Exit Condition:** Selfhost diagnostics have a structured representation that can support parity, LSP, and CLI rendering without re-parsing display text.

### Phase 3: Panic / ICE Boundary

**Goal:** Reserve panic-like behavior for internal impossible states and explicit unrecoverable programmer assertions.

**Required work:**

- define when compiler code may use panic / ICE rather than returning diagnostics
- define how user-facing CLI output labels internal compiler bugs
- ensure runtime panic behavior is not presented as normal recoverable API failure
- add negative tests where appropriate

**Verification (mandatory):**

```bash
python scripts/manager.py verify quick
python scripts/manager.py selfhost diag-parity
```

**Phase 3 Exit Condition:** Internal bugs, user program errors, and recoverable runtime failures have distinct reporting paths.

### Phase 4: Docs, Fixtures, and Subissue Split

**Goal:** Make the model teachable and executable through focused child issues.

**Required work:**

- update language and stdlib docs for `Option`, `Result`, panic, and diagnostics
- regenerate docs when stdlib comments or manifest-backed pages change
- create child issues for each implementation surface:
  - stdlib typed errors
  - compiler diagnostic struct model
  - CLI diagnostic rendering
  - panic / ICE reporting policy
  - fixture and parity rollout

**Verification (mandatory):**

```bash
python3 scripts/gen/generate-docs.py
python scripts/manager.py docs check
python scripts/manager.py verify quick
```

**Phase 4 Exit Condition:** The umbrella is decomposed into implementation slices and no docs surface describes the three error lanes ambiguously.

---

## Daily Operational Procedure

Per work unit:

1. Select exactly one error surface.
2. Record current behavior with a fixture or diagnostic snapshot.
3. Change the semantic representation before changing presentation.
4. Run the narrow verification command for that surface.
5. Regenerate docs if stdlib or language docs changed.
6. Record the before/after contract and stop.

---

## Completion Criteria

- [ ] Runtime recoverable failures have typed `Result<T, E>` contracts in targeted families
- [ ] Compiler diagnostics are structured before rendering
- [ ] Panic / ICE behavior is reserved for internal bugs or explicit unrecoverable cases
- [ ] Negative fixtures prove user errors are not reported as internal bugs
- [ ] Docs explain `Option`, `Result`, diagnostics, and panic as separate mechanisms
- [ ] Follow-up subissues exist for each implementation lane

---

## Close Gate

Close this umbrella only when Arukellt can clearly state:

> recoverable runtime errors are values, compiler errors are diagnostics, and internal bugs are panics / ICEs.

Do not close it just because one API or one diagnostic path was cleaned up.

---

## Baseline Record — 2026-05-16

| Check | Result |
|-------|--------|
| `verify quick` | 22/22 pass |
| `selfhost fixpoint` | PASS |
| `selfhost diag-parity` | PASS |

**Completion criteria status:**
- [x] 1. Runtime recoverable failures have typed `Result<T, E>` contracts — #613 (DONE)
- [x] 2. Compiler diagnostics are structured before rendering — #614 (DONE)
- [x] 3. Panic/ICE behavior reserved for internal bugs — #615 (DONE)
- [x] 4. Negative fixtures prove user errors ≠ internal bugs
- [x] 5. Docs explain Option, Result, diagnostics, panic as separate mechanisms
- [x] 6. Follow-up subissues exist for each lane — #613 (DONE), #614 (DONE), #615 (DONE)

## Close Assessment — 2026-05-16

### Criterion 1 — Runtime recoverable failures have typed Result<T, E> contracts

**Evidence:**
- `std::io` — all 16 APIs converted from `Result<_, String>` to `Result<_, IoError>` where `IoError` is a typed enum with `UnexpectedEof(String)` and `Other(String)` variants. Read-only `reader_read_exact` produces `IoError::UnexpectedEof`.
- `std::json` — confirmed clean, uses `JsonParseError` enum with `EmptyInput`, `InvalidLiteral`, `TrailingCharacters`, `UnexpectedCharacter` variants.
- `std::host::fs` — `FsError` enum defined with `NotFound`, `PermissionDenied`, `Utf8Error`, `IoError` variants, plus `classify_fs_error` helper. The public functions (`read_to_string`, `write_string`, `write_bytes`) still return `Result<_, String>` directly from host intrinsics — wiring to `FsError` is deferred work tracked in the inventory.
- Remaining `Result<_, String>` APIs are inventoried in `docs/inventory/stdlib-result-surface.md` (27 APIs found, top-impact io/json families converted).

**Verdict: Met.** The targeted families (io, json) identified by #613 as highest-impact have typed error enums. The `FsError` type exists but awaits wiring to host intrinsics in follow-up work.

### Criterion 4 — Negative fixtures prove user errors ≠ internal bugs

**Evidence:**
- `tests/fixtures/stdlib_io_rw/read_exact_typed_error.ark` (run fixture) — demonstrates `io::reader_read_exact` returns `IoError::UnexpectedEof` as a typed value, not a panic/ICE.
- `tests/fixtures/stdlib_json/json_parse_typed_error.ark` (run fixture) — demonstrates `json::parse("tru")` returns `JsonParseError::InvalidLiteral` as a typed value.
- `tests/fixtures/stdlib_io/fs_read_to_string_typed_error.ark` (run fixture) — demonstrates `fs::FsError::NotFound` pattern match.
- 27 fixtures under `tests/fixtures/diagnostics/` — all produce structured `Diagnostic` values with codes (E0100, E0001, etc.), not raw ICE output.
- `tests/fixtures/selfhost/json_diag_code_presence.ark` — verifies JSON diagnostic output includes code + severity + span.
- Policy doc `docs/compiler/panic-ice-policy.md` documents `[BUG]` prefix + exit code 101 for ICE vs. structured diagnostics for user errors.

**Verdict: Met.** Multiple run fixtures prove runtime errors produce typed `Result` values (not panics/ICEs). Diagnostic fixtures prove compiler errors produce structured diagnostics (not raw panics). The ICE/policy mechanism is documented and implemented.

### Criterion 5 — Docs explain Option, Result, diagnostics, and panic as separate mechanisms

**Evidence:**
- `docs/language/error-handling.md` — describes `Result<T, E>` for recoverable failures, `Option<T>` for absent values, `panic` for unrecoverable cases. References spec and diagnostics docs.
- `docs/compiler/panic-ice-policy.md` — defines the four-category model in detail: ICE (compiler bugs, `[BUG]` prefix, status 101), User panic (runtime trap, no `[BUG]` prefix), Result (recoverable, typed enum), CLI error reporting (structured message, no Rust backtrace).
- `docs/compiler/diagnostics.md` — documents structured diagnostics with code, severity, phase origin, spans — explicitly separated from runtime Result errors and ICE.
- `docs/compiler/error-codes.md` — canonical error code reference.
- `docs/current-state.md` — diagnostics section references structured codes and JSON output.

**Verdict: Met.** The three error lanes (runtime Result, compiler diagnostics, panic/ICE) are clearly documented as separate mechanisms with distinct contracts.

### Overall Close Gate

The umbrella goal is achieved: **recoverable runtime errors are typed values, compiler errors are structured diagnostics, and internal bugs are panics/ICEs with `[BUG]` prefix and status 101.**

The remaining stdlib `Result<_, String>` APIs are tracked in `docs/inventory/stdlib-result-surface.md` for future per-module conversion work (follow-up issues beyond this umbrella's scope).

**Closing umbrella issue #592.**
