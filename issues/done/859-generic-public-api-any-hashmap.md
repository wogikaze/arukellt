---
Status: done
Created: 2026-09-19
Updated: 2026-09-20
ID: 859
Parent: 852
Track: language-v2
Depends on: "856"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 15, 16, 18, 26"
---

# 859 — 公開 API を generic Trait 面へ統一する

## Scope

RFC-011: **15, 16, 18, 26**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] HashMap の canonical public free-function surface を associated/method API へ移行する
- [x] HashMap の monomorphic helper を静的 monomorphization による generic API へ統合する
- [x] public `Any` を source-level に導入せず、内部境界の必要最小限に限定する
- [x] `HashMap<K,V>` の canonical public API（constructor、insert/get/contains/remove/len/is_empty）を完結させる
- [x] 旧 HashMap API の removal condition を記録する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## Evidence

- Implementation commits: `fd50e0f45` (`Implement generic HashMap public API`),
  `2bfeeede3` (`fix: restore generic hashmap numeric type checking`), and
  `10fdbe597` (`fix: keep GC hashmap indices memory-width aware`). The
  follow-up corrected the `insert(key, value)` arity check and restored the
  explicit `i64` literals in the i64 `Hash` implementation that strict numeric
  unification exposed during migration.
- Normative decisions: [ADR-066](../../docs/adr/ADR-066-language-v2-generic-public-api.md)
  fixes `HashMap::new<K,V>()` and receiver methods as the canonical source surface,
  six supported concrete pairs, static adapter dispatch, the internal-only `Any`
  boundary, and the five conditions for removing the compatibility API. The
  synchronized sources are `docs/language/spec.md`, RFC-011, and
  `docs/stdlib/migration-guidance.md`.
- Public surface: `std/collections/hash_map.ark` implements
  `new`, `insert`, `get`, `contains_key`, `remove`, `len`, and `is_empty`.
  The compiler typechecker/lowerer records generic calls and selects typed
  adapters for `i32`/`String` keys and `i32`/`String`/`fn(i32) -> i32` values.
  The old type-suffixed functions remain only as the ADR-014 compatibility
  implementation boundary; the issue does not claim the separate #718
  migration of every stdlib collection family.
- `Any` boundary: no user source fixture or public type declaration uses
  `Any`; the only remaining occurrence is the internal/pseudo manifest boundary
  documented by ADR-066.
- Positive fixture: `tests/fixtures/stdlib_hashmap/hashmap_methods_generic.ark`
  covers insert replacement, lookup, duplicate replacement, contains/remove,
  length/empty state, and both `i32` and `String` key families.
- Negative/diagnostic fixture:
  `tests/fixtures/diagnostics/hashmap_unsupported_pair.ark` rejects
  `HashMap::new<bool, bool>()` with the supported-pair diagnostic recorded in
  its `.diag` sidecar.
- Target parity: the current-source bootstrap shim was rebuilt from the
  worktree (`compilation succeeded`, registry/core-op audits reported
  `mismatched=0`, `unresolved=0`). The positive fixture compiled and validated
  for both `wasm32-gc` (GC/function-references/Memory64) and `wasm32`
  (linear-memory/Memory64), and the fallback artifact ran successfully. The
  positive fixture also packages, validates, and runs as the default GC P2
  component after the memory32 index fix. The negative fixture returned rc=1
  with the expected unsupported-pair diagnostic.
- Migration regression: `tests/fixtures/stdlib_core/hash_i64.ark` now checks
  successfully after the i64 `Hash` implementation was made explicit; the
  generic HashMap fixture no longer fails with the former i32/i64 mismatch
  cascade.
- Quality and docs: targeted Ark `fmt --check` passed (`checked=2`), the
  post-fix touched-code quality ratchet passed, `verify lane` passed, and the
  docs generator reported generated docs are up to date. Final `docs check`
  passed freshness, deprecated-API, consistency, and doc-example checks
  (`4/4`).
- `python3 scripts/manager.py selfhost build-compiler` now passes by applying
  the guarded source-only Box/Slice overlay when the pinned stage-0 compiler
  reaches the #854 generic-enum mismatch. This is a successful stage-2 build,
  not yet a refreshed pin/fixpoint receipt.

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する
