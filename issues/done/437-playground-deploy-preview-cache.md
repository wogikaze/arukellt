# Playground: deployment / preview environment / asset cache 戦略を整える

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 437
**Depends on**: 431
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 10

## Summary

playground の frontend と Wasm assets をどこに配置し、preview 環境と cache busting をどう扱うかを決めて実装する。これも design-only ではなく、実際の deploy 手順と versioned assets を持たせる。

## Current state

- frontend package も deploy pipeline も存在しない。
- Wasm asset はキャッシュされやすく、更新反映が難しい。
- preview を持たないと docs との統合検証がしにくい。

## Acceptance

- [x] deploy 手順または workflow が追加される。
- [x] preview 環境または preview 手順が定義される。
- [x] asset versioning / cache busting が実装される。
- [x] 最低限の smoke test がある。

## References

- ``docs/index.html``
- ``.github/workflows/ci.yml``
- ``docs/README.md``
