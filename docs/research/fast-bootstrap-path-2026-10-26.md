# Fast bootstrap paths — negative result (2026-10-26)

Status: research note (not a decision)
Related: #851, ADR-053, `docs/compiler/bootstrap.md`, `bootstrap/PROVENANCE.md`

## Question

Can `selfhost build-compiler` drop from 21-41 min to ~50 s without changing the
official pin, while still producing a compiler that passes the fixture gate?

## Candidates and results

| Path | Build wall | Output | Gate result |
|---|---:|---|---|
| official pin `a2803831` (10/19) | 21-41 min | 11.5 MB, valid | 1832/1832 PASS |
| fast pin `3f0ca605` (10/10) | 39.9 s probe, 60.2 s cold | 11.1 MB, valid | 1831/1832 |
| gc host `50cc7576` (10/11) | 111.7 s | 8.3 MB | `wasm-tools validate` fails |

## Fast pin

`3f0ca605` compiles the current source for `wasm32/p1` in 43 s, and the failing
fixture compiles successfully on that path. The failure is specific to the
fixture gate's `wasm32-gc` / `wasi-p2` + `--memory-width memory32` compile:
`stdlib_trait/iterator_string_callbacks.ark` never finishes (30 s and 90 s
timeouts both fire). The pin predates #849 (join-temp declarations), so its
codegen for the newer gc path is not trustworthy.

Consequence: `build-compiler --fast` is usable for p1 smoke work and stage-2
iteration, but an s2 it builds is not fixture-valid.

## gc host

`.build/selfhost/arukellt-s3-gc.component.wasm` (sha `50cc7576…`) compiles the
current flat overlay to `wasm32-gc` / `wasi-p2` in 111.7 s (RSS ~1.43 GiB), but
the output fails `wasm-tools validate`:

```text
type mismatch: expected (ref null $type), found (ref null $type)
(at offset 0x21036d)
```

The host predates the current source's gc type rules; the Oct-11 49.5 s receipt
was for the Oct-11 source, not today's.

## Conclusion

The pre-built fast artifacts are not faithful: the 10/10 core pin miscompiles
the gc fixture path, and the Oct-11 gc host emits invalid gc wasm.

The working path is to rebuild the gc host from the current source with the
official s2 (one-time, 19.5 min) and package it as a P2 component. That
component compiles the overlay to p1 in 73 s and its output is byte-identical
to the official s2 (`e55825fd…`). It is committed as
`bootstrap/arukellt-selfhost-gc.component.wasm` and used by
`build-compiler --fast`.

## Reproduce

The gc-host probe is reproducible from the worktree:

```bash
cp -a .build/selfhost/flat-src .build/gc-overlay
wasmtime run --wasm gc --wasm function-references -W memory64=y \
  -W max-memory-size=17179869184 --wasm max-wasm-stack=16777216 \
  --dir .build/gc-overlay .build/selfhost/arukellt-s3-gc.component.wasm -- \
  compile src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2 \
  -o s2-gc.wasm
wasm-tools validate .build/gc-overlay/s2-gc.wasm
```
