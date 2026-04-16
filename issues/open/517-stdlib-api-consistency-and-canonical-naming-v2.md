# Stdlib: canonical naming / module layering / surface consistency の第2監査

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-16
**ID**: 517
**Depends on**: none
**Track**: stdlib
**Blocks v1 exit**: no
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

過去に monomorphic deprecation や naming 整理は進んだが、family 間の命名・返り値規約・module layering はまだ揺れている。
`hashmap_*`, `json_*`, `toml_*`, `env::var/get_var`, `text::concat` vs prelude `concat` などの不一致を再監査し、
canonical naming と alias/deprecation 計画を更新する。

## Repo evidence

- `std/env/mod.ark` に `var` と `get_var` の二重 surface がある
- `std/text` family と prelude の `concat` / formatting helpers が並立している
- collections family に monomorphic historical naming が残る

## Canonical naming policy (restated for #517)

以下は本 issue の整理作業で前提とする **canonical 方針の箇条書き**（最終決定は acceptance 完了時に確定）。

- **単一の公開名**: 同一シグネチャ・同一意味の API は family 内で **1 つの canonical `pub fn` 名** に寄せる。互換のための二重定義は **`@deprecated` 付き alias** か、次メジャーで削除する前提の重複に限定する。
- **Canonical の置き場所**: 環境・文字列の「本体」API は **`std::env` / `std::text`** に置き、prelude は tiny set か、移行期の deprecated re-export に留める（prelude コメントの v3/v4 方針に沿う）。
- **Getter の語彙**: optional / 失敗しうるルックアップは family ごとに **`get_*` 系** か **短い慣用名（例: `var`）** のどちらかに統一し、もう一方は deprecate または削除対象として一覧化する（混在を canonical としない）。
- **補助 API**: `*_or_default` のようなデフォルト付きは、canonical getter と **別名で併存** してよいが、命名パターン（`or_default` vs `unwrap_or` 等）は family 間で揃える。
- **サブモジュール**: `std::text` と `std::text::string` などで **同じ関数を複製** している場合は、どちらを canonical とみなすかを明記し、他方は thin re-export か deprecate とする。

## Inventory: `std::env` (`std/env/mod.ark`)

| Symbol | Location | Role / notes | Triage |
|--------|----------|----------------|--------|
| `args` | `std/env/mod.ark:8` | Process argv (no argv[0]) | **keep** |
| `arg_count` | `std/env/mod.ark:13` | Argument count | **keep** |
| `arg_at` | `std/env/mod.ark:18` | Indexed arg access | **keep** |
| `var` | `std/env/mod.ark:27` | Env lookup → `Option<String>` | **keep**（短名・ドキュメント上の primary とする案） |
| `get_var` | `std/env/mod.ark:32` | `var` と同一実装の別名（コメント: work-order naming） | **deprecate**（canonical を `var` に固定する案。`get_var` を canonical にする場合は `var` 行を **deprecate** に差し替え） |
| `var_or_default` | `std/env/mod.ark:37` | デフォルト付き env lookup | **keep** |

`get_var` 行の triage は上表のとおり **どちらか一方を deprecate** に寄せる必要がある。現状コードは `var` を本文、`get_var` を「work-order naming」向け alias としている。

## Inventory: `concat` — `std::text` vs prelude（代表 1 件）

| Symbol | Location | Role / notes | Triage |
|--------|----------|----------------|--------|
| `concat` (prelude) | `std/prelude.ark:46`–`48` | `@deprecated v3` — `text::concat` へ誘導済み | **deprecate**（維持、v4 削除予定の legacy） |
| `concat` (`std::text`) | `std/text/mod.ark:71`–`72` | 「Relocated from prelude」節の canonical 相当 | **keep** |
| `concat` (`std::text::string`) | `std/text/string.ark:19`–`20` | `text::string` 向けに `mod.ark` と同一シグネチャを複製 | **deprecate**（長期的には `text::concat` に集約し、`string::` 側は thin ラッパーまたは削除。現状は重複 surface） |

## Acceptance

- [ ] family ごとの canonical function naming policy が再定義される
- [ ] alias と historical names の整理対象が一覧化される
- [ ] rename / deprecation / keep-as-is の 3 分類が各 API に付与される
- [ ] generated docs と search index に必要な metadata 拡張の要否が判断される

## Primary paths

- `std/env/mod.ark`
- `std/text/`
- `std/prelude.ark`
- `std/collections/`
- `std/manifest.toml`

## References

- `issues/done/359-stdlib-monomorphic-deprecation.md`
- `issues/done/399-stdlib-docs-canonical-name-search-index.md`
