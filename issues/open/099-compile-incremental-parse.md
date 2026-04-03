# コンパイル速度: インクリメンタル解析 (ファイル変更差分のみ再パース)

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 099
**Depends on**: —
**Track**: compile-speed
**Blocks v4 exit**: no

**Status note**: Compiler architecture improvement — deferred to v5+. hello.ark already compiles in 4.2ms.


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/099-compile-incremental-parse.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`arukellt compile --watch` モードや LSP (`ark-lsp`) での繰り返しコンパイルで、
変更のないファイルの AST・MIR をキャッシュして再利用する
インクリメンタルコンパイルの基盤を設計・実装する。

## 受け入れ条件

1. `ark-driver/src/session.rs` にコンパイルキャッシュ (`HashMap<PathBuf, (mtime, AstModule)>`) を追加
2. ファイル mtime が変わっていない場合、前回の AST を再利用
3. 変更ファイルに依存する下流モジュールのみを再コンパイル
4. `--watch` フラグ: ファイル変更を監視して自動再コンパイル (notify crate)
5. 2回目のコンパイルが変更なしの場合に 80% 以上の時間削減

## 参照

- roadmap-v4.md §2 (コンパイル時間目標)
