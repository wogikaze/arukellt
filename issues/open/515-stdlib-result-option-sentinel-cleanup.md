# Stdlib: sentinel 値 / raw String error を Result / Option / Error enum に寄せる

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-15
**ID**: 515
**Depends on**: none
**Track**: stdlib
**Blocks v1 exit**: no
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

stdlib には `0 - 1` を not-found / invalid sentinel として返す古い API や、
`Result<_, String>` に過度に寄った surface がまだ残る。done issue #392 の naming/convention 整理を
実装 rollout まで進め、`Option` / `Result<_, Error>` / typed enum を優先する surface へ寄せる。

## Repo evidence

- `std/bytes/mod.ark`, `std/text/mod.ark`, `std/path/mod.ark`, `std/seq/mod.ark` に `0 - 1` sentinel が残る
- host / io family には `Result<_, String>` が多い
- `std/core/error.ark` はあるが family 横断で十分使われていない

## Acceptance

- [ ] sentinel 値を返す公開 API の一覧が作成される
- [ ] `Option` / `Result` / typed enum へ移行できる候補と、互換維持のため adapter が要る候補が分類される
- [ ] `String` error を `std::core::Error` へ寄せる migration 方針が family ごとに整理される
- [ ] 新規 stdlib API は sentinel `i32` を返さないルールが明文化される

## Primary paths

- `std/bytes/mod.ark`
- `std/text/mod.ark`
- `std/path/mod.ark`
- `std/seq/mod.ark`
- `std/core/error.ark`

## References

- `issues/done/392-stdlib-error-result-conventions.md`
