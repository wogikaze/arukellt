# Stdlib: prelude 直叩き前提を減らし、安全な wrapper surface を優先する

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-15
**ID**: 513
**Depends on**: none
**Track**: stdlib
**Blocks v1 exit**: no
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

stdlib 自身と docs example が prelude の低レベル helper を直接使う構造は、
ユーザーに「まず prelude を直叩きする」習慣を与えやすい。stdlib を教材としても優秀にするため、
公開 sample と family 実装は module-local wrapper / facade を優先し、
prelude intrinsic 露出を一段隠す方針へ寄せる。

## Repo evidence

- `std/prelude.ark` には `concat`, `i32_to_string`, `parse_i32` など旧来 helper が広く残る
- `std/json/mod.ark`, `std/path/mod.ark`, `std/test/mod.ark` などで prelude helper 直叩きが多い
- 既存 done issue は prelude dedup と surface audit までで、sample quality の観点は未完了

## Acceptance

- [ ] 「user-facing sample / docs / cookbook では prelude 直叩きを避ける」ルールが明文化される
- [ ] stdlib 内部で module facade に置き換えられる prelude call site の棚卸しが作られる
- [ ] `std::text`, `std::core::convert`, `std::io` など wrapper を優先すべき family が列挙される
- [ ] deprecated prelude helper を docs example から段階削除する migration plan がある

## Primary paths

- `std/prelude.ark`
- `std/text/`
- `std/core/convert.ark`
- `docs/stdlib/`
- `docs/cookbook/`

## References

- `issues/done/361-stdlib-prelude-dedup.md`
- `issues/done/394-stdlib-prelude-surface-audit.md`
