# Stdlib Docs: family overview ページを実装し learning path を作る

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 396
**Depends on**: 363
**Track**: stdlib-docs
**Blocks v1 exit**: no
**Priority**: 2

## Summary

generated な reference の上に、family ごとの curated overview を実際に用意する。overview は用途、推奨 API、避けるべき historical API、target 制約、関連 recipe をまとめるページとして機能させる。

## Current state

- module pages はあるが、family 単位で「何から読めばいいか」が弱い。
- legacy landing page と新しい module page の役割が重なっている。
- `docs/stdlib/README.md` は index としては強いが導線としてはやや薄い。

## Acceptance

- [ ] 主要 family に curated overview が追加される。
- [ ] overview から reference / recipe / migration へ横断リンクが張られる。
- [ ] legacy landing page の重複が整理される。
- [ ] README からの導線が更新される。

## References

- ``docs/stdlib/README.md``
- ``docs/stdlib/modules/*.md``
- ``docs/stdlib/reference.md``
