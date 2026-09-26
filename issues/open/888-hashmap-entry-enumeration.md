---
Status: open
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
