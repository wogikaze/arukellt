# Stdlib Baseline: Contract and Facade Honesty

**Status**: open
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 604
**Parent**: #590
**Depends on**: —
**Track**: stdlib
**Orchestration class**: implementation-ready

---

## Summary

Child issue for #590 Phase 1 — Contract / Facade Honesty (CRITICAL).

No targeted stable- or provisional-looking API should promise a stronger contract than
the implementation actually provides. This issue applies the raw/facade/adapter policy
to the problem families and aligns docs + manifest stability with truth.

**Phase 0 baseline is part of this issue:** write the gap ledger (current claim vs actual
behavior) for all targeted families before making any API changes.

Targeted families: `std::host::fs`, `std::json`, `std::toml`, `std::collections::hash`,
`std::host::http`, `std::host::sockets`, `std::text`, `std::time`.

---

## Scope

**In scope:**
- Write Phase 0 gap ledger: record exact "API name vs actual behavior" mismatches
- Apply `docs/stdlib/516-raw-facade-boundary-policy.md` to targeted modules
- Rename, deprecate, or demote raw helpers that overstate their semantics
- Add real `//!` module doc comments to targeted modules
- Regenerate docs so targeted modules no longer show `_No module doc comment yet_`
- Update `std/manifest.toml` stability labels to match actual implementation state
- For `std::host::fs::exists`: decide and implement: deprecate or rename to reflect
  read-probe semantics
- For `std::json`, `std::toml`: mark partial/experimental surfaces explicitly
- For `std::collections::hash`: separate raw layout helpers from user-facing facade

**Out of scope:**
- Adding new filesystem capabilities (that is #605)
- Structured data parser improvements (that is #606)
- True hash correctness fixes (that is #607)
- Docs regeneration gate work (that is #608)

---

## Primary paths

- `std/host/fs.ark`
- `std/json/mod.ark`
- `std/toml/mod.ark`
- `std/collections/hash.ark`
- `std/host/http.ark`
- `std/host/sockets.ark`
- `std/text/mod.ark`
- `std/time/mod.ark`
- `std/manifest.toml`
- `docs/stdlib/modules/` (targeted module doc files)

## Allowed adjacent paths

- `docs/stdlib/516-raw-facade-boundary-policy.md` (reference)

---

## Upstream / Depends on

None.

## Blocks

- #605 (host platform work must start from honest API surface)
- #606 (structured data work must start from honest contracts)
- #607 (hash hardening must start from honest facade/raw split)

---

## Acceptance

1. Phase 0 gap ledger is written and committed (one table: API name → actual behavior)
2. Targeted modules have real `//!` module doc comments (no more `_No module doc comment yet_`)
3. `std::host::fs::exists` behavior is explicitly documented or renamed to reflect read-probe semantics
4. `std/manifest.toml` stability labels match actual implementation for targeted families

---

## Required verification

```bash
python3 scripts/gen/generate-docs.py
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
```

---

## STOP_IF

- Do not add new capabilities in this issue — only fix honesty
- Do not implement streaming I/O, directory listing, or metadata in this issue
- Do not touch generics-backed collections before compiler support is ready (#044, #312, #512)

---

## Close gate

Close when: gap ledger exists, targeted module docs are real (no placeholder text),
manifest stability labels are accurate, and `generate-docs.py` runs cleanly.
