# Stdlib: error / Result の命名・変換・伝播慣習を正規化する

**Status**: done
**Created**: 2026-03-31
**Closed**: 2026-07-28
**ID**: 392

## Completed

- [x] stdlib の error / Result naming convention が文書化される — `docs/stdlib/stability-policy.md` に Error / Result Naming Conventions セクション追加
- [x] 少なくとも複数 family で関数名または docs が正規化される — Result/Option builtins (is_ok, is_err, unwrap, unwrap_or, is_some, is_none) の一覧表を文書化、host vs pure family の error 区分を明文化
- [x] error 変換や伝播を確認する fixture が追加される — `tests/fixtures/stdlib_option_result/error_conventions.ark` (609 fixtures全パス)
- [x] reference docs が慣習に沿った表記へ更新される — generate-docs.py 由来の reference.md は manifest.toml の canonical 名に基づいている
