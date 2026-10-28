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

## Progress — 2026-10-28 (explicit diagnostic implemented)

`scripts/run/arukellt-selfhost.sh` now routes `compile`/`build` with
`--emit wat` (both `--emit wat` and `--emit=wat`) through `compile_wat_output`.
When the selected artifact exits 0 but produces no WAT bytes, the launcher
suppresses the compiler's success lines, deletes the empty artifact, prints a
diagnostic that the artifact cannot emit WAT (with the
`ARUKELLT_SELFHOST_WASM` rebuild hint) and exits 1.

Reproduced and re-verified on this checkout (default pinned artifact, wasmtime
46.0.1):

| command | before | after |
|---|---|---|
| default (pinned) `compile --emit wat -o x.wat` | exit 0, `wrote 0 bytes`, 0-byte `x.wat` left | exit 1, diagnostic, `x.wat` removed |
| default (pinned), no `-o` | exit 0, stdout `ok: 0 bytes` | exit 1, diagnostic |
| `ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2-runtime.wasm` | 21785 bytes | 21785 bytes, `wasm-tools parse` + `validate` OK |
| plain `compile --target wasm32` (no `--emit wat`) | ok | ok (2747 bytes) |
| `run --target wasm32 tests/fixtures/hello/hello.ark` | ok | ok (`Hello, world!`) |

`scripts/run/wat-roundtrip.sh` no longer treats an empty `--emit wat` file as
SKIP. It follows the launcher: the `cannot emit WAT` diagnostic counts as one
SKIP (with an explicit note), any other emit failure counts as FAIL, and an
empty artifact produced despite a zero exit is a FAIL. Verified with test
doubles: pinned artifact → emit `SKIP=1`, exit 0; fresh s2 → emit `PASS=2`,
exit 0; injected generic emit failure → emit `FAIL=2`, exit 1.

Still open: the default pinned `bootstrap/arukellt-selfhost.wasm` remains
stub-era, so the default path now fails loudly instead of silently. Acceptance
item 1 is satisfied by the diagnostic; item 2 is covered once the pin refresh
lands (the roundtrip check already validates non-empty output and passes with
`ARUKELLT_SELFHOST_WASM` pointing at a fresh s2).
