# Stdlib: bytes / I/O helper の実用 surface を埋める

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 387
**Depends on**: —
**Track**: stdlib-api
**Blocks v1 exit**: no
**Priority**: 5

## Summary

bytes・reader・writer 周辺の小さな補助 API をまとめて整備する。これは設計だけの issue ではなく、`read_all`、`write_all`、`from_utf8`、`to_bytes` 相当の繰り返しパターンを減らし、cookbook と playground examples に載せやすい実用面を作るためのもの。

## Current state

- bytes family は存在するが、実用コードで毎回書く補助ロジックが散在している。
- I/O recipe は `stdio` や `fs` に偏っており、bytes と接続する補助が薄い。
- docs から見てどの helper が推奨か分かりにくい。

## Acceptance

- [ ] bytes / reader / writer の helper API 群が追加または整理される。
- [ ] 少なくとも 3 つ以上の recipe が helper API を利用する。
- [ ] helper API ごとに fixture が追加される。
- [ ] canonical naming が docs に反映される。

## References

- ``std/bytes/**``
- ``std/io/**``
- ``tests/fixtures/``
- ``docs/stdlib/cookbook.md``
