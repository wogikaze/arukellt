# capability surface の公式リストを文書化する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-01
**ID**: 294
**Depends on**: 291, 292, 293, 307
**Track**: capability

## Acceptance

- [x] docs に capability surface 一覧ページが存在する（使用可能 / stub / 未実装 / deny 可能の分類）
- [x] 各 host module の状態が `std/manifest.toml` と一致する
- [x] `--deny-clock`, `--deny-random`, `--deny-fs`, `--dir` の効果が記載される
- [x] `docs/current-state.md` §Known Limitations がこの文書を参照する

## Resolution

- Created `docs/capability-surface.md` (254 lines) covering all 8 host modules, 20 functions
- Includes target compatibility matrix (T1 vs T3), CLI capability flags, host stub enforcement
- Per-module function tables with signatures, status, WASI import details
- Known limitations section documents 7 current gaps
- All dependencies (#291, #292, #293, #307) already done
