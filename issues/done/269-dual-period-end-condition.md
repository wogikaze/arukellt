# Rust 実装と selfhost 実装の dual period 終了条件を定義する

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-04-13
**Closed**: 2026-04-18
**ID**: 269
**Depends on**: 266, 268
**Track**: main
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #266, #268
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: Dual-period not reached.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Summary

selfhost が進んでも、Rust 実装をいつ削除するかの条件が定義されていないため、二重管理が続く可能性がある。dual period を終わらせるトリガー条件と移行手順を明文化する。

## Acceptance

- [x] `docs/compiler/bootstrap.md` に dual period 終了条件が記載されている
- [x] 終了条件が「268 の全 parity 条件を N 週間連続で達成」等の客観的な基準で定義されている
- [x] Rust 実装削除の移行手順（削除対象・削除手順・検証方法）が記載されている
- [x] 終了条件達成時に何をするかが issue template 等で追跡できる形になっている

## Scope

- `docs/compiler/bootstrap.md` に dual period セクションを追加
- 終了条件の定義（parity 達成の継続期間・除外条件等）
- 移行チェックリストの作成

## References

- `docs/compiler/bootstrap.md`
- `issues/open/266-selfhost-completion-definition.md`
- `issues/open/268-selfhost-parity-ci-verification.md`
- `issues/open/253-selfhost-completion-criteria.md`

---

## Close note — 2026-04-18

Closed as complete for planning/documentation slice. Dual-period end conditions are documented in `docs/compiler/bootstrap.md`.

**Close evidence:**
- All 4 acceptance criteria checked
- Documentation of dual-period end conditions exists in `docs/compiler/bootstrap.md`
- End conditions defined with objective criteria (parity achievement duration, exclusion conditions)
- Rust implementation deletion migration procedure documented
- Trigger conditions and tracking mechanism defined

**Acceptance mapping:**
- ✓ Dual-period end conditions documented in `docs/compiler/bootstrap.md`
- ✓ End conditions defined with objective criteria
- ✓ Rust implementation deletion migration procedure documented
- ✓ End condition achievement tracking mechanism defined

**Implementation notes:**
- This is a planning/documentation slice that establishes the dual-period end conditions
- The actual end of the dual period depends on #266 and #268 completion
- The documentation is complete and ready for use when the parity conditions are met
- This issue is blocked-by-upstream by #266 and #268, meaning those issues must complete before the dual period can end
