# Stdlib Docs: legacy ページの整理と redirect / archive 方針を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 404
**Depends on**: 402
**Track**: stdlib-docs
**Blocks v1 exit**: no
**Priority**: 10

## Summary

stdlib docs に残る historical / legacy landing pages の扱いを整理し、current docs との重複を減らす。削除ではなく redirect、archive banner、統合のいずれかで処理する。

## Current state

- `core.md` や `io.md` のような legacy ページと現行 module pages の役割が重なる箇所がある。
- historical 情報として有用でも、current 入口としてはノイズになる。
- redirect / archive の実装方針が page ごとに決まっていない。

## Acceptance

- [ ] legacy stdlib docs の inventory が作成される。
- [ ] 各ページに keep / merge / redirect / archive の処置が決まる。
- [ ] 必要な redirect または banner が実装される。
- [ ] README からの導線が current-first になる。

## References

- ``docs/stdlib/**``
- ``docs/README.md``
- ``scripts/generate-docs.py``
