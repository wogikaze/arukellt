# MIR: ARUKELLT_DUMP_PHASES=optimized-mir 対応

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 119
**Depends on**: 101
**Track**: mir-opt
**Blocks v4 exit**: yes

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/119-mir-pass-trace-dump.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

roadmap-v4.md §6 item 2 で要求されている MIR 最適化トレースダンプ機能を実装する。
`ARUKELLT_DUMP_PHASES=optimized-mir` 環境変数で、最適化前後の MIR を比較できる形式で
stderr に出力する。デバッグと最適化効果の確認に使用する。

## 受け入れ条件

1. `MirStats.optimization_trace: Vec<(PassName, ReductionStats)>` フィールド追加
2. `ARUKELLT_DUMP_PHASES=optimized-mir` で各パス適用前後の MIR を出力
3. `ARUKELLT_DUMP_PHASES=all` で全フェーズのダンプ
4. 出力形式: 関数名・パス名・削減された命令数・変換前後の MIR テキスト

## 参照

- roadmap-v4.md §6 item 2
