# Pin refresh blocked on the wasm32-gc self-build (2026-10-31)

Tracking: ADR-029 / #851 / #917. This note records why
`bootstrap/arukellt-selfhost.wasm` was **not** refreshed on 2026-10-31 and why
the pin-only overlay shims were **not** deleted. It is evidence for the next
attempt, not a decision.

Lane: `.worktrees/wave-pin-refresh` (`wave/pin-refresh`), base `3cc72ee80`,
`ARUKELLT_BUILD_DIR=$PWD/.build-lane`, Wasmtime 46.0.1, wasm-tools 1.252.0.

## Starting point

`f1d163200` (pin `8df2698c…`) was rebased onto `3cc72ee80` as `b619cfb75`.
`8df2698c…` is the 2026-10-27 **direct core** pin built from `be7f6d3d9`.
`be7f6d3d9` is an ancestor of the current pin's build source `ff4ff6a4f`, so the
rebased pin is older than the committed `wasm32-gc` component `56ff0808…`
(built from `ff4ff6a4f`, 2026-10-30). Adopting it would replace the gc host with
an older direct core and restore 20–40 minute builds; it cannot delete any of
the shims the current pin needs. It was **not** installed.

The old pin was kept for the refresh attempt:
`56ff0808171dcc3c521b0e85d07f2b16a349a59419483ad71148a5ee928d23e8`.

## Candidate 1 — the pin's own gc build of HEAD

```
python3 .build-lane/scratch/build_gc_component.py bootstrap/arukellt-selfhost.wasm pin
```

The official pin compiled the HEAD overlay to `wasm32-gc --emit component` in
33.5 s:

| Artifact | Value |
|----------|-------|
| gc core | `b76d03b7eab4bce1b838a92dcfa090d550305d0eef93da2f1c141ee016d44f70`, 8 304 228 B |
| packaged component | `162a3ba4cab67dbe405aefd2a86778936a811a8875d5c520af977d79a12e2ad0`, 8 311 993 B |

The core validates, but the component is **not a faithful producer**. Running
it as the pin (`ARUKELLT_PINNED_WASM=…pin.component.wasm selfhost build-compiler`)
fails while compiling the overlay:

```text
error[E0212|typecheck]: use of moved value `callee_name`; v2 uses move semantics;
call `.clone()` before moving or borrow instead
```

A temporary diagnostic that appended `env.current_fn_name` to the message
identified the site as `mir_rewrite_vec_push`:
`src/compiler/mir/lower/call_rewrite_vec_push.ark` moves `callee_name` into
`rewritten` and uses it again for the early return. The prescribed fix is one
`.clone()` on the initializer; it is behavior-neutral and is committed
separately as a prerequisite (see the commit that touches
`call_rewrite_vec_push.ark`).

## Candidate 2 — the faithful p1 s2 builds gc

With the E0212 fix, the official pin still builds the p1 stage-2:

| Artifact | Value |
|----------|-------|
| p1 s2 built by the official pin | `d2e25604b0b609a892d3075758be43b397c753f5bac8786ff423325e66d22741`, 11 270 746 B |
| validate | PASS |

That s2 was then used as the gc producer (the documented
generation-converged refresh path):

```
python3 .build-lane/scratch/build_gc_component.py .build-lane/scratch/s2-s1.wasm s1
```

The 32.4-minute compile finished, but the emitted gc core is **invalid**:

| Field | Value |
|-------|-------|
| gc core | `35c8ec36d55616e59882e2ae871cbf94975d79b689951e9d0d514241f784cd45`, 8 308 310 B |
| `wasm-tools validate` | FAIL — `func 1516 failed to validate: type mismatch: expected (ref null $type) but nothing on stack (at offset 0xa007f)` |
| component packaging | FAIL — the invalid core cannot be encoded |

This is not the old pin's unfaithfulness: the producer here is the p1 stage-2
that the fixture gate runs. The current compiler's gc lowering (or the gc
self-build of the current source) is the blocker.

