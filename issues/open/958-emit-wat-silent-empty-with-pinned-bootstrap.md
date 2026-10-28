---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 958
Track: selfhost-infra
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: "ABC481 arukellt port + WAT emission session (2026-10-28, HEAD 0a64bdb62)"
Related: "#801, ADR-029"
---

# 958 — `--emit wat` silently writes a 0-byte file with the pinned bootstrap

## Summary

`--emit wat` is now backed by a real Wasm-to-WAT disassembler
(`src/compiler/wat_disasm.ark`, wired in `src/compiler/driver/emit.ark`), and a freshly
built stage-2 emits valid WAT for `wasm32` and `wasm32-gc`.

The default entrypoint still resolves the pinned `bootstrap/arukellt-selfhost.wasm`,
which was produced from the old overlay that stubbed `emit_wat` to `String_from("")`.
With that artifact the CLI reports success and writes an empty file:

```
$ scripts/run/arukellt-selfhost.sh compile --target wasm32 --emit wat -o .build/x.wat tmp/hello.ark
compilation succeeded (phase 6)
wrote 0 bytes to .build/x.wat
```

A user sees a successful WAT build and an unusable artifact.

## Expected

- A 0-byte WAT output must not be reported as success: emit a diagnostic that the
  current compiler artifact cannot emit WAT (suggest `ARUKELLT_SELFHOST_WASM=<fresh s2>`),
  or
- Refresh the pinned bootstrap as part of the pin-refresh wave so the default path has
  the disassembler.

## Repro

```
$ scripts/run/arukellt-selfhost.sh compile --target wasm32 --emit wat -o .build/x.wat <any>.ark
$ wc -c .build/x.wat      # 0, exit status 0
$ ARUKELLT_SELFHOST_WASM=$PWD/.build/selfhost/arukellt-s2-runtime.wasm \
    scripts/run/arukellt-selfhost.sh compile --target wasm32 --emit wat -o .build/x.wat <any>.ark
$ wc -c .build/x.wat      # non-zero, wasm-tools parse + validate OK
```

## Notes

- A regression check was added to `scripts/run/wat-roundtrip.sh`: it validates
  `--emit wat` output when non-empty and currently SKIPs while the pin is stubbed.
- After the pin refresh this issue closes by deleting the stub-era special case, if the
  explicit diagnostic is not needed.

## Acceptance

- [ ] Default `--emit wat` either produces valid WAT or fails with an explicit diagnostic.
- [ ] `wat_roundtrip` (or `verify wat`) covers the compiler-side `--emit wat` path once the pin refreshes.
