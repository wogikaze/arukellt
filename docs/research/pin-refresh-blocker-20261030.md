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

## Resolution — CoreOp accessor result typing (same day)

The third blocker was not a missing result type on the `*_to_string` CoreOp
handlers. The failing source is a comparison, not a handler: the generated
core-op handler accessors (`fn core_op_handler_text_to_string() -> i32`) return
the i32 handler id, and a name heuristic in the lowerer retyped the accessor
call result as `String`.

`mir/lower/core_call_arg_names.ark::mir_core_call_string_result_name` treats
every callee whose name ends with `_to_string` / `_from_string` /
`_from_bytes` / `_encode` as String-returning. That table describes the removed
CoreOp impl symbols (`raw_f64_to_string`, `__core_i32_to_string_impl`), which
have no declaration left, but it also matches the live accessor.
`mark_string_call_result_type` then forced the call destination to
`string` / `VT_GC_REF`, so
`handler_id == core_op_registry_generated::core_op_handler_text_to_string()`
lowered as string equality. From the invalid gc self-build
(`s2pin-invalid.wat`, func 5637):

```wat
local.get $handler_id
call $core_op_handler_text_to_string
local.set 780                   ;; (ref null 0) = string byte array
local.set 779                   ;; (ref null 0) = string byte array
i32.const 0
local.set 767
local.get 779
array.len
```

`local.set 780` stores the i32 handler id into a string-ref local — the
reported `expected (ref null $type), found i32`. The same shape appears at
every `handler == core_op_handler_*_to_string()` site (`call_text.ark`,
`call_runtime_io.ark`, `native_c/core_call.ark`).

Fix: `mir_core_call_callee_returns_string(ctx, name)` applies the spelling
table only when `ctx_fn_return_type_name` finds no declaration; a live
declaration owns its return type. All four call sites use it — call-result
typing (`call_types.ark`), both print-argument conversion paths, and core-let
binding typing (`core_bindings_type.ark`). The regression fixture is
`tests/fixtures/regression/coreop_accessor_result_type.ark`.

## Pin status after the fix

`selfhost build-compiler` with the committed pin produces s2 `33cfe196…`
(11 802 634 bytes). That s2 compiles the overlay to `wasm32-gc` and the core
now validates:

| Field | Value |
|-------|-------|
| gc core from the faithful p1 s2 | `db883d234df2cc871011ddf05b71bd7943014cd6a36287493b597346bc50ec37`, 8 523 650 bytes |
| wall / RSS | 37 m 50 s / 1.51 GiB |
| packaged component | 8 531 415 bytes, `64841c84…` |
| gc self-reproduction | PASS — the packaged component recompiles the overlay to the same core in 28 s |
| p1 identity | **FAIL** — the packaged component emits `99961a5e…` (11 803 476 bytes), not the s2 `33cfe196…` it was built from |

The p1 difference is +842 bytes confined to the code section (types, imports,
functions, exports, data, and name sections are identical). It is not caused by
this fix: the old pin and the fixed s2 emit byte-identical cores for
`string_eq.ark` (`e22cacbe…`), `bench_parse_tree_distance.ark` (`afcf4c99…`),
and the new regression fixture (`e8e46455…`); the fix only removes a path the
old pin never took (it lowers the free `ends_with` spelling in
`mir_core_call_string_result_name`, whose suffix branch does not fire in the
pin binary — both compilers already emit `i32.eq` there). The drift is the
accumulated emission change of the 39 `src/compiler` commits between the pin's
build source (`36f42b97e`) and HEAD, so the pin was **not** refreshed.

Next step for a refresh: treat the candidate as a new generation and converge
it — rebuild s2 with the packaged component (writes `99961a5e…`), regenerate
the gc core from *that* s2, package again, and require the second pin to
reproduce its own p1 s2 (`99961a5e…`) and its own gc core. That is a two-step
refresh because the committed pin's generation is 39 compiler commits stale,
not a one-step host swap. `bootstrap/arukellt-selfhost.wasm` stays `471855ad…`
until that convergence is reviewed.

## Resolution — generation convergence (2026-10-30)

The refresh converged on `wave/pin-converge` (base `96a66db83`). Two things
changed relative to the measurement above:

1. The base is not `93582a23e`. `96a66db83` merges it with `9e8f2ec66`, which
   carries `9ca742d8c` (re-landed polymorphic `.clone()`, 796 files) and
   `22071e894` (HashMap wide-literal narrowing). The report's `33cfe196…` /
   `99961a5e…` therefore do not apply to this tree; the chain was re-derived.
2. The three-way merge in `96a66db83` kept `93582a23e`'s pre-clone versions of
   eight MIR lowerer files, losing `9ca742d8c`'s clone migration (215 bare
   `clone(x)` calls). The hard-coded `mir_lower_ctx_mono_type_params.ark`
   bootstrap stub also still emitted 162 bare `clone(x)` calls into the flat
   overlay. Against the merged prelude's generic `clone<T>` shim, the gc
   self-build resolved those calls to unrelated `Vec__clone__…` mono instances
   and failed validation at func 7686
   (`mir_should_wrap_i64_arg_to_i32`: `type mismatch: expected (ref null
   $type), found (ref null $type)`). The p1 stage-2 built fine; only the gc
   self-build was affected.

`ba4aa88b1` restores the clone migration in the eight lowerer files (keeping
the CoreOp typing fix and HashMap narrowing) and `ff4ff6a4f` migrates the stub.
The regenerated overlay then has no bare `clone(` calls.

Converged chain on the fixed source `ff4ff6a4f`:

| Generation | Producer | Artifact | sha256 | Size |
|------------|----------|----------|--------|------|
| committed pin | — | `471855ad…` | `471855ad9db035c1ada136852fc55e896b477cf35303be066f352d9d563a8895` | 8 550 341 |
| gen-1 s2 | committed pin | `e8a0bdb1…` | `e8a0bdb135aadc426237b81b7d4939ece050d43bed6e4ddde0b756849e41bf5e` | 11 816 406 |
| gen-2 s2 (fixed point) | gen-1 s2 | `61aa349b…` | `61aa349b377432773354e633182648becd8da9c97343e53bbe57cf192b73c62d` | 11 817 385 |
| gc self-build | gen-2 s2 | `0ebeb365…` | `0ebeb365d2833ae6850ff0482b62a20fbd4298de8001f3d94cc812a817a2fd98` | 8 532 895 |
| new pin component | `wasm-tools component embed/new` | `56ff0808…` | `56ff0808171dcc3c521b0e85d07f2b16a349a59419483ad71148a5ee928d23e8` | 8 540 660 |

Acceptance, all measured with `ARUKELLT_BUILD_DIR=$PWD/.build-lane` and
Wasmtime 46.0.1:

| Contract | Result |
|----------|--------|
| p1 identity | PASS — the component compiles the overlay to `61aa349b…`, byte-identical to the gen-2 s2 it was packaged from |
| gc self-reproduction | PASS — the component rebuilds core `0ebeb365…` in 30.5 s |
| ADR-029 fixpoint | PASS — `s2 == s3 == 61aa349b…` |
| warm `build-compiler` | 51.4 s at host load ~13 (quiet floor ~30 s) |

Generation count: **2** s2 generations (`e8a0bdb1…` → `61aa349b…`), plus the
packaged component. The committed pin is `56ff0808…`; see
[`bootstrap/PROVENANCE.md`](../../bootstrap/PROVENANCE.md) for the refresh
procedure and the fixed-point contract.
