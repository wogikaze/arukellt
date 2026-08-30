# Pinned-reference selfhost wasm — Provenance

This directory holds the **committed pinned-reference selfhost wasm** that is
the single trusted base for the four canonical selfhost gates (see
[ADR-029](../docs/adr/ADR-029-selfhost-native-verification-contract.md)).

The pinned wasm is the source-of-truth for bootstrapping on a fresh clone:
the four gates do **not** require the legacy Rust binary
(`target/debug/arukellt`) and do **not** call `cargo build`.

## Artifact

| Field | Value |
|-------|-------|
| Path | `bootstrap/arukellt-selfhost.wasm` |
| Size | 7 016 885 bytes (≈ 6.69 MiB) |
| sha256 | `ec200344fa39dfc78480abda54f026544722d89a169a8c43a5f28a3ca8586aca` |
| Built from commit | `9f91bac4` wasm32-gc dest-typing + clone(T) stack-arg (s3==s4) |
| Build target | `wasm32-gc` / `wasi-p2` (guest `(memory 8192)` **memory32**) |
| Producer | Host-linker pin→s2→s3 fixpoint (sha256 equal). Intermediate s3==s4 before refresh. Guest memory32 wasm32-gc / wasi-p2; FS via `arukellt:runtime/host@0.1.0`; do not --to-memory64 |

## Reproducibility recipe

The pinned wasm is the deterministic Stage-2 output of the selfhost compiler
when compiled from the recorded source commit. Gates use
`scripts/selfhost/checks.py` which copies this memory32 GC pin for host-linker
(no Memory64 widen). A minimal recipe:

```bash
# 1. Check out the recorded source commit (or tip that matches this pin)
git checkout <pin-commit>

# 2. Rebuild via the official gate (host-linker + wasm32-gc emit)
python3 scripts/manager.py selfhost fixpoint --build

# 3. Verify byte-for-byte identity with the pinned wasm
sha256sum bootstrap/arukellt-selfhost.wasm .build/selfhost/arukellt-s2.wasm
# ⇒ both sums must be identical.
sha256sum .build/selfhost/arukellt-s2.wasm .build/selfhost/arukellt-s3.wasm
# ⇒ identical sums = fixpoint reached
```

The `selfhost fixpoint` gate (`scripts/selfhost/checks.py::run_fixpoint`)
performs the bootstrap → s2 → s3 chain automatically.

## Refresh policy

The pinned wasm is **explicitly refreshed**, never auto-bumped. Refresh is
required when an intentional behavioural change in the selfhost compiler
(`src/compiler/**`) makes the four gates fail against the previous pinned
reference. Refresh procedure:

1. Locally bootstrap a new Stage-2 wasm from the previous pinned reference and
   the new compiler source (`python3 scripts/manager.py selfhost fixpoint --build`).
2. Verify the Stage-3 fixpoint holds (`s2 == s3`). If the refresh path needs an
   intermediate Stage-3 artifact, verify one more round (`s3 == s4`) and pin the
   stable fixpoint artifact.
3. Run the full fixture-parity gate against the previous pinned reference and
   review every difference. Document each behavioural drift in the refresh
   commit message; if any drift is unintentional, **do not refresh**.
4. Replace `bootstrap/arukellt-selfhost.wasm` with the new fixpoint binary,
   update this file's *sha256*, *size*, and *Built from commit* rows, and
   commit both changes in one commit titled
   `chore(bootstrap): refresh pinned selfhost wasm to <short-sha>`.

The refresh commit must be signed off by a maintainer and mention every
behavioural drift in its body.

### wasm32-gc pinned (`9f91bac4`)

Pinned bootstrap is native `wasm32-gc` / `wasi-p2` with guest memory32
(`(memory 8192)`). `BOOTSTRAP_EMIT_TARGET` / `BOOTSTRAP_EMIT_WASI_VERSION` in
`scripts/selfhost/checks.py` match. `_ensure_bootstrap_compiler_wasm` copies
the pin without `--to-memory64`. Execution uses `scripts/run/arukellt-run-hosted.sh`
(host-linker) for `wasi:cli/` and `arukellt:runtime/host@0.1.0` filesystem imports.

Intentional drift from the previous TypeSectionPlan pin (`53ce8aac` / `4f4b8992`):

- TypeSectionPlan `fn_result_gc_map` owns CALL dest types (tuple / enum / return)
- leftover String dests yield to copied named-struct sources
- `clone(T)->T` dests use the LOCAL_GET source, not the shared String plan result
- `Result<Vec<String>, E>` encodes as `result:vec:String:E`
- `get_unchecked(Vec<Struct>)` dests take the receiver element type
- Isolated T3: 459 pass / 0 validate-fail / 0 compile-fail / 23 skip
- Intermediate s3==s4 before refresh (`ec200344…`)

Intentional drift from the previous #834 pin (`4d2da710`):

- FS imports moved from `wasi:filesystem/types@0.2.0` + `arukellt:fs@0.1.0`
  to `arukellt:runtime/host@0.1.0` (host-linker binds the same P1-shaped impls)
- Extra runtime host func types shift GC array indices (was type 9, now 11)
- TypeSectionPlan is the only type-index owner (`name=0 fallback=0`)
- Vec push and `i32.to_string` no longer lower to `unreachable`
- Overlay `optimize_module` runs validated `gc_hint` at O2 (LICM/unroll stay stubbed)
- Ordinary structs emit `final`; enum base stays `open`

## Why this artifact is committed

The four selfhost gates (`fixpoint`, `fixture-parity`, `diag-parity`,
and CLI parity) start from this binary so a fresh clone can verify the
selfhost compiler without a prior build. See ADR-029 for the contract.
