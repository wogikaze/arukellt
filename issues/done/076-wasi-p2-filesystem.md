---
Status: done
Created: 2026-03-28
Updated: 2026-08-14
ID: 076
Track: wasi-feature
Depends on: 074, 510
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v4 exit: no
Status note: Re-closed 2026-08-14 after real-WASI P2 runtime adapter disk-write gate passed.
---

# WASI P2 ネイティブ: wasi:filesystem ネイティブバインディング

## Summary

WASI Preview 2 filesystem operations now cross the versioned `arukellt:runtime/host@0.1.0`
component boundary and are implemented by the checked `wasm32-wasip2` runtime adapter.
Compiled P2 components are linked with that adapter and execute under stock Wasmtime;
the product path no longer depends on the old `arukellt:fs` host bridge for P2 filesystem
operations.

## Close gate

- `gate-076-wasi-p2-filesystem.py` verifies the P2 runtime filesystem import shape, checked real-WASI adapter implementation, compatibility bridge routing, and product adapter linking.
- The strict filesystem fixture contract remains `wasi_fs_p2.ark` → stdout `hello p2 fs` plus on-disk `p2_fs_out.txt` content `hello p2 fs` when the runtime E2E lane is enabled.
- `gate_074` remains unchanged.

## Serial audit history

- **5b9e5b3e / 7f069f90**: false-close — stdout-only stub adapt; `p2_fs_out.txt` missing.
- **c141540**: reopened; honest close requires disk-write acceptance.
- **PR #46**: replaced the transitional P2 filesystem bridge with the versioned runtime ABI + checked WASI P2 adapter and restored the production close gate.

## 受け入れ条件 (gate slice)

1. `wasi_fs_p2.ark` writes `p2_fs_out.txt` through the production component runtime path — **met**.
2. `gate-076-wasi-p2-filesystem.py` validates the real-WASI filesystem boundary and disk-write contract — **met**.
3. Full canonical ABI `descriptor` resource rollout — **deferred**, as allowed by this issue's scoped acceptance; the runtime adapter owns that resource boundary.

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md`
- `issues/done/074-wasi-p2-native-component.md`
- `runtime/wasi-p2-adapter/`
- `runtime/wasi-p2-bridge/`

## Close receipt — 2026-08-14

- Dedicated close gate: PASS
- `python3 scripts/manager.py verify quick`: PASS in PR #46 CI
- Verification harness / docs consistency / selfhost gates: PASS in PR #46 CI
- Implementation PR: #46 (`feat(wasi): productionize runtime ABI and real WASI host paths`)
