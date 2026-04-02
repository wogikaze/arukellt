# Language Docs: guide / reference の例コードを fixture 連動にする

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 410
**Depends on**: 406
**Track**: language-docs
**Blocks v1 exit**: no
**Priority**: 5

## Summary

言語 docs の例コードも stdlib docs と同様に source-backed 化する。構文、型、エラー例を fixture または compile-check 対象に紐づけ、仕様説明が壊れないようにする。

## Current state

- 言語 docs の例コードは説明に有用だが、実装変更で古くなるリスクがある。
- spec 中の例、guide 中の例、tests/fixtures の例が別管理になりやすい。
- 例コードの出典が追いにくい。

## Acceptance

- [x] language docs の例コードに source-of-truth が付与される。
- [x] 少なくとも guide の主要章の例は compile-check される。
- [x] 例コードと fixture の対応表が作られる。
- [x] CI が壊れた例コードを検出する。

## References

- ``docs/language/**``
- ``tests/fixtures/``
- ``docs/examples/**``
