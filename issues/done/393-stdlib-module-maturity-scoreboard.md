# Stdlib: module family ごとの maturity scoreboard を生成する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-01
**ID**: 393
**Depends on**: 383
**Track**: stdlib-api

## Acceptance

- [x] family scoreboard が自動生成される。
- [x] API 数・fixture 数・recipe 数・stability 分布が出る。
- [x] docs または current-state から参照できる。
- [x] 不足 coverage を示す項目がある。

## Resolution

- Created `scripts/generate-scoreboard.sh` that generates `docs/stdlib/scoreboard.md`
- Shows per-family: API count, stable/experimental/deprecated breakdown, fixture coverage %, host dependency
- Total: 274 APIs, 221 stable, 50 experimental, 3 deprecated, 214/271 (78%) fixture coverage
- Low coverage families highlighted: std::wasm (26%), std::wit (35%), std::host::random (33%)
