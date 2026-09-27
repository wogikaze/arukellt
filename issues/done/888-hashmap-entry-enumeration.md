---
Status: done
Created: 2026-09-26
Updated: 2026-09-26
ID: 888
Parent: 852
Track: language-v2
Depends on: "860"
Related: "ADR-058, RFC-011, #852, #860, #872"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 2
Source: "#860 RFC-011 item 37 residual — HashMap entry enumeration"
---

# 888 — HashMap entry (key+value) enumeration

## Scope

RFC-011 item 37 residual transferred from #860: `for key in map` iterates
every supported pair through the parametric
`impl<K,V> IntoIterator for HashMap<K,V>` (keys snapshot into
`VecIter<K>`), but entry-level enumeration — `map.entries()`,
`map.values()`, or `for (k, v) in map` — does not exist.

Today only the raw `Vec<i32>` i32→i32 map has `hashmap_values`
(`std/collections/hash.ark`); the typed `HashMap<K,V>` surface has no
per-family values adapter. Options, to be fixed by the implementing
change per ADR-014/046 API policy:

- source-level `entries()` returning a `ZipIter`/pair-producing
  adapter over keys+values snapshots (no new intrinsic), or
- a map-specific enumeration intrinsic if snapshot semantics are
  insufficient.

Live-order guarantees (insertion order, mutation-during-iteration
behavior) must be specified explicitly or declared out of scope.

## Acceptance

- [ ] Entry enumeration API surface を trait/method 形で固定する
- [ ] 全 supported pair で entry iteration が `wasm32-gc` と `wasm32`
      の両 backend で compile/validate/run する
- [ ] live-order / mutation-during-iteration の semantics を docs 化する
- [ ] positive fixture + negative/diagnostic fixture

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する

## Progress receipt — 2026-09-27 (issue closing)

Commit: `f6abca553` on `docs/adr-058-language-v2-semantics`.

API surface fixed as method-call form, mirroring `keys()`:
`m.values() -> Vec<V>` and `m.entries() -> Vec<(K, V)>` — eager
snapshots materializing the pairs present at the call. Bucket order is
implementation-defined and documented as such in
`docs/language/spec.md` §9.14.

Evidence:

- Implementation commit `f6abca553`. Enabling work: `Vec<(A,B)>`
  annotation→name paths now keep the `tuple:<k,...>` element spelling
  (shared `type_ann_tuple_local_name` across local/param/return/layout
  paths); method calls record arg0 like direct calls so GC emit
  resolves receivers on non-contiguous arg staging; callee-return
  inference maps `vec:fnref`/`vec:enum-like` elements to the dedicated
  fnpair/enum-ref array layouts instead of the i32-vec default;
  `hash::hashmap_i32_i32_entries` renamed to `hashmap_entry_pairs` to
  end the LM name collision with the `hash_map` wrapper.
- Positive fixture `tests/fixtures/collections/hashmap_entries_values.ark`
  covers `values()` + `entries()` for all six supported pairs plus
  eager-snapshot semantics; registered `run:` + `t3-run:`. Verified:
  wasm32 run and wasm32-gc component both print
  `30 330 7 37 5 8 3 6 23 6 23 6 1 1 2` matching `.expected`.
- Negative fixture `tests/fixtures/diagnostics/hashmap_entries_unsupported_pair.ark`
  (`diag:`) records `error[E0200|typecheck]: unsupported HashMap type
  pair` for `HashMap<i32, Vec<i32>>`.
- Normative spec update: `docs/language/spec.md` §9.14 lists
  `keys()`/`values()`/`entries()` and documents snapshot semantics and
  implementation-defined element order.
- `python3 scripts/manager.py verify lane`: PASS (comment policy,
  semantic-debt, lint smoke, touched-code ratchet, fmt, lint ratchet,
  quality contract).

Follow-up filed: #890 — `Option<tuple>` payloads (e.g.
`entries().get(i)` / `.pop()`) produce invalid wasm on both backends.
Pre-existing ABI gap reproduced without HashMap
(`fn mk() -> Option<(i32,i32)>`); the snapshot API itself does not
require Option<tuple>.

Scope note: `for (k, v) in map` desugaring was one listed option; the
accepted API surface is the values()/entries() method call pair. A
dedicated entry-for syntax may be proposed separately if wanted.
