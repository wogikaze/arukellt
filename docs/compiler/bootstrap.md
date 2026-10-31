# Selfhost Bootstrap (ADR-029)

> **Current contract.** Trusted base is the pinned selfhost wasm. Rust-era
> Stage 0 narratives live in
> [`../history/reports/bootstrap-rust-era-compiler-guide.md`](../history/reports/bootstrap-rust-era-compiler-guide.md).

Normative decision: [`../adr/ADR-029-selfhost-native-verification-contract.md`](../adr/ADR-029-selfhost-native-verification-contract.md).  
Status summary: [`../state/compiler.md`](../state/compiler.md).  
Structured SSOT: [`../data/bootstrap-contract.toml`](../data/bootstrap-contract.toml).

The official pinned bootstrap is a `wasm32-gc` / `wasi-p2` command component
(`bootstrap/arukellt-selfhost.wasm`), so the compiler itself runs on Wasmtime's
GC. It emits the stage-2/stage-3 compiler as `wasm32` / `wasi-p1` core modules
with memory32; the emitted compiler contract is unchanged. The core trust
anchor stays at `bootstrap/arukellt-selfhost-core.wasm` as an explicit
override. User-facing `wasm32-gc` / `wasi-p2` output is still packaged as an
official WASI component with the checked-in WIT world.

The core override pin predates generic enum payload lowering used by the
current `std/prelude.ark`. `selfhost build-compiler` keeps the structural
`Box` / `Slice` retry for that path. The checked-in prelude, user source
semantics, and emitted user artifacts are not rewritten.

The pin's builtin host modules only expose the free `std::host::*` functions,
so the flat overlay ships the namespace sources the compiler itself imports:
`std/host/fs.ark`, `std/host/clock.ark`, `std/host/process.ark`,
`std/host/stdio.ark`, and `std/io/mod.ark`. The checked-in `Fs::*`, `Clock::*`,
`Process::*`, `Stdio::*`, and `toml::TomlText::*` spellings therefore compile to
the real impls instead of the unresolved-call `drop; unreachable` fallback. The
overlay no longer rewrites those call spellings or installs
`bootstrap_toml_*` adapters.

The overlay does not ship `std/host/env.ark`, so the checked-in entry module's
`Env::args()` resolves only as `E0100 undefined module member` under every pin.
`_patch_bootstrap_main_host_to_prelude` therefore always routes `main.ark`
through the prelude `args` / `exit` / `println` host-intrinsic stubs. The core
trust-anchor/override pins (`a2803831…`, `8df2698c…`) additionally predate
#917's MIR env call-result typing, so `_patch_bootstrap_host_env_paths` keeps
the compiler's remaining `Env::*` spellings on the free builtin form. The gc
pin's `Env::*` typing is already #917-clean and that rewrite is
emission-neutral there (gating it off leaves s2 byte-identical), so it applies
only when a legacy core pin is selected. Delete both when `std/host/env.ark`
ships in the overlay and the selected pins resolve the checked-in entry module;
the function docstrings carry the probes that must validate the lowering first.

Overlay mode handling is pin-aware. The official gc pin parses and checks the
#951 parameter modes, so it compiles the overlay with the real modes:
#951/#943's call-site migration made the checked-in source and the generated
overlay stubs mode-clean, and the gc path no longer passes
`--bootstrap-allow-shared-param-mutation`. The legacy core
trust-anchor/override pins predate the mode syntax, so
`_strip_bootstrap_param_modes` removes the mode token only when one of those is
the selected pin; that stripped overlay is the only path that still uses the
permissive body-binding option (an s2 stage-3 build on the core fallback).
User programs, fixtures, and the analysis pipeline never pass the option and
keep the strict rule. Delete `_strip_bootstrap_param_modes` and the option when
the core pins are retired; probe with `fn bump(mut p: Point)` and a
shared-parameter assignment before removing them.

The isolated WIT parser closure in that overlay also normalizes qualified type
names (`types::WitType`, `ast::WitNode`, and related record types) to the flat
overlay namespace. Function calls remain qualified, and the checked-in `std/`
sources are not rewritten. This is a bootstrap namespace adapter only; it does
not relax the language type identity or make an s2 build equivalent to an
ADR-029 fixpoint.