The E0212 fix alone is also insufficient: the candidate built from the old
pin's gc output in this lane emitted an invalid **p1** core (`func 1509`,
`expected i32 but nothing on stack`), so that producer cannot be adopted
either.

## Why the overlay still cannot be unwound

`typechecker/contract_env_record.ark` initializes
`is_ownership_tracking_enabled: true`, so every self-compile enforces ADR-059
move semantics and the ADR-088 parameter-mode rules. The checked-in compiler
source is still v1-style:

- With the mode strip removed and the E0212 fix applied, the current compiler
  reports many `E0223 cannot pass an immutable place to a mut parameter` and
  `E0224 the same place env is passed to a mut parameter and another parameter`
  diagnostics. `_strip_bootstrap_param_modes` therefore must stay until the
  #951 / #943 migration lands.
- `_patch_bootstrap_host_env_paths`, `_patch_bootstrap_main_host_to_prelude`,
  `_rewrite_bootstrap_stdio_calls`, `_patch_bootstrap_printable_boundary`,
  `_patch_bootstrap_numeric_conversion_bridge`, the three prelude shims, and
  `_patch_bootstrap_prelude_vector_clone_shallow_text` are all still required by
  the committed `56ff0808…` pin, whose build source is 99 commits behind HEAD.
  No shim was deleted because no new pin was installed.

The gc self-build failure at `func 1516` is the hard blocker. Once it is fixed,
the refresh procedure in `bootstrap/PROVENANCE.md` applies unchanged: converge
the p1 s2, package the gc self-build of the fixed point, require p1 identity and
gc self-reproduction, then delete the shims that the new generation no longer
needs.

## Reproduction

```bash
export ARUKELLT_BUILD_DIR=$PWD/.build-lane
python3 scripts/manager.py selfhost build-compiler          # p1 s2 d2e25604…, PASS
# gc self-build of that s2:
python3 .build-lane/scratch/build_gc_component.py \
  .build-lane/scratch/s2-s1.wasm s1                          # invalid core, FAIL
```

The scratch helpers are lane artifacts (`.build-lane/` is ignored); the
authoritative commands remain `selfhost build-compiler`, `selfhost fixpoint`,
`verify fixtures`, `selfhost diag-parity`, and the component packaging recipe
in `bootstrap/PROVENANCE.md`.

## Residuals

1. **gc self-build invalid core** (`func 1516`,
   `expected (ref null $type) but nothing on stack`): the refresh blocker.
2. **v2 ownership migration**: resolved later on 2026-10-31
   (`wave/ownership-mut-migrate`). The call-site migration (8 real-source
   E0223 + 2 E0224 + 1 E0207, plus the 23 E0223 / 4 E0226 in the generated
   overlay stubs) leaves the s2 overlay check clean, and the mode strip is
   now pin-aware: it runs only for the legacy core pins, so the gc pin builds
   the real modes without the permissive body-binding flag. The gc
   self-build/reproduction blocker is still residual 1.
3. **Pin generation**: `56ff0808…` remains the official pin. The rebased
   `8df2698c…` core pin is superseded and must not be installed.
4. **Shims**: the mode strip is now pin-aware (gc path keeps the real modes);
   the other shims listed in `docs/compiler/bootstrap.md` and
   `scripts/selfhost/checks.py` remain, with their deletion conditions
   unchanged.

## Update 2026-10-31 — second blocker (func 7188) cleared

Lane `.worktrees/wave-gc-selfbuild-7188` (`wave/gc-selfbuild-7188`), base
`8a8142bd0`, commit `d1ccdbcfa`, same toolchain.

After `9ebd82899` the gc self-build passed func 1516 and failed at func 7188
(`edge_collect_calls`) with `type mismatch: values remaining on stack at end of
block @0x2d06e5`. The site was
`local.get $ta; local.get $ta; call $corehir_node_text`; the pin emits one
`local.get $ta`.

