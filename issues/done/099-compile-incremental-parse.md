# コンパイル速度: インクリメンタル解析 (ファイル変更差分のみ再パース)

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-15
**ID**: 099
**Depends on**: —
**Track**: compile-speed
**Blocks v4 exit**: no

## Summary

`arukellt compile --watch` モードや LSP (`ark-lsp`) での繰り返しコンパイルで、
変更のないファイルの AST を再利用できる最小インクリメンタル解析面を導入する。

## Completion note — bounded slice accepted

2026-04-15 に bounded slice として完了。
フル production incremental parser や依存グラフベースの下流再コンパイルまでは実装していないが、
実際のコンパイル経路で使われる parse cache hook と changed-file-only reparse entry point が追加された。

今回の着地:

- `crates/ark-driver/src/session.rs` に `parse_incremental()` / `reparse_changed_files()` / `parse_cache_stats()` を追加
- `mtime` が変わらないファイルでは AST を再利用し、変更ファイルだけを再パースできる
- `crates/ark-resolve` に parser callback hook を追加し、通常の module graph load でも session parse cache を使うようにした
- 回帰テストで「未変更ファイル reuse」「変更ファイルだけ reparse」「import graph load が parse cache を埋める」を確認

## Acceptance

- [x] `crates/ark-driver/src/session.rs` に real incremental parse surface (`parse_incremental`, `reparse_changed_files`) がある
- [x] 未変更ファイルは AST cache hit で再利用される
- [x] changed-file-only reparse entry point があり、無関係なファイルの AST cache を保持する
- [x] 通常の module graph load path でも parse cache hook が使われる
- [x] 回帰テストが追加されている
- [x] `cargo build --workspace --exclude ark-llvm` exits 0
- [x] `bash scripts/run/verify-harness.sh --quick` exits 0

## Key files

- `crates/ark-parser/src/lib.rs`
- `crates/ark-resolve/src/load.rs`
- `crates/ark-resolve/src/resolve.rs`
- `crates/ark-driver/src/session.rs`

## References

- roadmap-v4.md §2