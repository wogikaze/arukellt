
## Reopened by audit

- **Date**: 2026-04-21
- **Reason**: Extension points to GitHub Pages playground URL but endpoint serves non-functional playground (no wasm)
- **Root cause**: The playground wasm binary (ark-playground-wasm) has never been compiled. crates/ark-playground-wasm/pkg/ does not exist. docs/playground/wasm/ is empty. All playground user-visible functionality depends on this binary.
- **Evidence**: `find . -name '*.wasm' -path '*playground*'` returns nothing; `ls crates/ark-playground-wasm/pkg/` fails; `ls docs/playground/wasm/` is empty.

# Extension: playground surface は repo で証明できる endpoint だけを指す

**Status**: open
**Created**: 2026-04-03
**Updated**: 2026-04-21
**ID**: 469
**Depends on**: 378
**Track**: extension
**Orchestration class**: verification-ready
**Orchestration upstream**: —
**Blocks v4 exit**: no
**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.

---

## Reopened by audit — 2026-04-13

**Reason**: Conflicting metadata.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Closed by orchestration — 2026-04-14

**Reason**: Close gate satisfied with repo-native regression proof and surface consistency.

**Evidence**:
- `extensions/arukellt-all-in-one/test/playground-endpoint-guard.js` enforces that extension-exposed endpoint equals repo-proved route and requires `docs/playground/index.html` entrypoint proof.
- `extensions/arukellt-all-in-one/package.json` restricts `arukellt.playgroundUrl` enum/default to the repo-proved URL.
- `extensions/arukellt-all-in-one/src/extension.js` guards `openInPlayground` against non-allowed URLs.
- `extensions/arukellt-all-in-one/README.md` now states only the repo-proved route is supported.
- Verification in slice commit `0426b64`: `python scripts/manager.py verify quick` PASS, `npm --prefix extensions/arukellt-all-in-one run test:playground-endpoint` PASS.

**Action**: Move to `issues/done/` with status `done`.

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: docs/playground/index.html is real browser entrypoint calling createPlaygroundApp(), pages.yml deploys it

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/469-extension-playground-surface-points-to-repo-proved-endpoint.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

VS Code extension の playground command / config / README が、current repo で route / build / publish を証明できる endpoint だけを expose するようにする。repo 外 URL を sole proof にする構成を禁止する。

## Visibility

user-visible

## Why this is a separate issue

extension exposure は product wiring より後段である。ここを実装・deploy と混ぜると、リンクだけ先に出して false-done になる。

## Primary paths

- `extensions/arukellt-all-in-one/package.json`
- `extensions/arukellt-all-in-one/src/extension.js`
- `extensions/arukellt-all-in-one/README.md`
- route / publish proof from issues 466 and 468

## Allowed adjacent paths

- repo docs that state the canonical playground route

## Non-goals

- playground product code の実装
- docs shell route wiring
- deploy workflow の追加

## Acceptance criteria

- [x] extension command / config が指す playground endpoint は、issues 466 と 468 の repo proof から辿れる path だけである。
- [x] README の description は actual repo-proved endpoint behavior と一致する。
- [x] repo proof がない endpoint を default value や user-visible command で expose しない。

## Required verification

- extension command / setting value を grep する
- value を route / build / publish proof と突き合わせる
- README text を current behavior と比較する

## Close gate

- repo 内の現物ファイルが列挙されている
- user-visible command / setting が repo proof と一致している
- repo 外 URL を sole basis とした close を禁止する
- extension exposure だけで product availability を claim しない

## Evidence to cite when closing

- `extensions/arukellt-all-in-one/package.json`
- `extensions/arukellt-all-in-one/src/extension.js`
- `extensions/arukellt-all-in-one/README.md`
- prerequisite route / build / publish proof files

## False-done prevention checks

- Can this be closed with only parts existing? **No**
- Can docs get ahead and still allow close? **No**
- Can extension expose a link and still allow close without route proof? **No**
- Can deploy be claimed without workflow/output proof? **No**
- Does this rely on a repo-external URL as proof? **No**
- Can it be closed without concrete evidence files? **No**
- Does this contain a user-visible claim without entrypoint acceptance? **No**

## False-done risk if merged incorrectly

high — stale external URL を開くだけで feature shipped に見えてしまう。
