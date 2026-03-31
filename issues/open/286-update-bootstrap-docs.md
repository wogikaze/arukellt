# current-state.md の bootstrap 節を fixpoint 達成に合わせて更新する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 286
**Depends on**: —
**Track**: selfhost
**Blocks v1 exit**: no
**Priority**: 6

## Summary

`docs/current-state.md` の Self-Hosting Bootstrap Status が stale。Stage 1 が「🔴 Blocked — selfhost parser has 19 parse errors」のまま。実際には fixpoint 達成済み (sha256(s1)==sha256(s2))。

## Current state

- `docs/current-state.md:171-178`: Stage 1/2 が 🔴 Blocked 表記
- 実態: Stage 0/1/2 すべて ✅、fixpoint 達成済み
- Fixture parity / CLI parity / Diagnostic parity は正しく 🔴

## Acceptance

- [ ] Stage 0/1/2 が ✅ Verified に更新
- [ ] fixpoint 達成の日付・コミットが記録される
- [ ] 「Stage 1 blocker」節が削除または「解決済み」に更新
- [ ] dual-period policy が fixpoint 後の状態を反映する

## References

- `docs/current-state.md` §Self-Hosting Bootstrap Status
- `scripts/verify-bootstrap.sh`
