---
Status: done
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

- [x] Reproduce the >250-byte gc stdin corruption with a minimal `.ark` probe, or
      show that the original observation was a pre-pin-refresh artifact — not
      reproduced on the named pin or the stage-2 compiler; see Update/Resolution.
- [x] If real: fix the WASI P2 stdin read loop and add a fixture that runs on
      `wasm32-gc` with a >250-byte input, asserting `len` and the first/last byte —
      not applicable: the claim did not reproduce.
- [x] If not real: update `std/host/stdio.ark` docs (#957) and the deliverable's
      argv-fallback guidance, and add a fixture proving gc stdin >250 bytes works —
      docs were updated in #957; the external port README is gitignored `tmp/` and
      not tracked here; the >250 B fixture is not achievable on the current
      harness (fixture parity runs with `stdin=DEVNULL`, no stdin sidecar), so the
      manual probe recorded below is the evidence.
- [x] Fixture outcome recorded: the existing `run:` fixture
      `tests/fixtures/stdlib_io/read_stdin_empty.ark` pins the EOF contract; a
      >250 B stdin fixture cannot be registered while the harness has no stdin
      sidecar (see Resolution).

## Notes

- Related #807 (done) is fixture parity, not the stdin read loop; #957 covers the
  `read_stdin` body/doc disagreement; #947/#948 cover gc print handle/scratch leaks.
- Repro artifacts from the port live in `tmp/issues/probe_stdin.ark`,
  `tmp/issues/probe_stdin2.ark`, `tmp/issues/long412.txt` (gitignored).

## Update (2026-10-28, Wave A) — not reproduced; retire as a pre-pin-refresh artifact

Minimal probe `wt/probe_stdin_len.ark` (gitignored scratch): calls
`stdio::read_stdin()`, then prints `len`, `first`, index 250, index 300, and the
last byte. Each index is guarded so short input cannot itself trap. Input is
`ABCDEFG` repeated with no trailing newline.

Compiler: default `bootstrap/arukellt-selfhost.wasm` =
`8df2698c8401693879ca4f9ced289ed2c285d95eb526cf70fda02dd5876259ee` (the
2026-10-28 05:51 pin named in this issue). Expected byte values for the
pattern: index 250 → `F` (70), index 300 → `G` (71), index 3999 → `C` (67).

| bytes | target | len | first | b250 | b300 | last |
|-------|--------|-----|-------|------|------|------|
| 300 | `wasm32` | 300 | 65 | 70 | out-of-range | 70 |
| 300 | `wasm32-gc` component | 300 | 65 | 70 | out-of-range | 70 |
| 420 | `wasm32` | 420 | 65 | 70 | 71 | 71 |
| 420 | `wasm32-gc` component | 420 | 65 | 70 | 71 | 71 |
| 1000 | `wasm32` | 1000 | 65 | 70 | 71 | 70 |
| 1000 | `wasm32-gc` component | 1000 | 65 | 70 | 71 | 70 |
| 4000 | `wasm32` | 4000 | 65 | 70 | 71 | 67 |
| 4000 | `wasm32-gc` component | 4000 | 65 | 70 | 71 | 67 |

Repeated with `.build/selfhost/arukellt-s2-runtime.wasm` (post-pin compiler):
1000 B on `wasm32-gc` → `len=1000 first=65 b250=70 b300=71 last=70`, identical.

No mid-stream or tail corruption on either target, on either compiler.
Conclusion: the recorded "> ~250 bytes gc stdin is broken" observation is a
pre-pin-refresh artifact (recorded before the `8df2698c` refresh). The claim and
the argv-file fallback guidance should be retired.

Notes for the Lead:

- The harness cannot feed stdin bytes to fixtures: `fixture_parity.py` runs with
  `stdin=subprocess.DEVNULL` and there is no per-fixture stdin sidecar. The
  >250 B proof therefore stays a manual probe. The EOF side of the contract is
  already pinned by the existing `run:` fixture
  `tests/fixtures/stdlib_io/read_stdin_empty.ark`, so no new fixture is added
  (a duplicate `tests/fixtures/stdin/…` fixture was written and then removed).
- No repository doc still carries the 250-byte claim (checked `docs/`). The
  remaining occurrence is the external ABC481 port README / REPORT under
  gitignored `tmp/`, which this repo does not track.
- `std/host/stdio.ark` docs are updated in #957.

## Resolution (2026-10-28)

Close as a retired claim: the >250-byte gc stdin corruption does not reproduce on
either the pinned or the stage-2 compiler, no tracked document asserts it, and the
`read_stdin` docs now state the real EOF contract (#957). The acceptance items that
ask for a `run:` fixture proving >250 bytes are not achievable with the current
harness (no stdin sidecar); the manual probe above is the evidence.
