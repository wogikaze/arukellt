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