The extra load was **not** the staged call-argument reload: `corehir_node_text`
takes one argument, so that call is not staged (`child_count > 2` / printlike
only), and a stage-2 built from the pre-`9ebd82899` parent (`aefd33641`) emits
the same double load. The duplicate came from `mir_emit_clone_identity`
(`src/compiler/mir/lower/method_emit.ark`): it lowered the identity `clone` to
a `LOCAL_GET source -> dest` copy, but the method-call lowering had already
pushed the receiver operand, so `f(x.clone())` left two copies of `x` and the
call popped one. String receivers use the emitter's clone intrinsic and were
unaffected; a concrete struct receiver (`AstNode`) took the lowerer path.

The fix emits the clone CoreOp (`__intrinsic_string_clone`) as a stack call
again: the #928 constraint holds (the bare `clone` name is not resolved
through the shared cache) and the emitter store policy chooses SET/TEE/SKIP
from the enclosing consumer. `tests/fixtures/clone_no_import/
clone_call_arg_in_block.ark` is the regression fixture: it fails validation on
both targets before the fix (`values remaining on stack at end of block`) and
passes after it.

Validation evidence with `ARUKELLT_BUILD_DIR=$PWD/.build-lane`:

| Gate | Result |
|------|--------|
| `selfhost build-compiler` | PASS, s2 `60a7b9c9…` |
| `verify fixtures` | PASS=1970 FAIL=0 (43 test PASS) |
| `verify lane` | PASS |
| `docs check` | 6/6 PASS |
| `selfhost diag-parity` | PASS |
| `selfhost fixpoint` | PASS, s2 == s3 `60a7b9c9…` |
| gc self-build of the overlay | rc=0, 2242 s, core 8 308 553 B `b49a8341…`, `wasm-tools validate` PASS, component 8 316 318 B `83021916…` |

The new core's `$edge_collect_calls` has the pin's single
`local.get $ta; call $corehir_node_text`, and the whole core has no
`local.get X; local.get X; call` pair (the invalid core had 15).

## Pin refresh status after func 7188

The validation blocker is cleared: the gc core validates and packages. The
refresh is **not** yet adoptable, because the accelerator fixed-point contract
(`bootstrap/PROVENANCE.md` step 3) did not hold in the lane probe:

- fixed-point s2: `60a7b9c9…`, 11 271 157 B;
- the component built from it compiled the overlay to
  `--target wasm32 --wasi-version wasi-p1` as `6d19759e…`, 11 281 967 B
  (+10 810 B), and the same component used as a pin produced that same
  `6d19759e…` s2 (build-compiler PASS, 54.5 s).

Caveat: the lane helper `.build-lane/scratch/build_gc_component.py` still
passes the legacy `--bootstrap-allow-shared-param-mutation`, which
`bootstrap/PROVENANCE.md` reserves for the stripped legacy core path
(`typechecker/body_params.ark` treats every parameter as mutable under it).
The next step is the canonical no-flag self-build of the fixed-point s2. If
p1 identity still differs, the next diagnosis is a gc-host vs p1-host
emission difference, not the func 7188 lowering.

## Update 2026-10-31 — identity resolved; refresh blocked on a converged-generation miscompile

Lane `.worktrees/wave-pin-refresh-final` (`wave/pin-refresh-final`), base
`704641246`, same toolchain. The recorded accelerator-contract "failure" does
not reproduce as a contract failure: the p1 identity and gc self-reproduction
both hold at the generation-converged fixed point. The refresh is nevertheless
**not adopted**, because the converged compiler miscompiles itself: it lowers
the retired free `len` callee in the compiler source to the unresolved-call
stub, so the built stage-2 traps in `generic_inst_args` and 290 fixtures fail.

### The recorded failure was a generation hop

The gc component built with producer `60a7b9c9…` is the gc image of the **next**
generation. Its p1 output `6d19759e…` is `s2_2`, and the component used as a pin
reproduces `6d19759e…`, so `6d19759e…` was the converged generation. The lane
compared that p1 output with the producer `60a7b9c9…` (generation N) and read
the 10 810-byte generation delta as an accelerator-contract failure.

