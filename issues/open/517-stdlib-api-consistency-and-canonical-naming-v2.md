# Stdlib: canonical naming / module layering / surface consistency の第2監査

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-16 (impl-stdlib #517 slice: env/text inventory + triage)
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

## Metadata / search-index judgement

The current docs generator already knows how to render `stability` and `deprecated_by`
metadata into `docs/stdlib/reference.md` and `docs/stdlib/name-index.md`:
the missing piece is the manifest metadata for the families below, not a new schema.

| Family | Judgement | Repo-grounded evidence | Why this is the judgement |
|--------|-----------|------------------------|---------------------------|
| `std::env` | **yes** | `std/manifest.toml:4851-4876` defines both `var` and `get_var` as `stable` with no `deprecated_by`; `docs/stdlib/reference.md:414-416` and `docs/stdlib/name-index.md:230,574-576,842,1186-1188` render both as canonical stable entries. | The canonical-name policy cannot be expressed in generated docs/search index until one name is marked historical/deprecated in manifest metadata. |
| `std::text` | **yes** | `std/manifest.toml:2079-2086` defines `std::text::concat`, while `std/text/string.ark` still duplicates `concat` and `docs/stdlib/reference.md:857` + `docs/stdlib/name-index.md:750-751` currently list both `prelude` and `std::text` surfaces as canonical stable names. | The duplicate text surface is still rendered as first-class API, so docs/search index need metadata to distinguish canonical vs historical placement. |
| representative collections family: `HashMap_*` prelude surface | **yes** | `std/manifest.toml:1260-1440` shows the old monomorphic `HashMap_*` surface as plain `stable` entries with no `deprecated_by`; `docs/stdlib/reference.md:1018-1034` and `docs/stdlib/name-index.md:240-245,852-865` likewise render them as canonical stable names. | The monomorphic historical naming is still present in the generated docs/search index as active API, so policy-aligned historical/deprecated metadata is needed here too. |

Bottom line: the current generator is sufficient, but these families still need
manifest metadata changes before generated docs/search index can faithfully encode
the canonical-name/deprecation policy.

## Canonical naming policy (restated for #517)

以下は本 issue の整理作業で前提とする **canonical 方針の箇条書き**（最終決定は acceptance 完了時に確定）。

- **単一の公開名**: 同一シグネチャ・同一意味の API は family 内で **1 つの canonical `pub fn` 名** に寄せる。互換のための二重定義は **`@deprecated` 付き alias** か、次メジャーで削除する前提の重複に限定する。
- **Canonical の置き場所**: 環境・文字列の「本体」API は **`std::env` / `std::text`** に置き、prelude は tiny set か、移行期の deprecated re-export に留める（prelude コメントの v3/v4 方針に沿う）。
- **Getter の語彙**: optional / 失敗しうるルックアップは family ごとに **`get_*` 系** か **短い慣用名（例: `var`）** のどちらかに統一し、もう一方は deprecate または削除対象として一覧化する（混在を canonical としない）。
- **補助 API**: `*_or_default` のようなデフォルト付きは、canonical getter と **別名で併存** してよいが、命名パターン（`or_default` vs `unwrap_or` 等）は family 間で揃える。
- **サブモジュール**: `std::text` と `std::text::string` などで **同じ関数を複製** している場合は、どちらを canonical とみなすかを明記し、他方は thin re-export か deprecate とする。

**English summary (same policy):**

- One **canonical** `pub fn` name per identical signature/semantics in a family; duplicates are **deprecated aliases** or explicitly temporary.
- **Home modules** for env/string operations: `std::env`, `std::text`; prelude stays minimal or holds **deprecated** shims only.
- **Pick one** style per family for fallible lookups: either `get_*` **or** a short idiom (`var`); do not treat the mix as final.
- **Companion helpers** like `*_or_default` may coexist with the canonical getter; align suffix patterns across families.
- **Submodules** must not indefinitely duplicate the same API; choose canonical location, then re-export thinly or deprecate the duplicate.

### Triage vocabulary (per-row: exactly one of)

| Value | Meaning |
|-------|---------|
| **keep** | Retain as non-deprecated public API; treat as canonical (or intentional companion helper). |
| **deprecate** | Mark `@deprecated` (or plan removal); callers should migrate to the canonical name/module. |
| **rename** | Planned **symbol rename** (mechanical codemod + docs); not used below until a rename issue lands. |

Inventory rows below use **`path:line`** for the `pub fn` that defines the symbol (first line of the definition).

## Inventory: `std::env` (`std/env/mod.ark`)

| Symbol | Location | Role / notes | Triage |
|--------|----------|----------------|--------|
| `args` | `std/env/mod.ark:8` | Process argv (excluding argv[0]); WASI `args_get` | **keep** |
| `arg_count` | `std/env/mod.ark:13` | Count of args (excluding argv[0]) | **keep** |
| `arg_at` | `std/env/mod.ark:18` | Indexed access via `args()` + `get` | **keep** |
| `var` | `std/env/mod.ark:27` | Env lookup → `Option<String>` (`__intrinsic_env_var`) | **keep** |
| `get_var` | `std/env/mod.ark:32` | Duplicate body of `var`; comment: “work-order naming convention” | **deprecate** |
| `var_or_default` | `std/env/mod.ark:37` | Defaulting wrapper over `__intrinsic_env_var` | **keep** |

**Open decision (env):** `var` と `get_var` は同実装の二重 surface。**一方だけ canonical（keep）、他方は deprecate** とする。上表は仮に **`var` = canonical** とした場合（`get_var` → **deprecate**）。**`get_var` を canonical にする**方針なら、`var` 行の Triage を **deprecate**、`get_var` を **keep** に差し替える — **rename** は不要（既存名のどちらかを残すだけ）。

（モジュール先頭コメントの `vars` は現状 **未実装**；将来 `vars` を足す場合は本表に行を追加し、getter 語彙方針に合わせて命名する。）

## Inventory: `concat` — `std::text` vs prelude（代表 1 件）

同一 intrinsic（`__intrinsic_concat`）を **prelude** と **`std::text`** と **`std::text::string`** が共有しており、名前衝突は **モジュール修飾** で回避されるが、人間向け canonical は 1 箇所に寄せる。

| Symbol | Location | Role / notes | Triage |
|--------|----------|----------------|--------|
| `concat` (prelude) | `std/prelude.ark:47` | `pub fn concat`; 直上行に `@deprecated v3: use text::concat` | **deprecate** |
| `concat` (`std::text`) | `std/text/mod.ark:71` | 「Relocated from prelude」ブロック内の本体 | **keep** |
| `concat` (`std::text::string`) | `std/text/string.ark:19` | `string` サブモジュール側の複製（`mod.ark` と同シグネチャ） | **deprecate** |

**Open decision (text):** `std::text::string::concat` は **deprecate** 後、canonical の `text::concat` への **thin re-export**（1 行ラッパー）に落とすか、呼び出し側を `text::concat` に寄せて削除するかを acceptance で確定。**rename** は不要（シンボル名 `concat` は family 内で既に canonical）。

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
