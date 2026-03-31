# Playground: privacy / telemetry / error reporting を実装方針付きで定める

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 438
**Depends on**: 437
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 11

## Summary

playground で何を収集し何を収集しないか、エラー報告をどう扱うかを定義し、設定またはコードレベルの guardrail まで入れる。単なる方針文ではなく、実装に反映される形にする。

## Current state

- playground はこれから作るため、privacy/telemetry が後追いになりやすい。
- share link や examples 利用だけでもログ方針が必要。
- error reporting を入れる場合も opt-in/opt-out を設計する必要がある。

## Acceptance

- [ ] privacy / telemetry 方針が文書化される。
- [ ] 必要なら telemetry の on/off 設定や無効化既定が実装される。
- [ ] error reporting の送信条件がコードまたは config に反映される。
- [ ] docs に利用者向け説明が追加される。

## References

- ``docs/adr/**``
- ``.github/workflows/ci.yml``
- ``docs/index.html``
