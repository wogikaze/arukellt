# Advanced debug intelligence

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 201
**Depends on**: 200
**Track**: parallel
**Blocks v1 exit**: no

## Summary

time-travel debug、value history graph、`why panic?` explain、cross-module stepping visualization、step 間 state diff など、最小 DAP の上に載る高度な debug DX を追う。

## Acceptance

- [ ] history/time-travel 系 UX の責務が追跡できる
- [ ] value history / panic explanation / cross-module visualization が定義されている
- [ ] advanced debug DX の残課題を issue queue 上で追跡できる

## References

- `issues/open/187-debug-surface-dap-and-source-level-debugging.md`
- `issues/open/200-runtime-inspection-stepping-and-evaluate-surface.md`
- `docs/compiler/bootstrap.md`
