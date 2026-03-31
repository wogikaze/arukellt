# Stdlib: JSON / TOML / CSV の round-trip baseline を整備する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 389
**Depends on**: —
**Track**: stdlib-api
**Blocks v1 exit**: no
**Priority**: 7

## Summary

serialization family は format ごとの差異が大きいため、docs だけでなく baseline データを用意して round-trip 期待値を固定する。parse → encode、encode → parse、エラー時の扱いを format ごとに比較可能にする。

## Current state

- JSON / TOML / CSV family は存在するが、round-trip の安定性を示す共通 baseline が無い。
- error message や lossy conversion の扱いが family ごとにばらつきやすい。
- docs 上の examples が実際の round-trip 契約と結びついていない。

## Acceptance

- [ ] JSON / TOML / CSV それぞれに round-trip baseline fixture が追加される。
- [ ] 正常系と異常系の両方が baseline 化される。
- [ ] family ごとの encode / decode 契約の差分が docs に明記される。
- [ ] baseline 更新手順が記録される。

## References

- ``std/json/**``
- ``std/toml/**``
- ``std/csv/**``
- ``tests/fixtures/``
- ``tests/baselines/``
