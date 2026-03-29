# Runtime inspection / stepping / evaluate surface

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 200
**Depends on**: 199
**Track**: parallel
**Blocks v1 exit**: no

## Summary

breakpoint、continue、step、stack frames、locals、evaluate、panic/trap source mapping をまとめて、通常の source-level debugging で必要な inspection surface を追う。

## Acceptance

- [ ] breakpoint / stepping / evaluate の責務が追跡できる
- [ ] stack frame / locals / runtime inspection の責務が整理されている
- [ ] panic / trap / assertion failure との接続を issue queue 上で追跡できる

## References

- `issues/open/187-debug-surface-dap-and-source-level-debugging.md`
- `issues/open/199-debug-metadata-and-dap-adapter-foundation.md`
- `docs/compiler/diagnostics.md`
