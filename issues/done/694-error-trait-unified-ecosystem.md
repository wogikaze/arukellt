---
Status: done
Created: 2026-06-26
Updated: 2026-09-19
ID: 694
Track: stdlib-api
Depends on: "690, 692"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#690 ? operator + From, #692 From trait"
Blocks v{N}: none
Priority: 2
Source: Stdlib abstraction gap audit 2026-06-26 — Rust parity comparison
---

# 694 — `Error` trait and unified error type ecosystem

## Summary

Arukellt error types are fragmented: `parse_i32` returns `Result<i32, String>`,
`std::io` returns `Result<_, IoError>`, JSON/TOML/CSV each have their own
error shape, and there is no unifying `Error` trait or `From<E>` conversion.
Combined with the missing `?` operator (#690), error propagation across
library boundaries requires manual `match` and ad-hoc conversion at every
call site.

Rust's `std::error::Error` trait + `From<E1> for E2` + `?` provides a uniform
error ecosystem, enabling `anyhow`-style aggregation and library
interoperability.

## Current state

- `IoError` enum in `std::io` (concrete).
- `Result<i32, String>` from parse functions.
- `std::error::Error` is now a provisional static-dispatch trait. The stable
  `std::core::error::Error` enum remains unchanged.
- `Error: Display` is enforced as a supertrait. Its current ABI-safe source
  shape is `source(self: Error) -> Option<String>`; typed dynamic sources are
  deferred by ADR-064 because `dyn Trait` has no adopted object ABI.
- `std::io::IoError` and `String` implement `Error`. I/O APIs now expose
  `IoError`, while `From<IoError> for String` remains available for migration.

## Required work

- [x] Define `trait Error: Display` in `std::error`. Its `source` method uses
      `Option<String>` under the accepted ADR-064 static ABI boundary; the
      original `Option<Error>` shape is not claimed without a typed object ABI.
- [x] Require `Display` as a supertrait (depends on existing `Display` trait
      becoming generically dispatched — #688).
- [x] Enforce the `Display` supertrait and implement `Error` for `IoError` and
      `String`.
- [x] Implement the required `From` conversions at the concrete library and
      application boundaries (depends on #692).
- [x] Document a recommended top-level application error pattern (enum
      wrapping library errors with explicit `From` implementations).
- [x] Fixtures: a function returning `Result<T, AppError>` that uses `?` to
      propagate `IoError` and `parse` errors via `From`.
- [x] Regenerate stdlib docs and manifest.
- [x] Run the applicable current-artifact verification gates. The canonical
      `verify quick` contract remains externally blocked by the pre-existing
      #854 bootstrap mismatch and the unrelated #856 struct-pattern fixture;
      those failures are recorded below rather than hidden.

## Acceptance

- [x] `Error` trait defined with `Display` supertrait and the accepted
      ABI-safe `source` method.
- [x] At least `IoError` and the `String` parse-error representation implement
      `Error`.
- [x] A fixture demonstrates cross-library error propagation via `?` + `From`.
- [x] The applicable current-artifact verification gates pass; the canonical
      quick gate is tracked as a pre-existing bootstrap/recovery blocker, not
      as an implementation failure in this issue.

## References

- Depends on: #690 (`?` operator), #692 (`From` trait), #688 (trait dispatch)
- `std/core/error.ark`, `std/io/mod.ark`, `std/core/convert.ark`
- Rust `std::error`: <https://doc.rust-lang.org/std/error/index.html>

## Completion evidence (2026-09-19)

- Implementation: `7c4d7837b` (`language-v2: implement unified error trait
  ecosystem`). The same commit also fixes qualified nominal-type matching for
  imported enum paths and explicit `Copy` enum values, which is required by
  the typed `IoError` surface.
- Normative contract: [ADR-064](../../docs/adr/ADR-064-error-trait-namespace-and-source-boundary.md),
  with synchronized language guide/spec/error-handling documentation and
  generated stdlib reference pages.
- Public surface: `std::error::Error` is provisional and statically
  dispatched; `std::core::error::Error` remains the stable legacy enum.
  `IoError` has `Display`/`Error` implementations, and `String` implements
  `Error`. The `std::io` manifest signatures now use `IoError`.
- Positive fixtures: `tests/fixtures/stdlib_trait/error_trait.ark` and
  `tests/fixtures/question_mark/error_ecosystem.ark`; the latter exercises
  `From<IoError>` and `From<String>` into `AppError` with `?`. Diagnostic
  coverage is `tests/fixtures/diagnostics/error_supertrait_missing.ark`.
- Current compiler artifact: `.build/bootstrap-shim-694e/selfhost/arukellt-s2.wasm`
  built with registry audit `candidates=46605 matched=46605 mismatched=0`,
  CoreOp audit `candidates=21048 matched=21048 mismatched=0 unresolved=0`,
  and output size `5433448` bytes.
- Target parity: `error_ecosystem.ark` compiled as a `wasm32-gc` component
  (`core=10483` bytes, WIT `580` bytes) and as a `wasm32` linear module
  (`8273` bytes). Both artifacts validated with `wasm-tools`; both ran under
  Wasmtime with output `OK`. The GC component used GC/function-references/
  Memory64 execution flags.
- Applicable gates: `verify lane --gate quality-quick` passed;
  `verify lane --gate diag-parity` passed; the targeted fixture tranche passed
  `PASS=289 FAIL=0` across the stdlib I/O, ownership, question-mark, trait,
  diagnostics, and WIT domains; docs regeneration passed and current-artifact
  `docs check` passed `4/4`.
- Full current-only fixture receipt: `PASS=1698 FAIL=1
  COMPILE_VALIDATE=1699 EXECUTED=101 RUNTIME_UNSUPPORTED=2`. The sole failure
  is the pre-existing `match_extensions/struct_pattern.ark` unknown-enum
  diagnostic already recorded by #856; it is outside this issue and was not
  converted into a skip or allow-list entry.
- Bootstrap boundary: canonical `python3 scripts/manager.py selfhost
  build-compiler` was rerun and fails at the pre-existing #854 generic-enum
  bootstrap mismatch (`unbox` and `slice_get` both report `T` vs `T`). The
  current artifact receipt above is therefore explicit and does not claim a
  pinned bootstrap success.

## False-done guard

- [x] Acceptance is mapped to implementation, fixtures, generated docs, and
      current-artifact receipts above.
- [x] No new TODO, SKIP, semantic-debt allow-list entry, or baseline change
      was used to hide an unimplemented error path.
- [x] The remaining bootstrap and struct-pattern failures are named with
      their owning upstream issues and are not reclassified as this issue's
      implementation result.
