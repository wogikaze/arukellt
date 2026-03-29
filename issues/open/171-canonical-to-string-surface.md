# canonical stringification surface を `to_string(x)` に統一する

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 171
**Depends on**: none
**Track**: language-design
**Blocks v1 exit**: no
**ADR candidate**: yes

## Summary

Arukellt には `i32_to_string` などの primitive helper、`f"..."`、`Display`/method syntax が混在している。
LLM と user-facing docs の主導線を安定させるため、canonical stringification surface を `to_string(x)` に統一する。

## 受け入れ条件

1. ADR で `to_string(x)` を canonical、`.to_string()` を secondary sugar として記録する
2. compiler / emitter / manifest / LSP が `to_string` を public surface として一貫して扱う
3. docs / quickstart / cookbook の主要サンプルが `to_string(x)` を第一表記にする
4. builtin scalar / char / String と Display-based struct の fixture coverage がある
5. issue index / dependency graph が再生成されている

## 実装タスク

1. `docs/adr/` に stringification policy の ADR を追加する
2. `std/manifest.toml` と stdlib metadata を見直し、`to_string` を canonical surface として扱う
3. emitter の generic `to_string` dispatch の穴を埋める
4. quickstart / syntax / cookbook / migration docs の代表例を更新する
5. `tests/fixtures/stdlib_io/to_string.ark` などの coverage を追加する

## 参照

- `docs/adr/ADR-004-trait-strategy.md`
- `docs/adr/ADR-012-stringification-surface.md`
- `std/manifest.toml`
- `crates/ark-parser/src/parser/expr.rs`
