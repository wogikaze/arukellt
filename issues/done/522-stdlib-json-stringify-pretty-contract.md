---
Status: done
Created: 2026-04-18
Updated: 2026-04-18
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Stdlib JSON: `stringify_pretty` の product claim を現実に合わせる
**Closed**: 2026-04-18
**ID**: 522
**Depends on**: none
**Track**: stdlib
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no
**Source**: false-done audit from `docs/stdlib/modernization/514-parser-host-quality-audit.md`

## Summary

`std::json::stringify_pretty(v, indent)` は名前上 pretty-print semantics を約束しているが、
current docs は「basic pass-through; full indentation deferred」と記している。
この mismatch は user-visible claim の先行であり、implementation か naming/docs contract のどちらかを揃える必要がある。

### Progress (audit 2026-04-18, #522)

The mismatch described above is **obsolete**: `std/json/mod.ark` already implements newlines and per-level space indentation for arrays and objects (scalars pass through stored `raw`). Evidence:

- Fixture: `tests/fixtures/stdlib_json/json_pretty.ark` / `json_pretty.expected` (registered in `tests/fixtures/manifest.txt`) exercises nested object/array output with `indent = 2`.
- API docs: generated `docs/stdlib/modules/json.md` and `docs/stdlib/reference.md` describe the same semantics (from `std/json/mod.ark` doc comments + manifest).
- Stale prose remains in `docs/stdlib/modernization/514-parser-host-quality-audit.md` and related matrices; track separately if those audit pages should be refreshed.

## Why this must exist

- `docs/stdlib/modules/json.md`（generated）は現行 `stringify_pretty` 実装と一致した記述になっているが、modernization audit 系ドキュメントに旧「pass-through」記述が残る
- `docs/stdlib/modernization/514-parser-host-quality-audit.md` が focused follow-up を要求している
- 現行 open queue に `stringify_pretty` だけを close gate 付きで扱う issue がない

## Evidence source

- `docs/stdlib/modules/json.md`
- `docs/stdlib/modernization/514-parser-host-quality-audit.md`
- `std/json/mod.ark`

## Primary paths

- `std/json/mod.ark`
- `docs/stdlib/modules/json.md`
- `docs/stdlib/reference.md`
- `tests/fixtures/stdlib_json/`
- `tests/fixtures/manifest.txt`

## Non-goals

- full JSON parser redesign
- object/array internal representation redesign
- cross-format pretty printers for TOML / CSV

## Acceptance

- [x] repo chooses one contract explicitly: **pretty-print semantics are implemented** (not a pass-through alias).
- [x] either `stringify_pretty` implements indentation/newline semantics with fixture proof — see `tests/fixtures/stdlib_json/json_pretty.*`
- [x] or docs/reference/name text are downgraded so no pretty-print claim remains ahead of reality — N/A; implementation matches the name.
- [x] chosen contract is reflected consistently in module docs, reference docs, and fixture evidence — `docs/stdlib/modules/json.md` / `docs/stdlib/reference.md` + `json_pretty` fixture.

## Required verification

- focused JSON fixture run proving the chosen outcome
- `python3 scripts/check/check-docs-consistency.py`
- if implementation changes: `bash scripts/run/verify-harness.sh --fixtures`

## Close gate

- no docs / reference page claims indentation semantics without repo proof
- callable surface and docs text say the same thing
- evidence file(s) for either implementation or relabel outcome are checked in

---

## Close note — 2026-04-18

Closed as complete. The mismatch described in the summary is obsolete; `stringify_pretty` already implements pretty-print semantics matching its name.

**Close evidence:**
- Progress section confirms the mismatch is obsolete
- `std/json/mod.ark` implements newlines and per-level space indentation for arrays and objects
- Fixture `tests/fixtures/stdlib_json/json_pretty.ark` / `json_pretty.expected` exercises nested object/array output with indent=2
- API docs in `docs/stdlib/modules/json.md` and `docs/stdlib/reference.md` describe the same semantics
- All 4 acceptance criteria checked

**Acceptance mapping:**
- ✓ Contract chosen: pretty-print semantics implemented (not pass-through)
- ✓ `stringify_pretty` implements indentation/newline semantics with fixture proof
- ✓ Docs/reference/name text consistent (implementation matches name, no downgrade needed)
- ✓ Contract reflected consistently in module docs, reference docs, and fixture evidence

**Implementation notes:**
- This issue was created to address a docs/implementation mismatch that no longer exists
- The implementation already matches the product claim in the function name
- Stale prose remains in modernization audit docs; that cleanup is tracked separately if needed