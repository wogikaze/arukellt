# selfhost build breakage — 2026-10-29 (HashSet retirement)

Tracking: #851 / #917.  Related plan section: selfhost-compiler-core-rewrite.

## Symptom

After the concurrent HashSet-retirement series, `selfhost build-compiler`
failed with invalid output: the generated `arukellt-s2.wasm` did not validate
(`expected i32 but nothing on stack`, `expected (ref null $type) but nothing on
stack`) and the bootstrap helper deleted it, so the runtime compiler could not
be prepared.

## Isolation

- HEAD `084a0d1be` **on its own** (frozen worktree) fails with the current pin
  `62aecf93…`, and also fails when compiled by a fresh gc host built from
  `b9fbac631` (candidate pin below).  The output is invalid in both the
  `wasm32`/`wasi-p1` and `wasm32-gc`/`wasi-p2` paths.
- Applying the resolver-table change that was staged in the main working tree
  (later committed as `75e17ff4f` / `8416ff9bf`) makes the build succeed.  The
  commit alone is therefore incomplete: the retired-name resolver update is
  required.
- origin HEAD `b9fbac631` builds and passes fixpoint with the current pin
  (`s2 == s3 == e108ff07…`).  Quiet measurements there:
  build-compiler warm **31.9 s** / user 30.6 s, RSS 1.13 GiB.
- Current working tree (resolver fix included): build-compiler warm
  **33.2–34.4 s** (host load 2–20 during the runs), fixpoint PASS
  (`s2 == s3 == 353473ae…`), stdlib_json / generics_v1 fixture parity PASS,
  RSS 1.13 GiB.

## Candidate gc pin (not adopted)

Built from the stable origin HEAD `b9fbac631` with the current pin as producer:

| Field | Value |
|-------|-------|
| component size | 8 550 003 bytes |
| component sha256 | `7a2257f2a95e6ba1daa12e831b03d4894504e4b27b261509d83f26d5f3e79922` |
| core sha256 | `aa64c65dea032e25c760206728448b2d7f746ce0484cd24473a6eb40d85e1b49` |

The candidate compiles the stable source; refresh should wait until the
HashSet-retirement series lands in main, then run the full PROVENANCE refresh
procedure (p1 identity + gc self-reproduction).

## Operational notes

- A stale flat-overlay disk cache can outlive a source change when builds run
  concurrently: `rm .build/selfhost/flat-overlay-cache.json` and regenerate.
- Frozen worktrees are the reliable way to measure a fixed commit; the shared
  checkout receives concurrent commits every few minutes.
- `ctx_mono_type_params.ark` is replaced wholesale by
  `_patch_bootstrap_stub_static_dispatch` in the bootstrap overlay, so changes
  there do not affect the selfhost compiler (see the plan's stub-module note).

## Pin refresh 2026-10-29 (471855ad)

Refreshed the official gc pin from `36f42b97e` with the previous pin as the
producer.

| Field | Value |
|-------|-------|
| component | 8 550 341 bytes, sha256 `471855ad9db035c1ada136852fc55e896b477cf35303be066f352d9d563a8895` |
| p1 identity | `353473ae5a24230b1e9c83d23d73b818396c0ae8bdd45c46688ac742a5101739` (previous pin's s2 for that source) |
| gc self core | `c0db44ce037ba87e19c32c15a917f5b93619b00c8274f1ff778957ca5d55a5e7` |
| HEAD at adoption | `aac7621ab` (advanced during the refresh) |
| verification with the new pin | s2 == s3 == `1eb4a81c…`, fixpoint PASS, stdlib_json parity PASS, verify lane PASS |

Warm build-compiler with the new pin measured 44.0 / 45.0 s at host load
13-20 (user 37.7 / 39.1 s); the quiet-host range is documented as ~30-34 s.

## Leaf mono-instance index (2026-10-29)

`sig_reg_leaf_instance_index` scanned every mono instance for each decl with
`mono_index_count == 0` (quadratic on the instance list). The guest profile put
98% of the `MonoInstance_type_arg_count` / `TypeInfo_tag` /
`MonoInstance_type_arg_at` calls under it (~11.8% self combined).

Replaced the scan with a leaf-name bucket index built once per registry build
(`head` NameIndex + `next` chain in `signature_registry_build_mono.ark`);
lookups walk only the leaf bucket and keep the same match/ambiguity semantics.

- instruction A/B (AOT cwasm + perf): 255.30 -> 246.08 G instructions
  (-3.61%, reproduced across runs)
- emitted wasm byte-identical between baseline compiler, optimized compiler,
  and pin-built stage-2
- on HEAD `1a4690403` (change included): fixpoint PASS
  (`s2 == s3 == 19060c25…`), fixture parity PASS, warm build-compiler
  **25.5 s** / user 24.6 s, RSS 1.13 GiB
- with the earlier name-scan removals the series is ~-9% instructions
