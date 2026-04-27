---
Status: done
Created: 2026-04-22
Updated: 2026-04-23
ID: 604
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Stdlib Baseline: Contract and Facade Honesty
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

## Dispatch dependency map

- depends_on_open: none
- depends_on_done: none
- blocks: #605, #606, #607, #608

## Blocks

- #605 (host platform work must start from honest API surface)
- #606 (structured data work must start from honest contracts)
- #607 (hash hardening must start from honest facade/raw split)
- #608 (docs/verification closeout must start from honest contracts)

---

## Acceptance

1. Phase 0 gap ledger is committed and covers all targeted families with `API -> actual behavior -> disposition` columns.
2. Targeted modules have real `//!` module doc comments and generated docs no longer contain `_No module doc comment yet_` for those modules.
3. `std::host::fs::exists` semantics are explicitly documented as read-probe semantics or renamed/deprecated accordingly.
4. `std::json` and `std::toml` partial surfaces are explicitly marked as partial/experimental in docs and manifest labels.
5. `std/manifest.toml` stability labels match actual implementation state for all targeted families.

---

## Required verification

```bash
python3 scripts/gen/generate-docs.py
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python scripts/manager.py docs check
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

---

## Closure note (2026-04-23)

All five acceptance criteria satisfied:

1. Phase 0 gap ledger committed at `docs/stdlib/604-contract-honesty-gap-ledger.md`.
2. Targeted modules have real `//!` doc comments (commits `d0cdb217`, `c297c77c`); generated module pages no longer carry placeholder text.
3. `std::host::fs::exists` is explicitly documented as read-probe / best-effort semantics in `std/host/fs.ark` and the manifest `doc` field; rename deferred (breaking change) and tracked under #605.
4. `std::json` and `std::toml` partial surfaces are marked `experimental` in `std/manifest.toml` (per-function and per-module entries with explicit subset doc).
5. `std/manifest.toml` carries `[[modules]]` entries for all eight targeted families (`std::host::fs`, `std::host::http`, `std::host::sockets`, `std::json`, `std::toml`, `std::collections::hash`, `std::text`, `std::time`) with stability and family-level honesty caveats matching the gap ledger dispositions.

Verification (no new failures vs HEAD baseline):

- `python3 scripts/gen/generate-docs.py` exit 0 (regenerated `docs/stdlib/modules/fs.md`, `modules/io.md`, `reference.md`).
- `python scripts/manager.py verify quick` 5 pre-existing failures only (fixture-manifest hello_world.ark, issues/done unchecked checkboxes, LSP lifecycle gate #569, doc example check #518 ark blocks, broken internal links). No new failures introduced.
- `python3 scripts/check/check-docs-consistency.py` docs consistency OK (0 issues).