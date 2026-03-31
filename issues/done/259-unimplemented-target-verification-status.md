# 未実装 target (T2/T4/T5) の検証配線状況を文書化する

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 259
**Depends on**: 257
**Track**: main
**Blocks v1 exit**: no

## Summary

T2/T4/T5 は「未実装」として記載されているが、どの検証が未配線でどこまでが scaffold かが不明である。このため、将来これらの target を実装し始めるときに、何から手をつければよいかが分からない。

## Acceptance

- [x] `docs/target-contract.md` で T2/T4/T5 の各行に `scaffold / blocked / not-started` が明記されている
- [x] T2/T4/T5 のそれぞれについて「どこまで実装されているか」「何が必要か」が 1 段落以内で記述されている
- [x] T2/T4/T5 に関連する既存の scaffold コードがコメントまたは `#[allow(dead_code)]` 等で識別されている

## Scope

- コードベースを調査して T2/T4/T5 の scaffold 状況を確認
- `docs/target-contract.md` に T2/T4/T5 の状況行を追加
- scaffold コードに `// T2 scaffold` 等のコメントを追加（必要に応じて）

## References

- `docs/target-contract.md`（257 で作成）
- `issues/open/251-target-matrix-execution-contract.md`
