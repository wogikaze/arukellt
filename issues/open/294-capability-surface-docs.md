# capability surface の公式リストを文書化する

**Status**: open
**Created**: 2026-03-31
**ID**: 294
**Depends on**: 291, 292, 293
**Track**: main
**Priority**: 14

## Summary

「どの環境で何が使えて、何が拒否されるか」の一覧が docs にない。host API の capability surface を明示的に文書化する。

## Current state

- `docs/current-state.md` §Known Limitations に断片的な記述あり
- `std/manifest.toml` に `kind`, `target`, `stability` のメタデータあり
- 統合的な capability surface document はない

## Acceptance

- [ ] docs に capability surface 一覧ページが存在する（使用可能 / stub / 未実装 / deny 可能の分類）
- [ ] 各 host module の状態が `std/manifest.toml` と一致する
- [ ] `--deny-clock`, `--deny-random`, `--deny-fs`, `--dir` の効果が記載される
- [ ] `docs/current-state.md` §Known Limitations がこの文書を参照する

## References

- `std/manifest.toml`
- `docs/current-state.md`
- `std/host/*.ark`
