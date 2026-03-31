# Repo Hygiene: ディレクトリごとの ownership / maintenance map を作る

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 425
**Depends on**: —
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 9

## Summary

repo が広いため、どのディレクトリが product surface か、どこが generated か、どこが archive か、どの script が所有するかを一覧化する。cleanup と onboarding の両方に効く。

## Current state

- workspace が大きく、ディレクトリの意味が暗黙知になりやすい。
- docs / tests / tools / generated artifact の所有関係が明示されていない。
- cleanup 対象の判断にも ownership 情報が必要。

## Acceptance

- [ ] directory ownership map が追加される。
- [ ] 主要ディレクトリに役割と保守責任が記載される。
- [ ] generated / archive / product / internal の区分が含まれる。
- [ ] README または docs から辿れる。

## References

- ``README.md``
- ``docs/README.md``
- ``crates/**``
- ``scripts/**``
- ``tests/**``
