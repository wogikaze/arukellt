# MIR 最適化パスの --opt-level 分離と passes/ ディレクトリ構造確立

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-14
**ID**: 122
**Depends on**: 101
**Track**: mir-opt
**Blocks v4 exit**: yes

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/122-compile-mir-opt-level-separation.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

roadmap-v4.md §6 item 1 で要求されている
`crates/ark-mir/src/passes/` ディレクトリ構造と `OptimizationPass` トレイトを確立する。
現在 `opt/pipeline.rs` に集約している全パスを独立ファイルに分割し、
`--opt-level` による有効/無効の制御を統一インタフェースで実装する。

## 受け入れ条件

1. `crates/ark-mir/src/passes/` ディレクトリを新設し、各パスを独立ファイルに移動
2. `fn run(module: &mut MirModule, level: OptLevel) -> PassStats` シグネチャの統一
3. `--no-pass=<name>` フラグで個別パスを無効化できる
4. `passes/README.md` に各パスの説明・適用条件・依存関係を記載

## 参照

- roadmap-v4.md §5.1 および §6 item 1