### Pin-only overlay adapters

The `bc81d60b…` refresh retired the adapters that only the previous pin
generation needed. These are gone from `scripts/selfhost/checks.py`:

| Adapter | Evidence after the refresh |
|---------|----------------------------|
| numeric conversion bridge (`_patch_bootstrap_numeric_conversion_bridge`, `BOOTSTRAP_NUMERIC_*`) | the pin's CoreOp table knows the `__intrinsic_*` conversion keys; the candidate-generation probe left the stage-2 bytes unchanged |
| `__Printable` print boundary (`_patch_bootstrap_printable_boundary`) | the pin parses `String<__Printable>`; the candidate-generation probe left the stage-2 bytes unchanged |
| `Stdio::*` intrinsic rewrite (`_rewrite_bootstrap_stdio_calls`) | the pin lowers `Stdio::*` natively; removing it changed s2 `d2224704…` → `1d5056db…` and every gate passed |

The remaining adapters are still required; delete each only under its
condition:

| Adapter | Why it stays | Delete when |
|---------|--------------|-------------|
| shallow `Vec::clone` bridge (`_patch_bootstrap_prelude_vector_clone_shallow_text`) | the pin lowers `String::clone`'s helper to an unresolved-call stub; the rebuilt s2 traps at startup in `__clone_string_helper` | the pin's clone lowering compiles the compiler with `use std::core::clone`; probe by removing the bridge and running one compile with the rebuilt s2 |
| `len` / `get_unchecked` / `get` prelude shims (`BOOTSTRAP_PRELUDE_*_SHIM`) | the pin's method lowering still emits the bare retired free call; without the declarations stage 2 fails with `E0212 use of moved value rest_len` | the pin emits the intrinsic spelling from `mir_vec_method_prefer_coreop_name` instead of the bare free callee |
| `_patch_bootstrap_main_host_to_prelude` | the overlay does not ship `std/host/env.ark`, so a checked-in `Env::args()` is `E0100 undefined module member` | `std/host/env.ark` ships in the overlay and every supported pin resolves the checked-in entry module |
| `_patch_bootstrap_host_env_paths` | the legacy core pins cannot type a reachable `Env::args` result; the rewrite is emission-neutral under the gc pin (gated to legacy pins) | the core trust-anchor/override pins are retired |
| `_strip_bootstrap_param_modes` | the legacy core pins predate the #951 mode syntax (gated to legacy pins) | the core pins are retired, or every supported pin accepts modes |

## Which command?

Agents and humans confuse these two. Use the table:

| Goal | Command | Typical time | Notes |
|------|---------|--------------|-------|
| **Refresh the compiler after editing `src/compiler/**`** | `python3 scripts/manager.py selfhost build-compiler` | **~30–34 s** with the official gc component pin (AOT-cached, quiet host); 20–40 min with the core fallback pin | Stage-2 only. **Default for emitter / Memory64 / T3 work.** |
| Check ADR-029 fixpoint (`sha256(s2) == sha256(s3)`) | `python3 scripts/manager.py selfhost fixpoint` | seconds if s2/s3 exist; ~34 s to build s3 with the gc pin | Does not refresh the emitter by itself |
| Rebuild s2 **and** s3 then compare (gate only) | `python3 scripts/manager.py selfhost fixpoint --build` | ~60 s with the gc pin | **Not** for routine iteration. The ≤10 s gc-host target is [#851](../../issues/open/851-selfhost-compiler-core-rewrite.md) / [ADR-053](../adr/ADR-053-selfhost-compiler-core-rewrite.md) |

Aliases for `build-compiler`: `build-s2`, `rebuild-s2`.

Do **not** use `fixpoint --build --no-cache` to “just rebuild s2” — that also
runs stage-3 and floods long builds under parallel agents.

Copy files with `/bin/cp -f` (never interactive `cp -iv`).

### Bootstrap pin

`bootstrap/arukellt-selfhost.wasm` is the official `wasm32-gc` / `wasi-p2`
command component (8 334 637 bytes; sha256 `bc81d60b…`). It runs on Wasmtime's
GC and compiles the flat overlay through to the stage-2 `wasm32` / `wasi-p1`
compiler in ~30–34 s warm (quiet host):

```bash
python3 scripts/manager.py selfhost build-compiler
```

The component's p1 output is the fixed-point stage-2 s2 it was generated from
(`d2224704…`), and it reproduces its own gc core (`439f0dcb…`); those values
describe the pin artifact at the refresh commit. The shim prune changed the
current overlay, so the same pin now builds the fixed point `1d5056db…`
(11 296 972 B; s2 == s3). The Python compile path AOT-caches the component and
the gc path applies no permissive body binding. The previous core pin is
recoverable from commit `fb5944edb`
(see [`bootstrap/PROVENANCE.md`](../../bootstrap/PROVENANCE.md)); the 2026-10-19
core trust anchor is at `bootstrap/arukellt-selfhost-core.wasm` and can be
selected with `ARUKELLT_PINNED_WASM=bootstrap/arukellt-selfhost-core.wasm`
(20-40 minutes per build). The ADR-029 fixpoint is still required before a
tag. Provenance, the refresh procedure, and removal conditions:
[`bootstrap/PROVENANCE.md`](../../bootstrap/PROVENANCE.md).

### How to iterate without unnecessary rebuilds

`build-compiler` is a **full compile of the entire selfhost compiler**
(typecheck + MIR lower + wasm emit). Overlay cache hits only skip the flat-src
rewrite; they do **not** skip that compile.

**Do not rebuild once per one-line hypothesis.** That makes agents
latency-bound (`~32 s × N` tries).

Recommended loop:

1. Classify failures / read WAT with the **current** s2 (no rebuild).
2. Batch all planned `src/compiler/**` edits.
3. **One** `selfhost build-compiler` (~30–34 s with the gc pin, AOT-warm, quiet host).
4. Re-validate **many** fixtures against that s2. Use
   `selfhost fixture-parity --filter-dir <area>` while a batch is in flight,
   and `ARUKELLT_FIXTURE_WORKERS=12 verify fixtures` at a checkpoint.
5. Only rebuild again after the next batch of source edits.

Parallel agents must **share** one rebuilt s2 (parent rebuilds once); each lane
must not run its own `build-compiler`.

### wasm32 overlay compile: preopen the overlay only

The flat overlay copies a trimmed `std/prelude.ark` (it strips
`use std::collections::*`). Measuring or emitting a wasm32 compiler with
`--dir=<repo>` as well lets `std/prelude.ark` resolve to the full tree, which
pulls `impl String` methods. On wasm32 those methods rewrite `len`/`slice` and
the successor leaks (multi-GiB) during lower.

`scripts/selfhost/checks.py` `_wasm_compile` therefore preopens **only** the
overlay workspace when one is set. Manual timing must do the same:

```bash
wasmtime run --allow-precompiled --wasm gc --wasm function-references \
  -W memory64=y -W max-memory-size=17179869184 \
  --dir=.build/selfhost/flat-src \
  HOST.cwasm -- \
  compile src/compiler/main.ark --target wasm32 --wasi-version wasi-p1 \
  -o out.wasm
```

The bootstrap host emits a direct core module for the configured bootstrap
profile. A command component is packaged from standard P2 core imports with
the official `wasm-tools component embed` / `component new` commands; no
adapter or bridge is permitted.

### AOT image cache

`wasmtime run` JIT-compiles the compiler on every invocation (~5 s), so the
wrapper and the Python gates keep a precompiled image next to the compiler
wasm. The official gc component pin uses the same cache: the Python
`_compile_with_gc_pin` path AOT-compiles `bootstrap/arukellt-selfhost.wasm`
on first use and then runs the image with `--allow-precompiled`.

| Path | Role |
|------|------|
| `<compiler>.collector-copying.cwasm` | image produced by `wasmtime compile -C collector=copying` |
| `<compiler>.collector-copying.cwasm.tag` | the cache key the image was built under (one line) |
| `<compiler>.collector-copying.cwasm.lock` | `flock` target that serializes compiles |

The key is:

```
aot-cache-v2;collector=copying;wt=<wasmtime identity>;wasm=<wasm identity>
```

An identity is `<size>-<seconds>.<nanoseconds>` from `stat`. Those are the only
inputs that change the image: the wasm bytes, the Wasmtime binary (honouring
`ARUKELLT_WASMTIME_BIN`), and the collector flags. A rebuild rewrites the
wasm's size and/or nanosecond mtime, so the next invocation recompiles the
image; an untouched compiler keeps hitting the cache. Do not narrow the key
back to the Wasmtime identity alone (or to cwasm-mtime ordering): that is what
silently ran the previous compiler after a rebuild.

`scripts/run/arukellt-selfhost.sh` (`aot_cache_tag`) and
`scripts/selfhost/checks.py` (`_aot_cache_key`) must emit the identical string
because they share the one `.tag` file. Regression tests:
`python3 -m unittest scripts.tests.test_aot_cache`.

Invalidation is just deleting the sidecars:

```bash
rm -f .build/selfhost/arukellt-s2-runtime.collector-copying.cwasm*
```

## Trust model

| Stage | Artifact / check | Command |
|-------|------------------|---------|
| **0 (trust base)** | `bootstrap/arukellt-selfhost.wasm` (pinned; see `bootstrap/PROVENANCE.md`) | — |
| **Build current selfhost** | pinned compiles `src/compiler/main.ark` → `.build/selfhost/arukellt-s2.wasm` | `python3 scripts/manager.py selfhost build-compiler` |
| **Fixpoint** | `sha256(s2) == sha256(s3)` | `python3 scripts/manager.py selfhost fixpoint` |
| **Verification** | current-only fixture test / CLI / diag | `python3 scripts/manager.py selfhost fixture-parity`, `… parity --mode --cli`, `… diag-parity` |

Stage-3 / runtime compiler wasms are validated with `wasm-tools validate` after
build (and when reused). An invalid artifact — for example Memory64 GC output
that does `struct.set` of an i32 field without `i32.wrap_i64` — is deleted and
must not remain as a selectable `arukellt-s3.wasm`. The day-to-day wrapper
prefers `.build/selfhost/arukellt-s2-runtime.wasm` over s3 for this reason.

Stage 0 is **the pinned wasm**. The selfhost launcher has no Rust compiler
fallback. `ARUKELLT_USE_RUST=1` is rejected by the launcher.

## User-facing entrypoint

```bash
# Preferred: wrapper resolves pinned / s2 / env override
scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark --target wasm32-gc

# Or point ARUKELLT_SELFHOST_WASM at a freshly built s2 for library component work
ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm \
  scripts/run/arukellt-selfhost.sh compile lib.ark --target wasm32-gc --emit component
```

Resolution order (wrapper `scripts/run/arukellt-selfhost.sh`, with each
existing candidate rejected if it uses a retired ABI):
`$ARUKELLT_SELFHOST_WASM` → `bootstrap/arukellt-selfhost.wasm` →
`.build/selfhost/arukellt-s2-runtime.wasm` → `.build/selfhost/arukellt-s3.wasm` →
`.build/selfhost/arukellt-s2.wasm` → `.bootstrap-build/arukellt-s2.wasm` →
`.build/selfhost/arukellt-pinned-bootstrap.wasm`.

## Artifact policy

| Path | Status |
|------|--------|
| Legacy P1-shaped P2 core imports | Rejected; no compatibility adapter |
| Repository-specific host imports | Rejected; use official WASI interfaces |
| Rust compiler fallback | Removed from the product and verification paths |

Historical walkthroughs of the old Rust→s1→s2 script remain under `docs/history/reports/`.

## CI

Bootstrap evidence runs in the **`selfhost`** job of `.github/workflows/ci.yml`
(fixpoint + parity). Do not invent a `verification-bootstrap` job name.
