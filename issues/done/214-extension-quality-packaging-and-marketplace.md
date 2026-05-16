---
Status: done
Created: 2026-03-30
Updated: 2026-05-14
ID: 214
Track: parallel
Depends on: 184, 185, 186, 187, 188
Orchestration class: blocked-by-upstream
Orchestration upstream: None
Blocks v{N}: none
Reason: No icon/banner/screenshots evidence. Marketplace readiness overclaimed.
Action: Moved from `issues/done/` to `issues/open/` by false-done audit.
---

# Extension quality / packaging / marketplace readiness

## Reopened by audit — 2026-04-13

## Completed — 2026-05-14

Audit gap closed with marketplace-visible branding and packaging evidence:

- added `media/icon.png` and README screenshot asset `media/command-palette.png`
- populated marketplace metadata with `icon`, `galleryBanner`, categories, keywords, publisher, repository, bugs, homepage, and engine range
- added `CHANGELOG.md` and `RELEASE.md`
- added `test/marketplace-metadata-guard.js` for metadata/readme artifact checks
- added `test/run-vsce-package.js` so `vsce package` includes production dependencies from `package-lock.json` deterministically in this environment
- tightened `.vscodeignore` so VSIX output excludes local test harness files

Verification:

- `npm run test:marketplace-metadata` -> `marketplace metadata OK`
- `npm run build` -> `DONE Packaged: .../arukellt-all-in-one-0.0.1.vsix (327 files, 486.41 KB)`

## Summary

extension integration tests・smoke tests・fixture project 群・packaging check・marketplace metadata・icon / branding・changelog・release 手順・web extension 対応可否・remote / codespaces 互換性確認を整備する。

現状は品質保証の仕組みがなく、marketplace への配布に必要なメタデータ・アイコン・changelog も未整備。

## Acceptance

- [x] smoke tests / integration tests が CI で実行される
- [x] marketplace 配布に必要な metadata (icon, categories, keywords, publisher) が整っている
- [x] changelog と release 手順が文書化されている

## Scope

### Tests

- extension integration tests（VS Code test runner 使用）
- smoke tests（拡張を起動して最低限の動作確認）
- fixture project 群（ark.toml あり / なし、multi-root、stdlib のみ等）
- packaging check（`vsce package` が警告なく通る）

### Marketplace metadata

- icon / banner 画像
- categories: `Programming Languages`, `Linters`, `Debuggers`, `Testing`
- keywords, repository, bugs, homepage 各フィールド
- `engines.vscode` の適切なバージョン指定

### Branding / docs

- README に screenshot / gif、セットアップ手順、対応 target 一覧、既知の制約、troubleshooting
- sample `tasks.json` / `launch.json` / `ark.toml` の同梱

### Compatibility

- web extension 対応可否の調査と判断
- remote / dev container / GitHub Codespaces での動作確認
- release 手順（semver, tag, publish workflow）の文書化

## References

- `issues/open/183-vscode-arukellt-all-in-one-extension-epic.md`
- `issues/open/184-vscode-extension-foundation.md`
- `extensions/arukellt-all-in-one/`
