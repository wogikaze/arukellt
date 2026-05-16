---
Status: open
Created: 2026-04-22
Updated: 2026-05-14
ID: 606
Track: stdlib
Orchestration class: blocked-by-upstream
Depends on: 604
Parent: #590
Fix contract ambiguity and semantic gaps in `std: ":json`, `std::toml`, `std::text`, and"
In scope: 
JSON: 
TOML: 
Text: 
Time: 
Out of scope: 
Close when: all four families have updated docs, negative fixtures pass, and the
---

# Stdlib Baseline: Structured Data and Semantics Baseline
`std: ":time` / `std::host::clock` boundary."
- `std: ":text` must clearly distinguish byte-based vs char/Unicode-aware operations"
- Add fixtures for at-boundary behavior: key types, array of tables edge cases
- Split `std: ":time` (duration math) from `std::host::clock` (actual host time reads)"
- Do not let `std: ":time` imply runtime host access it does not have"
1. `std: ":json` parse rejects trailing garbage in a negative fixture"
2. `std: ":toml` bounded subset is prominently documented with negative fixtures at boundaries"
3. `std: ":text` distinguishes byte vs char operations in doc comments"
4. `std: ":time` / `std::host::clock` split is explicit in docs and API surface"
# Stdlib Baseline: Structured Data and Semantics Baseline

---

## Summary

Child issue for #590 Phase 3 — Structured Data / Semantics Baseline.

Fix contract ambiguity and semantic gaps in `std::json`, `std::toml`, `std::text`, and
`std::time` / `std::host::clock` boundary.

---

## Scope

**In scope:**

**JSON:**
- `std::json` parse semantics must be whole-document (not prefix/heuristic)
- Add negative-case fixtures for trailing garbage, malformed input
- Do not imply a DOM or streaming contract that is not implemented

**TOML:**
- Bounded subset must be explicitly documented
- Add fixtures for at-boundary behavior: key types, array of tables edge cases
- Mark partial/experimental explicitly

**Text:**
- `std::text` must clearly distinguish byte-based vs char/Unicode-aware operations
- `len_chars` must document its "best-effort" qualifier explicitly
- `to_lower`, `to_upper`, `trim_*` must document ASCII-only scope

**Time:**
- Split `std::time` (duration math) from `std::host::clock` (actual host time reads)
- Make the boundary explicit and documented
- Do not let `std::time` imply runtime host access it does not have

**Out of scope:**
- Full TOML 1.0 compliance
- Full Unicode normalization
- Timezone support beyond what host can provide
- CSV parser (that is #055, separate issue)

---

## Primary paths

- `std/json/mod.ark`
- `std/toml/mod.ark`
- `std/text/mod.ark`
- `std/time/mod.ark`
- `std/host/clock.ark`
- `tests/fixtures/` (stdlib fixtures for targeted families)

## Allowed adjacent paths

- `std/manifest.toml`
- `docs/stdlib/modules/` (json.md, toml.md, text.md, time.md)

---

## Upstream / Depends on

604 (contract honesty — must not add semantics while names are still misleading)

## Blocks

- #608 (docs/bench closeout)

---

## Acceptance

1. `std::json` parse rejects trailing garbage in a negative fixture
2. `std::toml` bounded subset is prominently documented with negative fixtures at boundaries
3. `std::text` distinguishes byte vs char operations in doc comments
4. `std::time` / `std::host::clock` split is explicit in docs and API surface

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python3 scripts/gen/generate-docs.py
```

---

## Recheck — 2026-05-14

Current implementation status:

- `std::json` documents whole-document parse semantics and has registered
  negative fixtures:
  - `tests/fixtures/stdlib_json/json_parse_trailing_garbage.ark`
  - `tests/fixtures/stdlib_json/json_parse_trailing_object_garbage.ark`
  - `tests/fixtures/stdlib_json/json_parse_multiple_values.ark`
- `std::toml` documents the bounded subset and has registered negative
  fixtures for unsupported table headers, trailing garbage, empty values, and
  unclosed strings.
- `std::text` doc comments and generated docs explicitly state byte / ASCII
  semantics, best-effort `len_chars`, and ASCII-only case/trim behavior.
- `std::time` only exposes duration helpers; `std::host::clock` owns host clock
  reads. The deprecated `time::monotonic_now()` diagnostic fixture is registered.

Verification findings:

- `python3 scripts/manager.py verify fixtures` initially failed before running
  fixtures because `tests/fixtures/lsp_perf/*.ark` is intentionally skipped by
  `scripts/verify/fixtures.py` but was still considered an orphan by the Rust
  fixture harness self-check. The Rust harness was updated to use the same
  `lsp_perf/` skip rule.
- After that harness-contract fix, full fixture execution still failed
  (`PASS: 413 FAIL: 405 SKIP: 20`).
- Targeted #606 failures remain:
  - `target/release/arukellt run tests/fixtures/stdlib_json/json_parse_trailing_garbage.ark`
    traps at runtime instead of printing `error:trailing characters`.
  - `target/release/arukellt run tests/fixtures/stdlib_time/duration.ark`
    produces invalid Wasm (`type mismatch: expected i64, found i32`).

Updated verdict: close-candidate `no`. The docs/API split is mostly present, but
the required negative/time fixtures do not pass yet.

---

## STOP_IF

- Do not implement full TOML 1.0 compliance
- Do not add Unicode normalization or timezone handling
- Do not implement CSV in this issue

---

## Close gate

Close when: all four families have updated docs, negative fixtures pass, and the
time/clock split is explicit and fixture-backed.