Evidence:

- Compiling the overlay with and without the legacy
  `--bootstrap-allow-shared-param-mutation` writes byte-identical p1 output
  (`9a515653…` on the `704641246` overlay): the flag is emission-neutral. The
  lane helper's leftover flag was non-canonical but not the cause.
- The 113-function p1 WAT delta (`60a7b9c9…` vs `6d19759e…`) is confined to
  string/printing functions and is the emitter change between compiler
  generations, not a gc-host vs p1-host difference.
- The gc host is faithful to the p1 host: the p1 stage-2 `d6a52c97…` and its gc
  image (component `c1`, `2be61406…`) emit byte-identical p1 output for
  `docs/examples/hello.ark` (`c8e35c88…`) and for the generic fixture
  `stdlib_sort/sort_custom_ord.ark` (`5c2dd6e7…`).
- The canonical p1 hop confirms the chain without the accelerator: the p1
  stage-2 `d6a52c97…` compiling the flat overlay to `wasm32` / `wasi-p1` writes
  `9a515653…` (11 295 250 B, 2 111 s), byte-identical to the gc image's p1
  output. The generation chain is therefore canonical, not a host artifact.

### Accelerator contract at 704641246 (passes)

| Step | Value |
|------|-------|
| convergence (pre-shim overlay) | `d6a52c97…` → `9a515653…` → `9a515653…` |
| candidate pin | `dd71ff59…`, 8 275 828 B; core `5a9ff9e9…`, 8 268 063 B |
| p1 identity | PASS — the candidate's p1 output is `da613c3a…` (11 279 112 B), its converged generation |
| gc self-reproduction | PASS — the candidate's gc core is `5a9ff9e9…` and repackages to `dd71ff59…` |
| `build-compiler` / `fixpoint --build` / `diag-parity` | PASS at the candidate fixed point |

### New blocker: the converged generation stubs `len`

Every converged generation (`9a515653…`, `17d7764f…`, `da613c3a…` — with
and without the shim deletions) contains unresolved-call stubs
(`local.get <arg>; drop; unreachable`) for the retired free `len` callee in the
compiler's own monomorphization code:

- `generic_inst_args` (`ctx_struct_mono.ark`: `let n = len(name)`,
  `let m = len(inner)`),
- `ctx_resolve_angle_type_vars`, `ctx_resolve_fnsig_type_vars`,
  `ctx_collect_type_var_names`, `ctx_push_fnsig_var_segments`,
  `ctx_assoc_mono_next_names` (`.len()` method calls, which the lowering emits
  as the bare `len` callee), and
- `removed_csv_member_hint` (`resolver/removed_names.ark`).

The WAT diff between the last good-emitted generation `d6a52c97…` and the next
one `9a515653…` shows these seven functions as the only ones gaining an
`unreachable` instruction (one each; 2 038 vs 2 045 in the whole module).

Consequences with the converged s2 `da613c3a…` (after the `len` workaround
below, `python3 scripts/manager.py verify fixtures`):

```
[fixture-test] test: PASS=33 FAIL=10
[fixture-test] PASS=1681 FAIL=290 COMPILE_VALIDATE=1971 EXECUTED=94 RUNTIME_UNSUPPORTED=2
```

The failures trap the compiler in `generic_inst_args` (for example
`stdlib_sort/sort_custom_ord.ark`, backtrace `generic_inst_args →
generic_inst_args_for_param_count → ctx_derive_generic_struct_layout`). The same
stubs fail two documentation examples:
`adr/ADR-086-linear-collection-adts.md` block 0 and
`stdlib/migration-guidance.md` block 16. The p1 stage-2 and the gc pin trap on
the same examples, so this is not a gc-host issue.

The base tree is unaffected because its stage-2 is emitted by the older
`56ff0808…` pin. The regression only appears when a compiler generation built
from the current source compiles the current source, which is exactly what a pin
refresh converges to.

