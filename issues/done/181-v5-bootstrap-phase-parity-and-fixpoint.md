# v5 Bootstrap: phase parity and fixpoint verification

**Status**: open
**Created**: 2026-03-29
**ID**: 181
**Depends on**: 165, 167
**Track**: main
**Blocks v1 exit**: no

## Summary

Rust 版と selfhost 版の phase 出力比較、Stage 0→1→2 の fixpoint 検証、生成物比較の導線を整備する。bootstrap verification の中核になる子 issue。

## Acceptance

- [ ] Rust 版 / selfhost 版の比較導線がある
- [ ] Stage 0→1→2 fixpoint 検証を追跡できる
- [ ] dump phases (#167) と連動したデバッグ入口がある

## References

- `issues/open/165-v5-phase3-wasm-emitter.md`
- `issues/open/167-v5-debug-dump-phases.md`
