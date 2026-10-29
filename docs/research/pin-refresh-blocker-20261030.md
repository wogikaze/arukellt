# Pin refresh blocked on gc self-build type consistency (2026-10-30)

Tracking: #851 / #917. This note records why
`bootstrap/arukellt-selfhost.wasm` was **not** refreshed on 2026-10-30. It is
evidence for the next attempt, not a decision. The committed pin stays
`471855ad…` (8 550 341 bytes).

## Reproduction

Lane: `.worktrees/wave-pin-refresh2` (`wave/pin-refresh2`, base `6c8609d35`),
`ARUKELLT_BUILD_DIR=$PWD/.build-lane`, gc pin, wasmtime 46.0.1.

The committed pin (`471855ad…`) fails `tests/fixtures/operators/string_eq.ark`
at clean HEAD with `compiler produced invalid core Wasm`, while the fresh
stage-2 built by `selfhost build-compiler` from the same source compiles it and
runs all 31 lines correctly on both targets. That is the pin-oracle gap this
lane set out to close.

## Blocker 1 — overlay omitted the parse implementation modules

Compiling the flat overlay to `wasm32-gc --emit component` with the committed
pin failed validation at `f64::try_from__String` (func 306):

```text
type mismatch: expected i32, found (ref null $type)
```

The bootstrap overlay copied `std/core/convert.ark` but not
`std/core/scalar_format.ark` / `std/core/raw.ark`. `convert.ark`'s
`__core_parse_f64_impl` call therefore fell back to the i32 `parse_f64`
intrinsic, while the gc handler (`wasm/intrinsic_parse_f64_gc.ark`) returns a
`Result` reference. The MIR temp stayed i32 and the self-build emitted invalid
core Wasm.

Fix (landed in this lane): ship both modules in the overlay closure
(`scripts/selfhost/checks.py`, `BOOTSTRAP_STD_SOURCE_FILES` + the copy block).
With them, the committed pin's gc self-build validates:
core `66b86fbe3e9bf346264a5a5fcc6427d4d87113d3c0821f45b82c021cff84a9e9`
(8 503 709 bytes).

## Blocker 2 — untyped `sig` locals resolve to the wrong `len`

With blocker 1 fixed, the old pin's gc self-build still failed at
`find_use_site_sig_in_node` (func 10027):

```text
type mismatch: expected (ref null $type), found (ref null $type)
```

`src/compiler/analysis/symbols.ark` binds `let sig = use_site_signature(...)`
and then calls `len(sig)`. The unannotated local is typed `String` for the MIR
local, but the `len` overload resolved to `RawArray::len` on a `Vec` ref. A
`String` annotation on the three `sig` locals fixes the mismatch; the same
module's `use_site_signature` is unchanged.

## Blocker 3 — the faithful p1 s2 cannot self-build gc

The old pin cannot be its own producer: its gc build of the current source is
valid but **not faithful**. With blockers 1 and 2 fixed, the p1 s2 the old pin
produces (`ab460d53dfbc900716bb1babb2e5b65d3e430b77ad39041f6136f86baf67bed1`,
11 771 614 bytes) compiles `string_eq.ark` correctly on both targets, but the
same source compiled by the old pin to gc behaves differently:

| Artifact | sha256 |
|----------|--------|
| p1 s2 built by the old pin | `ab460d53…` |
| old-pin gc pin's p1 output | `277ad053…` |
| old-pin gc core | `66b86fbe…` |
| old-pin gc pin recompiling itself | `95feb482…` |

The p1 identity and gc self-reproduction contracts both fail, so the valid
`66b86fbe…` core cannot be adopted.

Using the faithful p1 s2 (`ab460d53…`) as the gc producer instead fails at
`try_emit_name_intrinsic_fallback` (func 5637):

```text
type mismatch: expected (ref null $type), found i32
```

after a `core_op_handler_text_to_string` call. That is a third latent
CoreOp-result/local-type consistency bug, independent of the overlay closure.

## Consequence

- No pin was committed. The committed pin remains `471855ad…`, which still
  fails `string_eq.ark` at clean HEAD.
- The two source fixes above are safe and verified: `selfhost build-compiler`
  with the committed pin takes 48 s and produces `ab460d53…`, and that s2
  compiles and runs `string_eq.ark` correctly on `wasm32-gc` and `wasm32`
  (31/31 output lines).
- `eq` retirement stays blocked. The eq-source stage-2 built by the committed
  pin (`e0664454…`) still emits invalid core Wasm for `string_eq.ark`, and the
  pre-eq pin built by the old pin fails the same fixture. The remaining failure
  is the eq path itself: an impl-based `String` equality whose operands come
  from inline calls is staged into an i32 temp instead of a String ref. The
  free `eq`'s declared `(String, String)` signature used to provide that
  staging; the `Eq` method path does not.

## Next step

Fix the gc MIR result/local typing for CoreOp fallbacks (start with
`try_emit_name_intrinsic_fallback`), then rebuild the pin from the p1 s2 and
re-run the p1 identity + gc self-reproduction checks before touching
`bootstrap/arukellt-selfhost.wasm`.