Likely cause: `9a8616413` (entry bodies by declaration identity) made `len` and
`pop` lower from the checker's recorded `FunctionId` instead of recomposing the
owner-qualified spelling. `mir_rewrite_vec_len` returns the recorded id only
when it is exactly `String::len` or `Vec::len`; otherwise it falls back to the
owner (`String` / `Vec`) and finally to the original callee. The flat overlay's
`len` is the free prelude declaration added by `BOOTSTRAP_PRELUDE_LEN_SHIM`
(`pub fn len(s: String)`), not an impl method: its recorded id is `len`, the
owner is empty, and the bare `len` callee that the method lowering still emits
for untracked String receivers is returned unchanged. The emitter has no emitted
`len` declaration to bind (the previous generation inlines the shim body,
`raw_string_len_bytes`), so it emits `drop; unreachable`. The seven functions
appear to be the ones where a `.len()` receiver comes from a `.clone()` /
`.slice()` chain whose owner is not tracked. Fix direction: map the bootstrap overlay's
bare `len` to the `String::len` intrinsic (or make the checker record the shim's
receiver owner), then re-run the generation convergence.

### Workaround tested (not installed)

`resolver/removed_names.ark`:
`name.clone().slice(4, len(name))` → `name.clone().slice(4, name.len())`
removes the `removed_csv_member_hint` stub and makes `selfhost diag-parity` PASS
at the converged generation, but leaves the six monomorphization stubs and the
fixture failures. The overlay prelude declares `len` and the typechecker accepts
the call, so the missing piece is the emitter binding of the bare `len` callee
for loaded-module bodies (see the likely cause above). That is the next unit of
work.

`bootstrap/arukellt-selfhost.wasm` was restored to `56ff0808…`; no pin was
installed and no overlay shim was deleted in the committed tree. The shim
deletions tested in the lane (numeric-conversion bridge, printable boundary,
stdio rewrite) are emission-neutral or benign at the candidate generation and
can be re-applied once the generation is faithful.

### Residuals

1. **Converged-generation `len` stub**: the blocker above. Affected functions
   listed; fixtures 290 fail, diag-parity 2 fail (one fixed by the workaround).
2. **Shallow `Vec::clone` bridge and the three prelude shims** are still
   required. Removing the clone bridge produces an s2 that traps in
   `__clone_string_helper`; removing the `.len()` / `.get_unchecked()` / `.get()`
   declarations fails the stage-2 build with `E0212 use of moved value
   rest_len` in `mir/lower/call_mono.ark`.
3. **`Env::*` / entry-module host rewrites** stay gated on the legacy core pins.

## Update 2026-10-31 — `len` stub fixed; refresh blocked by a gc clone misresolution

Lane `.worktrees/wave-len-shim-bootstrap` (`wave/len-shim-bootstrap`), base
`5ff36769d`, commit `56fedbcd8`, `ARUKELLT_BUILD_DIR=$PWD/.build-lane`, same
toolchain (Wasmtime 46.0.1, wasm-tools 1.252.0).

### The fix

The seven sites all reached `mir_rewrite_vec_len` as the bare `len` callee with
the prelude shim's FunctionId (`fid 69`). That declaration has no MIR body in
the overlay, so no emitted function exists to bind, and the emitter's name
fallback matches owner-qualified spellings only (`::len`); the bare callee
therefore lowered to `drop; unreachable`. The checker's per-call record is
present for the working neighbours and absent exactly when it could not type
the receiver (a `String` / `Vec` produced by `.slice(...)` or another untyped
call): the MIR receiver local still carries `String`, `string` or `vec:String`.

`mir_rewrite_vec_len` now takes the receiver local index and, after the
recorded FunctionId / owner lookups, recovers the owner from the receiver local
spelling (`Vec` / `vec:` / `array:` -> `Vec::len`; `String` / `string` ->
`String::len`) for the prelude `len` spellings only. A mangled `len__*`
instance keeps its bound declaration, and no emitter-side bare-name resolution
was added, so #917 / #928 are unchanged for user code. A `test mod` pins the
name-set guard.

