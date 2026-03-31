# Stdlib Docs: host module の target/capability 注記を統一する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 364
**Depends on**: —
**Track**: stdlib-docs
**Blocks v1 exit**: no
**Priority**: 12

## Summary

host family の各 API に対して、target 互換性 (wasi-p1 / wasi-p2 / freestanding)、必要な capability、stub 状態を統一フォーマットで注記する。現在 `host_stub` は manifest に表記があるが、reference docs や module pages での視覚的警告が不十分で、利用者が「使えると思って呼んだら stub だった」状態になりうる。

## Current state

- `std/manifest.toml`: target フィールドで wasi-p2 限定を表記
- `docs/stdlib/reference.md`: stability 表示はあるが capability 制約が目立たない
- host module の module page に target/capability の統一注記なし
- `host_stub` API に「今使うべきではない」レベルの強い product guidance なし

## Acceptance

- [ ] host family の全 API に target 互換性注記が docs に存在する
- [ ] `host_stub` API に明確な「未実装」警告バナーが表示される
- [ ] capability 注記フォーマットが統一される (target, required capability, status)
- [ ] `scripts/generate-docs.py` が manifest の target/kind から注記を自動生成する

## References

- `std/manifest.toml` — target / kind フィールド
- `docs/stdlib/modules/*.md` — module pages
- `docs/stdlib/reference.md` — reference
- `scripts/generate-docs.py` — docs 生成
