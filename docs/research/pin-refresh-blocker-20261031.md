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
2. **v2 ownership migration**: the one `mir_rewrite_vec_push` E0212 is fixed;
   the rest of the source is not `mut`-parameter-clean under the v2 checker, so
   the overlay mode strip stays.
3. **Pin generation**: `56ff0808…` remains the official pin. The rebased
   `8df2698c…` core pin is superseded and must not be installed.
4. **Shims**: all listed in `docs/compiler/bootstrap.md` and
   `scripts/selfhost/checks.py` remain; their deletion conditions are unchanged.
