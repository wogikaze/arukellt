---
Status: open
Created: 2026-03-28
Updated: 2026-07-12
ID: 076
Track: wasi-feature
Depends on: 074, 510
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v4 exit: no
Status note: Closed 2026-06-14 — gate_076 passes (validate + wasmtime stdout `hello p2 fs` + `p2_fs_out.txt` on disk via preview1 reactor + guest fs patch).
---
## Reopened 2026-07-12 (close-gate failure)

Close gate failed under current selfhost path; moved back to `issues/open/` so verify is not blocked (false-done prevention). Re-close when gate passes.

# WASI P2 ネイティブ: wasi:filesystem ネイティブバインディング

## Summary

WASI Preview 2 component path for `std::host::fs::write_string` on pinned bootstrap:
guest core fd_write/path_open/fd_close imports are retargeted to `wasi_snapshot_preview1`
via `p2_guest_fs_patch.py`, and `p2_component_wrap.py` links the preview1 reactor adapter
during `wasm-tools component new` so `wasi_fs_p2.ark` persists bytes under
`wasmtime run --dir <repo>`.

## Close gate

- `gate_076`: `component-compile:wasi_fs_p2.ark` → validate → wasmtime stdout `hello p2 fs` (strict UTF-8) + `p2_fs_out.txt` content `hello p2 fs`
- `gate_074`: unchanged green

## Implementation notes

- `scripts/selfhost/p2_guest_fs_patch.py` — retarget fs imports to preview1; route mis-emitted fd_write off stdout import 0
- `scripts/selfhost/p2_component_wrap.py` — `--adapt wasi_snapshot_preview1=<reactor>` on component new (locate or fetch wasmtime reactor wasm)
- `scripts/check/check-false-done-close-gates.py` — gate_076 checks on-disk artifact

## Serial audit history

- **5b9e5b3e / 7f069f90**: false-close — stdout-only stub adapt; `p2_fs_out.txt` missing
- **c141540**: reopened; honest close requires disk write acceptance

## 受け入れ条件 (gate slice)

1. `wasi_fs_p2.ark` writes `p2_fs_out.txt` via wasmtime `run --dir <repo>` — **met**
2. `gate_076` strict stdout + file content — **met**
3. Full canonical ABI `descriptor` resource rollout — **deferred** (pinned-bootstrap post-wrap path only)

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md`
- `issues/done/074-wasi-p2-native-component.md`
