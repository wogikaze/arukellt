---
Status: done
Created: 2026-07-25
Updated: 2026-07-25
Closed: 2026-07-25
ID: 837
Track: compiler-internal
Depends on: 833, 835
Related: ADR-033
Orchestration class: implementation-ready
Orchestration upstream: 835
Blocks v{N}: none
Priority: 2
Source: HashMap_* without use emits unreachable
---

# HashMap_* builtin auto-load reachability

## Summary

`HashMap_i32_fn_*` / `HashMap_i32_String_*` は resolver builtin・loader auto-load・
emit `hash_aliases` まで揃っているが、`use` なしで呼ぶと main が `unreachable` になる。
auto-load された stdlib 本体が MIR reachability に残らず DCE されるのが原因。

## Root cause

Call の callee 文字列は `HashMap_i32_fn_insert` のまま。emit は `hash_aliases` で
`hash_fn::hashmap_i32_fn_set` へ解決できるが、reachability が alias を辿らないため
stdlib 関数が prune され、fallback が `drop*` + `unreachable` になる。

## Fix

- `mir/reachability_walk.ark`: symbol enqueue 失敗時に `hashmap_builtin_alias` を辿る
- legacy walk も同趣旨

## Acceptance

- [x] `hashmap_i32_fn_builtin` validate + hosted run（stdout `10`）
- [x] `hashmap_i32_string_builtin` validate + hosted run（stdout `hi`）
- [x] 既存 `hashmap_i32_fn_call` / `hashmap_i32_fn_mut_get` が緑
- [x] `python3 scripts/manager.py verify lane --gate t3`