Cheap probe (producer = the stage-2 s2, entry importing the seven affected
modules): 7 bare `callee=len` -> 0, the emitted module loses exactly the seven
`drop; unreachable` stubs, and the sites now emit `String::len` / `Vec::len`
with the same FunctionIds as their already-working neighbours.

### Convergence

| Step | Value |
|------|-------|
| s2_1 = committed pin(source) | `ef67953636f9528a18641c30194199382876b0ada08015741cd90337b9f099d8`, 11 305 681 B |
| s2_2 = s2_1(source), 2 413.6 s | `8e01d11c904260dca0cb320f29a72ee3eea41df363d99a33428032316e800936`, 11 316 609 B |
| s2_3 = s2_2(source), 2 209.4 s | `8e01d11c…` — fixed point |
| s2_2's seven functions | `unreachable` 0,0,0,2,0,1,0 (was 1,1,1,3,1,2,1); the 2+1 are `Vec` bounds traps (`i32.ge_u; if unreachable`), not unresolved calls |

The p1 generation is converged and faithful: the converged generation no longer
stubs `len`. Gates against the current pin: `verify fixtures` PASS (1974
compile+validate, 43/43 tests), `verify lane` PASS, `docs check` 6/6,
`selfhost diag-parity` PASS, `selfhost fixpoint` PASS (`s2 == s3 ==
ef679536…`).

`docs check` needs the doc-example gate pointed at the lane s2 through
`ARUKELLT_SELFHOST_WASM`; with the committed pin the ADR-086 block 0 example
fails validation (the pin miscompiles it, while the fixed s2 compiles it
byte-identically to the pre-fix s2).

### New blocker: the gc self-build misresolves `Vec::clone`

The gc self-build of the fixed-point s2 is invalid, so the candidate pin cannot
be packaged and no pin was installed:

| Field | Value |
|-------|-------|
| producer | s2_2 `8e01d11c…` |
| gc core | `229f7133a0da443d54f411c05e753842b49bccb9102b0f819f47128b93b3f531`, 8 330 327 B |
| `wasm-tools validate` | FAIL — `func 1372 failed to validate: type mismatch: expected (ref null $type), found (ref null $type) (at offset 0x96a91)` |
| component packaging | FAIL (invalid core) |
| failing instruction | `local.get $declared; call $Vec__clone__TypeInfo` in `resolver_impl_target__impl_target_type_name`, where `declared: Vec<String>` |

The same producer's **p1** output calls `Vec__clone__String` at all three
`declared.clone()` sites, so the p1 chain validates and hides the defect; the
gc target exposes the wrong instance through ref types. The gc core has 23
`Vec__clone__TypeInfo` call sites; the wrong one is the second `declared.clone()`
in the function. This is the instantiation-blind clone-instance selection (the
name-rewrite pool picks the first unused `Vec__clone__*` instance) surfacing
when the gc target mints the `Vec<TypeInfo>` instance, not a `len` regression:
the pre-fix s2 at HEAD (`bbc1f2b4…`, the `len` fix reverted) builds gc core
`0f8be83a…`, which fails at the *same* call (`func 1372`, offset `0x96a8f`,
`call $Vec__clone__TypeInfo`). The failure is therefore present at HEAD without
the `len` fix and is the next refresh blocker.

### Residuals after this lane

1. **gc clone misresolution** (the blocker above): fix the clone instance
   selection (or the span-based mono lookup), re-run the gc self-build, then
   the candidate p1 identity / gc self-reproduction / fixture contract.
2. **Pin**: `56ff0808…` remains the official pin. The `len` fix is committed on
   `wave/len-shim-bootstrap` (`56fedbcd8`), but the pin was not refreshed.
3. **Shims**: unchanged. The prelude `len` shim stays because the compiler
   source still calls the free `len`; the overlay still needs the declaration
   after a pin refresh.
