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
| Size | 7 264 783 bytes (≈ 6.93 MiB) |
| sha256 | `f0e7abb0c813d3c4d7cdd5f4a1341de5aa53eca2fd3b0ac0158bed5e255e4c0e` |
| Built from commit | `ca1e5621` wasm32-gc library emit uses GC canonical ABI adapter (s2==s3) |
| Build target | `wasm32-gc` / `wasi-p2` (guest `(memory 8192)` **memory32**) |
| Producer | Host-linker pin→s2→s3 fixpoint (sha256 equal). Guest memory32 wasm32-gc / wasi-p2; FS via `arukellt:runtime/host@0.1.0`; do not --to-memory64 |

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

### wasm32-gc pinned (`ca1e5621`)

Pinned bootstrap is the s2==s3 fixpoint of `ca1e5621` (wasm32-gc library
`--emit component` uses `wasm::emit_library_component` / GC ↔ canonical
ABI adapter). Guest remains `wasm32-gc` / `wasi-p2` memory32. Official
`build-compiler` then `fixpoint --build` 2026-08-31T08:19–08:29Z:
s2==s3=`f0e7abb0` EXIT 0.

Intentional drift from the previous pin (`9a821f07` / `427fe626`):

- driver routes wasm32-gc library worlds to `wasm::emit_library_component`
  so specialized linear i32 adapters are not instantiated against GC
  String/list/record exports
- overlay stub and driver patch match `core_wasi` (previous patch looked
  for `config_wasi_version` and never applied)
- Official string-multi / string-greet / string-len / list-first /
  record-point / bool-logic validate + wasmtime invoke PASS
- gc-layout-audit on string-multi: `name=0 fallback=0`

### wasm32-gc pinned (`9a821f07`)

Pinned bootstrap is the s2==s3 fixpoint of `9a821f07` (component emit
bool-only exports no longer trap; generic core-func alias matches
library emit). Guest remains `wasm32-gc` / `wasi-p2` memory32. Official
`build-compiler` then `fixpoint --build` 2026-08-31T07:48–07:57Z:
s2==s3=`427fe626` EXIT 0.

Intentional drift from the previous pin (`75401a23` / `695de0cb`):

- `StringGeneralPlan` / `F32GeneralPlan` collect `Vec<String>` /
  `Vec<i32>` only so leftover dest cannot retype `mir` as a custom
  struct vec (`--emit component` bool-logic no longer traps)
- generic core-func alias includes the `0x01` instance-sort byte
- Official bool-logic 6/6 PASS on this artifact
- string-multi still validate-fails: specialized string adapters
  import user funcs as `(i32)->(i32)` while wasm32-gc exports
  `(ref null String)` — next pin after the GC adapter fix

### wasm32-gc pinned (`75401a23`)

Pinned bootstrap is the s2==s3 fixpoint of `75401a23` (PR 51 `#665` +
PR 52 leftover linear emit on clock-qmark). Guest remains `wasm32-gc` /
`wasi-p2` memory32. Official `build-compiler` then `fixpoint --build`
2026-08-31T05:31–05:44Z: s2==s3=`695de0cb` EXIT 0.

Intentional drift from the previous pin (`25e28e48` / `54d01aff`):

- WIT import-load no longer calls `parser::parse_full` (`#665`
  `compose_roundtrip --emit component` does not trap)
- Linear leftover: nested `STRUCT_GET` base load, `hashmap_new` inline
  when DCE'd, `ref.func` → `i32.const` table index (`closure_map`)
- leftover isolated T3: 460 pass / 0 validate-fail / 0 compile-fail / 23 skip

### wasm32-gc pinned (`25e28e48`)

Pinned bootstrap is the s2==s3 fixpoint of `25e28e48`. Guest remains
`wasm32-gc` / `wasi-p2` memory32. Re-proved 2026-08-30T21:19–21:28Z with
`ARUKELLT_FIXPOINT_NO_CACHE=1`: pin==s2==s3=`54d01aff` EXIT 0.

Intentional drift from the previous pin (`fdf2101e` / `87e5d135`):

- `emit_struct_set` does not `local.tee` an open-enum `ref.cast` into a
  final variant local (`enum_struct_variant` validates; Circle/Rect keep
  their own type)
- Isolated: `enum_struct_variant` MATCH `75` / `24`

### wasm32-gc pinned (`87e5d135`)

Pinned bootstrap is the s3==s4 fixpoint of `87e5d135` (not pin→s2 `082f4148`).
Guest remains `wasm32-gc` / `wasi-p2` memory32. Official `fixpoint --build`
from the previous pin produced `sha256(s2)=082f4148…` ≠ `sha256(s3)=fdf2101e…`;
one more round compiled `s3 → s4` with `s3==s4`.

Intentional drift from the previous pin (`5abf9573` / `d4ceef89`):

- `ctx_variant_name_index` key-match returns a storage index so
  `json::JsonParseError::TrailingCharacters` / `UnexpectedCharacter` compare
  tags 2/3 (not builtin Option slots 0/1)
- Name-fallback / core_op for `text::lines`, `slice_bytes`, `product`,
  `remove`, `sort_f64`, trim, reverse, pad, seq min/max/count/search/unique
- `format_bool` always leaves a String on the stack
- JSON incomplete `true`/`false`/`null` → `InvalidLiteral`; RFC 8259 exponents
- Empty `read_stdin` yields `""`; Result `result:T:E` keeps path-colon E
- Isolated official-355 compiler leftovers MATCH; host/sandbox EXTERNAL remain

### wasm32-gc pinned (`d4ceef89`)

Pinned bootstrap is native `wasm32-gc` / `wasi-p2` with guest memory32
(`(memory 8192)`). `BOOTSTRAP_EMIT_TARGET` / `BOOTSTRAP_EMIT_WASI_VERSION` in
`scripts/selfhost/checks.py` match. `_ensure_bootstrap_compiler_wasm` copies
the pin without `--to-memory64`. Execution uses `scripts/run/arukellt-run-hosted.sh`
(host-linker) for `wasi:cli/` and `arukellt:runtime/host@0.1.0` filesystem imports.

Intentional drift from the previous P2-stub pin (`7f0b5edd` / `1d924aea`):

- TypeSectionPlan constructor call and P2 runtime memory-index predicate
  wrapped to restore `lines_ge_200` ratchet (2)
- New i32 `to_string` / `vec_push` fallback predicates use `callee_name_is`
  so `eq(clone(callee)` stays at baseline 74
- Isolated verify-quick hygiene: ADR unique IDs, ci-jobs, gate-667 `wasm32-gc`,
  formatter exception hashes, CI aggregate canonical manager commands

Intentional drift from the previous WIT-import pin (`dfd074f3` / `0565a6ca`):

- P2 command components stub unused `arukellt:runtime/host` at component level
  instead of emitting leftover `runtime-*` imports that `wac plug` cannot close
- P2 run-world type indices are computed after prefix + runtime import count
  (fixes `type index 7 is not a defined type`)
- Isolated #074/#510 PASS: `wasi_p2_native/hello.ark` component validates and
  `wasmtime run -W gc=y -W gc-support=y` prints `hello p2`
- Isolated #665 close-gate required path still PASS

Intentional drift from the previous dest-typing pin (`9f91bac4` / `ec200344`):

- `parse_wit_import_file` no longer calls discarded `parser::parse_full`
  (`WitNode` enum into leftover String dest trapped on readable WIT files)
- Isolated #665 close-gate required path PASS on this artifact

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
