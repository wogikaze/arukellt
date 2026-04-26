# verify-bootstrap.sh の scaffold を解除する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-08
**ID**: 324
**Depends on**: 323
**Track**: selfhost-verification
**Blocks v1 exit**: no
**Priority**: 16

## Summary

verify-bootstrap.sh の Stage 1 / Stage 2 を実際に実行する。現在は unified binary がないため skip されている。#323 で統一バイナリが得られた後、scaffold ラベルを外し、Stage 1 (s1 で selfhost を再コンパイル) と Stage 2 (sha256 fixpoint 検証) を実行する。

## Current state

- `scripts/run/verify-bootstrap.sh:127`: ヘッダが `"Bootstrap verification — fixpoint scaffold"` と自称
- Stage 1/2 は `main.wasm` 不在で conditional skip
- machine-readable summary で `fixture-parity` / `cli-parity` / `diagnostic-parity` / `determinism` が `not-verified` 固定
- `--stage1-only` フラグで CI は Stage 0 のみ実行

## Acceptance

- [x] verify-bootstrap.sh が Stage 1 を実行し `arukellt-s2.wasm` を生成する
- [x] Stage 2 で `sha256(s1) == sha256(s2)` fixpoint を検証する
- [x] machine-readable summary の status が `not-verified` 以外の値を返す
- [x] scaffold ラベルが除去される
- [x] `--stage1-only` 以外の mode で Stage 1/2 が実行可能になる

## References

- `scripts/run/verify-bootstrap.sh` — bootstrap verification script
- `docs/compiler/bootstrap.md` — completion criteria
