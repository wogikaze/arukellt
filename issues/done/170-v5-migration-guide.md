# v5 Migration guide

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 170
**Depends on**: 165, 166, 169
**Track**: main
**Blocks v1 exit**: no

## Summary

`docs/migration/v4-to-v5.md` に v4→v5 移行ガイドを整理する。特に、デフォルトコンパイラ切り替えの有無、bootstrap 手順の位置づけ、Rust 版と selfhost 版の二重メンテナンス方針を明文化する。

## Acceptance

- [x] v5 で追加された selfhost compiler / bootstrap workflow の説明がある
- [x] デフォルトコンパイラ切り替えの有無が曖昧さなく記述されている
- [x] 開発者向けに Rust 版と selfhost 版の併走方針が記述されている

## References

- `issues/open/165-v5-phase3-wasm-emitter.md`
- `issues/open/166-v5-bootstrap-verification.md`
- `issues/open/169-v5-bootstrap-doc.md`
- `docs/current-state.md`
