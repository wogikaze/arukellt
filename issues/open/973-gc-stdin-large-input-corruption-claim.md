---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 973
Track: wasi
Depends on: none
Orchestration class: verification-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "ABC481 arukellt port review (2026-10-28, HEAD 2436c92b0; deliverable README claim)"
Related: "#807, #957, #947, #948"
---

# 973 — `wasm32-gc` stdin above ~250 bytes is reported corrupt; reproduce it or retire the workaround

## Summary

The ABC481 arukellt port records that the `wasm32-gc` component stdin path breaks
input larger than about 250 bytes, and every solution that reads stdin carries an
argv-file fallback because of it:

> 現行 selfhost の gc component では stdin 経路（#807 系）が 250 バイトを超える入力の
> 末尾を壊すため、gc ターゲットでローカルに stdin を使うことはできない。

The port README and the source comments still state this, so gc stdin is documented
as unusable for real inputs and the deliverable carries a dual-mode input path that
would otherwise be unnecessary. The claim needs either a minimal repro or retirement.

## Current status (2026-10-28 16:40, pin `8df2698c`, s2 `ab08448d`)

A first verification attempt did **not** reproduce the corruption:

- `stdio::read_stdin()` of a 4000-byte piped input: `len=4000`, bytes at 200/250/300
  and the last byte all correct.
- `__intrinsic_stdin_read_to_string()` of the same 4000 bytes: same result.
- C solution (`read_stdin` + byte parser) against a 1104-byte and an 11 KB input on
  the gc component: output matched the reference implementation.
- All gc sample runs fed through stdin pass, including E (449 B) and G (374 B).

The corruption was recorded before the 2026-10-28 05:51 pin refresh
(`8df2698c`, `wave/pin-i64-fix`), so it may be a pre-refresh observation. The exact
input, solution and read API were not recorded, so this issue starts with
reproduction rather than a fix.

## Why it matters

- If real, every gc-target program that reads stdin is affected and the documented
  argv fallback is required.
- If not, the port README, `std/host/stdio.ark` docs (#957) and solution comments
  are stale and mislead users into avoiding gc stdin.

## Acceptance

- [ ] Reproduce the >250-byte gc stdin corruption with a minimal `.ark` probe, or
      show that the original observation was a pre-pin-refresh artifact.
- [ ] If real: fix the WASI P2 stdin read loop and add a fixture that runs on
      `wasm32-gc` with a >250-byte input, asserting `len` and the first/last byte.
- [ ] If not real: update `std/host/stdio.ark` docs (#957) and the deliverable's
      argv-fallback guidance, and add a fixture proving gc stdin >250 bytes works.
- [ ] Fixture is registered as `run:` so both targets execute it.

## Notes

- Related #807 (done) is fixture parity, not the stdin read loop; #957 covers the
  `read_stdin` body/doc disagreement; #947/#948 cover gc print handle/scratch leaks.
- Repro artifacts from the port live in `tmp/issues/probe_stdin.ark`,
  `tmp/issues/probe_stdin2.ark`, `tmp/issues/long412.txt` (gitignored).
