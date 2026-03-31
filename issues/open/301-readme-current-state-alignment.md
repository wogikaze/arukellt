# current-state.md と README.md の整合を取る

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 301
**Depends on**: 303
**Track**: docs/ops
**Blocks v1 exit**: no
**Priority**: 21

## Summary

README.md と current-state.md の間に fixture count 等の数値ズレがある。project-state.toml を source of truth として両方を再生成し、一致させる。

## Current state

- README.md: 588 entries と記載
- current-state.md: 586 entries と記載
- `docs/data/project-state.toml:81`: `fixture_manifest_count = 586`
- `python3 scripts/generate-docs.py` で再生成すれば解消するはず

## Acceptance

- [ ] `python3 scripts/generate-docs.py` で再生成し、README / current-state の数値が一致
- [ ] project-state.toml の値が実態 (manifest.txt の行数) と一致
- [ ] `scripts/check-docs-consistency.py` が pass

## References

- `README.md`
- `docs/current-state.md`
- `docs/data/project-state.toml`
- `scripts/generate-docs.py`
