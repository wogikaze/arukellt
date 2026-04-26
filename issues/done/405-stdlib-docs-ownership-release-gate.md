# Stdlib Docs: ownership map と release gate を整備する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 405
**Depends on**: 401, 403
**Track**: stdlib-docs
**Blocks v1 exit**: no
**Priority**: 11

## Summary

stdlib docs を継続的に維持するため、どの page / generator / manifest field を誰が守るかの ownership と、release 前に満たすべき docs quality gate を定義する。

## Current state

- docs は充実しているが、どこが generated でどこが curated かの ownership が薄い。
- release 時に stdlib docs の品質を確認する gate が散在している。
- docs と API 変更が同時に入ると、責務が見えにくい。

## Acceptance

- [x] stdlib docs の ownership map が作成される。
- [x] release gate に stdlib docs 項目が明記される。
- [x] generator / manifest / curated docs の責務境界が書かれる。
- [x] checklist が CI または release docs と接続される。

## References

- ``docs/stdlib/**``
- ``docs/release-checklist.md``
- ``scripts/gen/generate-docs.py``
- ``std/manifest.toml``
