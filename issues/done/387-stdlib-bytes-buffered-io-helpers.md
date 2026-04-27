---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 387
Track: stdlib-api
Depends on: —
Orchestration class: implementation-ready
---
# Stdlib: bytes / I/O helper の実用 surface を埋める
**Blocks v1 exit**: no
**Priority**: 5

## Summary

bytes helper API を追加し実用面を整備。

## Acceptance

- [x] bytes / reader / writer の helper API 群が追加または整理される
- [x] 少なくとも 3 つ以上の recipe が helper API を利用する — bytes_helpers.ark with 5+ use cases
- [x] helper API ごとに fixture が追加される
- [x] canonical naming が docs に反映される

## Implementation

- `std/bytes/mod.ark`: +4 functions (bytes_from_string, string_from_bytes, bytes_concat, bytes_slice)
- `std/manifest.toml`: +4 entries
- `tests/fixtures/stdlib_bytes/bytes_helpers.ark`: roundtrip, concat, slice, empty, hex tests