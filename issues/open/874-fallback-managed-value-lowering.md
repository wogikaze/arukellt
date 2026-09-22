---
Status: open
Created: 2026-09-19
Updated: 2026-09-20
ID: 874
Parent: 852
Track: language-v2
Depends on: "854, 855, 859, 860, 873"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 8 (fallback lowering)"
---

# 874 — fallback managed-value lowering を実装する

## Scope

RFC-011: **8 (fallback lowering)**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] struct/enum/Option/Result の fallback layout を実装する
- [ ] closure env を fallback heap へ lower する
- [ ] Vec/String/HashMap を fallback runtime に接続する
- [ ] 共有・循環参照を reject せず実行する
- [ ] Wasm GC backend と同じ move/borrow semantics を保つ

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する

## Progress receipt — 2026-09-20 (aggregate allocation boundary)

- Implementation commit `6f480e71e` connects linear-memory `struct` and array
  constructors, plus the fallback `Vec` layout constructor, to the private
  `LINEAR_GC_ALLOC` runtime function. The allocator records stable payload
  ranges and can reuse swept records; the old raw heap bump remains available
  for migration paths that have not yet supplied managed descriptors.
- Normative contract: [ADR-075](../../docs/adr/ADR-075-language-v2-linear-memory-tracing-runtime.md)
  and the [linear-memory lowering plan](../../docs/plans/wasm32-linear-memory-lowering.md),
  with RFC-011 section 8 as the language-v2 scope. No new public API or ABI
  was introduced.
- Positive fixtures and outputs: `tests/fixtures/enums/enum_struct_variant.ark`
  produces `75\n24` on both backends; `tests/fixtures/collections/vec_option_i32.ark`
  produces `7` on the fallback; `vec_string_println_get.ark` produces `hi`,
  and `generics/gc_struct_field_vec_string.ark` produces `1` on the fallback.
  The GC enum fixture was packaged as a component; the linear artifacts were
  validated as core modules and contain no `struct.*`, `array.*`, or `ref.*`
  instructions.
- Diagnostic coverage: `tests/fixtures/diagnostics/non_exhaustive.ark` still
  rejects with `E0200` and missing `Color::Blue`; allocator wiring does not
  bypass enum diagnostics.
- Verification receipts:

  ```text
  python3 scripts/manager.py selfhost build-compiler       PASS
  python3 scripts/manager.py verify lane                  PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py PASS (4 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py      PASS (1 test)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32-gc enum component 3159 bytes  be00f6c48b931eb26c732701fad6cb00664c6ae14ad2f1a1994877be9255b914
  wasm32 enum core         2489 bytes  f7509d77104d18c4f7f547c9f68630b0412401dbb050bb71d74cf8bac6c28597
  wasm32 Vec<Option<i32>> 2269 bytes  87f29da04f696d0834d4483e2e20ae998d683eb3d1ce2f1a04daf83b6cd889b7
  ```

- Tool versions were `wasmtime 46.0.1 (823d1b8f2 2026-06-24)` and
  `wasm-tools 1.252.0`. Root-frame insertion and automatic collection remain
  disabled until all managed locals have descriptor-aware updates. String and
  HashMap descriptor lowering, closure environments, shared/cyclic source
  graphs, and the complete move/borrow parity audit remain open; this receipt
  does not close #874.

## Progress receipt — 2026-09-20 (String allocation boundary)

- Implementation commit `8df8fa460` routes linear `concat` and
  `string_from_bytes` through the same private `LINEAR_GC_ALLOC` entry used by
  the aggregate constructors. Both paths preserve the existing length-prefix
  ABI: the allocator returns the record base, the stored length remains four
  bytes before the returned payload pointer, and copying still addresses the
  payload pointer.
- Positive fixtures: `tests/fixtures/stdlib_migration/new_api_basic.ark`
  runs as `Hello, Arukellt!`, `3`, and `one | two | three` on `wasm32`;
  `tests/fixtures/stdlib_bytes/conv_string_from_bytes.ark` completes its `HI`
  assertion on `wasm32`; and `tests/fixtures/stdlib_string/string_concat.ark`
  passes on both the linear core module and the GC component.
- `wasm-tools print` of the two linear artifacts shows calls to the private
  allocator, while the linear WAT contains no `struct.*`, `array.*`, or
  `ref.*` instructions. The GC component path remains on the existing GC
  array representation and validates independently.
- Verification after the change:

  ```text
  python3 scripts/manager.py selfhost build-compiler       PASS
  python3 scripts/manager.py verify lane                  PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py PASS (4 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py      PASS (1 test)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32 linear concat       4850 bytes  96d9f5d898a56e79cdb87a155353da7f665e58af98f8e42e993dcf44ffbbef77
  wasm32 linear from-bytes  4276 bytes  ae204a573aad3bc12e083bebb5b2d734801b07908bd9c2fd01bc30a6dcecce4c
  wasm32-gc concat component 4803 bytes 833bacae7ee92fd6dedd41e8a25c2c0dc1533609e000432777286b209be856e8
  ```

- This slice covers two String allocation paths only. String case/format/
  split/join/replace and other allocation helpers, HashMap descriptors,
  closure environments, root updates, automatic collection, and complete
  cross-backend move/borrow parity remain open.

## Progress receipt — 2026-09-20 (compiler root-frame integration)

- Implementation commit `aed3af031` connects the linear fallback compiler to
  the existing private root-frame runtime. Each linear function saves a frame
  cursor, conservatively registers its i32/ref locals, updates the matching
  slot after `LOCAL_SET`, and restores the frame on return/fallthrough. The
  GC backend is unchanged, so this boundary does not introduce target-specific
  source semantics or a public API.
- Positive verification: the current-source linear String and enum artifacts
  validate, the fallback enum executes `75\n24`, the GC enum component also
  validates and executes `75\n24`, and the GC aggregate parity guard passes
  (`Ran 2 tests ... OK`). The focused root/runtime contract passes (`Ran 5
  tests ... OK`), the collector e2e passes (`Ran 1 test ... OK`), and
  `verify lane` passes.
- This is a root-hook integration slice, not completion of fallback managed
  values. Descriptor-aware struct/enum/Option/Result/Vec/String/HashMap write
  coverage, closure environments, shared/cyclic source graphs, automatic
  collection, and the full move/borrow parity audit remain open. No `collect`
  call was added.

## Progress receipt — 2026-09-20 (String transform allocation boundary)

- Implementation commit `196da76fa` routes the linear fallback allocations in
  `slice`, `trim`, `trim_start`, `trim_end`, `to_upper`, and `to_lower` through
  the private `LINEAR_GC_ALLOC` entry. Each path now consumes `length + 4`,
  stores the allocator result as the record base, and preserves the existing
  length-prefix ABI by using `base + 4` as the payload pointer. The Wasm GC
  branch remains on its existing array lowering and no public API or source
  semantic distinction was introduced.
- Positive fixtures `tests/fixtures/stdlib_string/string_slice.ark`,
  `trim_basic.ark`, and `to_lower_upper.ark` compile, validate, and execute
  successfully on both the `wasm32` linear core and `wasm32-gc` component
  paths. The fixtures' string assertions pass on all six artifacts; the
  linear modules contain no `struct.*`, `array.*`, or `ref.*` instructions.
- Verification after the change:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  wasm-tools validate                                       PASS (6 artifacts)
  wasmtime run                                              PASS (6 artifacts)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32 linear slice       4138 bytes  86362a16f3957ac8fa26d9deb2e13f4cb3826f623921d07a5ceca7eb2d3034bd
  wasm32 linear trim        4771 bytes  c49eacf0f3fdebe579a3be7a18ecfbe29d7fc45df3cadc286126461604915360
  wasm32 linear case        4489 bytes  83c4ea98f4e2e2aee7703e3c665c512d9b61f91c5cd2787abefbf04bb82ed439
  wasm32-gc slice component 12693 bytes 827913fba6b549ed986293865c606e13ed2da33420d0acce481da607b7910902
  wasm32-gc trim component  13517 bytes 4ec6db4848bd4c41a6058947879b3dfe5cf544a0ed1169b50e68e4dc230d40dc
  wasm32-gc case component  13054 bytes 1cc01b99582786d834467ed1971707fd61da959b4f5f12ed08211d9a690f1181
  ```

- This receipt covers three String transform families only. Remaining String
  format/split/join/replace and other allocation helpers, HashMap descriptors,
  closure environments, descriptor-aware root writes, automatic collection,
  shared/cyclic source graphs, and complete cross-backend move/borrow parity
  remain open; #874 stays open.

## Progress receipt — 2026-09-20 (String composition allocation boundary)

- Implementation commit `bf94dbe98` routes fallback `join`, `repeat`,
  `pad_left`, and `pad_right` allocations through `LINEAR_GC_ALLOC`. `join`
  allocates after its total-length pass, `repeat` uses the multiplied byte
  length, and `pad` uses the requested final width; all preserve the
  length-prefix record ABI and return `base + 4` as the payload pointer.
  The no-fill pad path emits the documented ASCII space directly, while an
  explicit fill continues to read the supplied String. The Wasm GC branches
  remain unchanged.
- Positive fixtures `tests/fixtures/stdlib_string/string_join.ark`,
  `repeat_basic.ark`, `pad_left_basic.ark`, and `pad_right_basic.ark`, plus
  `join_empty_vec.ark` and `repeat_zero.ark`, compile, validate, and execute
  successfully on both the `wasm32` linear core and `wasm32-gc` component
  paths. All twelve artifacts pass their string assertions, including
  `"  5"`/`"5  "` and zero-length results.
- Verification after the change:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  wasm-tools validate                                       PASS (12 artifacts)
  wasmtime run                                              PASS (12 artifacts)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32 linear join        4880 bytes  e5566204ed311bfb318869e058bd8ebff4f6ce6d6e438a689c1a7204de407433
  wasm32 linear repeat      4415 bytes  8fddf1839c88f7d731e38d097d3183fb56479e2d906de11e4aeb1678351fc6b9
  wasm32 linear pad-left    4481 bytes  f8b2f887e3270ba75c08bd328db908f4799b9ee78d895b77137730256474c467
  wasm32 linear pad-right   4484 bytes  1081e85fd8d1c8d63c51fc4d297978b7fd8b14297c0cbff21aef49227282ce47
  wasm32-gc join component  13437 bytes 4f51a903510cb1663da0a902aebb07fb6098f59d816870b97ac55e887c964716
  wasm32-gc repeat component 13065 bytes 1b7afe5092c1b99a96e98d4350c561d3456acdb72e387ef952063f3daa727cd7
  wasm32-gc pad-left component 13312 bytes 4c59d5e772a107d35cdb05ca46967c1b3e9f6c68a2aaf08417e43ac022068a11
  wasm32-gc pad-right component 13284 bytes f9aff0778e16a7ac3cf3ba169cd64bc8c65011a23a252fee64a7e8b0ed31d1f7
  ```

- This receipt covers the second String allocation family only. String
  split/replace/format/char and other allocation helpers, HashMap descriptors,
  closure environments, descriptor-aware root writes, automatic collection,
  shared/cyclic source graphs, and complete cross-backend move/borrow parity
  remain open; #874 stays open.

## Progress receipt — 2026-09-20 (String replace allocation boundary)

- Implementation commit `d00da2f1c` routes fallback `replace` allocation through
  `LINEAR_GC_ALLOC`. The existing upper-bound calculation now supplies the
  allocator with `upper_bound + 4`; the returned record base is retained and
  the length-prefixed payload continues to use `base + 4`. The Wasm GC branch
  and public API remain unchanged.
- An empty pattern now returns the original source string. This preserves the
  non-progressing edge case without entering the replacement loop, which would
  otherwise repeatedly inspect the same position and eventually write out of
  bounds.
- Positive fixtures `tests/fixtures/stdlib_string/replace_basic.ark`,
  `replace_multiple.ark`, `replace_no_match.ark`, `replace_with_empty.ark`,
  and `edge_repeated_chars.ark` compile, validate, and execute successfully on
  both the `wasm32` linear core and `wasm32-gc` component paths. All ten
  artifacts pass their assertions, including the empty-pattern case.
- Verification after the change:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  wasm-tools validate                                       PASS (10 artifacts)
  wasmtime run                                              PASS (10 artifacts)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32 linear replace-basic        5176 bytes  84b8deac5a5e36fbbd2c6b693cba7171b19617ae123833aff757e68a8bb5eb96
  wasm32 linear replace-multiple     5179 bytes  8e9b5e02951a49f39b02fc68a45ac33197f40dcc9304973fafcb858d79454335
  wasm32 linear replace-no-match     5159 bytes  fb6ff79fc543b05fcf900c5063f5e6638842efe8de371a4273aca1cb4006f742
  wasm32 linear replace-empty        5162 bytes  8a57f7ad82cb7112714811b5c9c0137402099423a6ecaac383c48765d3905973
  wasm32 linear repeated-chars       5832 bytes  3836731666d1e424a0e6f2b8db8e3a12bf5ca874d90cd132b20c410c77ab899a
  wasm32-gc replace-basic component 14348 bytes  2f835f332bbebeeb7158ec88f745e300358de77c0114f91e004172789b80d7c
  wasm32-gc replace-multiple        14351 bytes  bd492d103b4ab1b2c0aa442675e25c4adb51029f560993edcfb5a9c3a637f539
  wasm32-gc replace-no-match        14241 bytes  1ce2f387aa9638adc50430967fc4d850483f987cd024422f9fefc6f056adcc47
  wasm32-gc replace-empty           14334 bytes  da9cd2ead9cedb54005bb9dc29def05139c678944bf74b3d01c4dfad63b6466d
  wasm32-gc repeated-chars          14835 bytes  ff425e7bdafa011ff72e087e8bb4a4ea374d7d47484678ecac63402be5773704
  ```

- This receipt covers the `replace` allocation path only. String
  split/format/char and other allocation helpers, HashMap descriptors,
  closure environments, descriptor-aware root writes, automatic collection,
  shared/cyclic source graphs, and complete cross-backend move/borrow parity
  remain open; #874 stays open.

## Progress receipt — 2026-09-20 (String split allocation boundary)

- Implementation commit `26513a797` routes the fallback non-empty-separator
  `split` path through `LINEAR_GC_ALLOC`. The 76-byte Vec header/backing
  storage and every split String record are now registered allocations. Each
  String size is `round_up(4 + segment_len, 4)`; the allocator result remains
  the record base and the existing length-prefix payload uses `base + 4`.
  The Wasm GC branch and public API remain unchanged.
- Empty-separator `split` still delegates to the existing fallback `chars`
  lowering. That path was intentionally not folded into this slice: its Vec
  and per-character String allocations remain a separate boundary. The
  existing behavior was regression-tested on both targets.
- Positive fixtures `tests/fixtures/stdlib_string/string_split.ark`,
  `split_multichar_delim.ark`, `split_single.ark`, `split_no_match.ark`, and
  `stress_many_split.ark` compile, validate, and execute successfully on both
  the `wasm32` linear core and `wasm32-gc` component paths. The existing
  `tests/fixtures/stdlib_text/split_empty_sep.ark` also passes on both paths;
  all twelve artifacts pass their assertions.
- Verification after the change:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  wasm-tools validate                                       PASS (12 artifacts)
  wasmtime run                                              PASS (12 artifacts)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32 linear string-split          6491 bytes  040db3a430360794b58adf7bf9dabfc1f5554371fbf09e9958f2bccfc70132a6
  wasm32 linear multichar-delim       7407 bytes  1b25656752a2b2866679f64d7d5b214857f31639a449480e27338fbac20a40f9
  wasm32 linear split-single          6993 bytes  33bd5e19386a974574348c13cf580253fb6d8632f2c4a92f0e5a990496651b83
  wasm32 linear split-no-match        6996 bytes  e4b1f849b71dccb3ef7a549ec9d5b8baefc90524ce2b93c49aae9dbd4b333388
  wasm32 linear stress-many-split     6584 bytes  f4a22911300deb2ae7d9ad97638adb4b8861e47f4c563e8449c4f3d493fafb09
  wasm32 linear empty-separator       6732 bytes  70632129a2c7be77b3cb34ffb5d38b8b73d359a3357c15af15e4fa5729707342
  wasm32-gc string-split component   15797 bytes  24b5da0222f2c129df0179b1a4af171c6fe13dea43a3873da2918d0fa7e6b3ce
  wasm32-gc multichar-delim          15809 bytes  8d8cf5de582dc4307b3a752664ad08ec8d9803ebdaa8280bf82265a1a7c30594
  wasm32-gc split-single             15403 bytes  25ce8747a4d133424d30687d5a8db376e7b4459c64a9ff495d850e38467e8461
  wasm32-gc split-no-match           15406 bytes  daf37f7594c87055074bb2f491576ba4a30a8fb8159d42a2fae9d804f7e20fa1
  wasm32-gc stress-many-split        14723 bytes  59505d957a2b745ae6395f441848c7e146c721ca887498cd69b4c2fcf1617641
  wasm32-gc empty-separator          14793 bytes  8a68feb490bec07e41fb170cb0aa23df57ac0cfaa19fbc51b9b77b7a836bf35d
  ```

- This receipt covers only the non-empty-separator `split` allocation path;
  `chars`/empty-separator, String format, other allocation helpers, HashMap
  descriptors, closure environments, descriptor-aware root writes, automatic
  collection, shared/cyclic source graphs, and complete cross-backend
  move/borrow parity remain open; #874 stays open.

## Progress receipt — 2026-09-20 (String chars allocation boundary)

- Implementation commit `b31fe4557` routes fallback `chars` allocation through
  `LINEAR_GC_ALLOC`. The Vec size remains `12 + count * 4`, and each scalar
  String uses `width + 4`; the allocator result is stored as the record base,
  with the established length-prefix payload at `base + 4`. The Wasm GC branch
  and public API remain unchanged.
- This completes the fallback allocation path used by empty-separator `split`:
  `text::split(source, "")` now reaches the same allocator boundary as the
  non-empty-separator path. Empty source behavior remains one empty String
  element, as covered by the existing fixture.
- Positive fixtures `tests/fixtures/stdlib_text/chars_basic.ark`,
  `string_chars.ark`, `split_empty_sep.ark`, and `stress_many_splits.ark`
  compile, validate, and execute successfully on both the `wasm32` linear core
  and `wasm32-gc` component paths. All eight artifacts pass their assertions
  and output checks.
- Verification after the change:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  wasm-tools validate                                       PASS (8 artifacts)
  wasmtime run                                              PASS (8 artifacts)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32 linear chars-basic          6567 bytes  16869409e9fb2cdae594610f86cdb0e84d4f9c31f3767d997f417b2fdc19ce56
  wasm32 linear string-chars         4473 bytes  a23a3954c503ef28caf75c2d04cafad0a966633ad32db6bd30ed29b9b90f1a93
  wasm32 linear split-empty-sep      6381 bytes  e9003ee5d45c23e6adf5ceabc7c840e898a1a9ee9a4bf5915363c2fb0c48fb37
  wasm32 linear stress-many-splits   7703 bytes  0504a2484ede2beb6e61326c28a5623f3a8fde67cd4fc18fd33068c3318681d4
  wasm32-gc chars-basic component   14799 bytes  1954d5050bb22e13a70ee87a0dfa87f4e1e12fd4dfbb4fe424f87c4e729c547f
  wasm32-gc string-chars            12667 bytes  78ca72b67c91423d8cc29806aec4d4730235ccfcf1dd43e21f4c7b81724e2b82
  wasm32-gc split-empty-sep         14793 bytes  8a68feb490bec07e41fb170cb0aa23df57ac0cfaa19fbc51b9b77b7a836bf35d
  wasm32-gc stress-many-splits      16389 bytes  219482ba0901b0ed583f3745b6ab11bb480f5ba15bf3c16435d233d4af58e087
  ```

- This receipt covers the fallback `chars` and empty-separator `split` paths.
  String format and other allocation helpers, HashMap descriptors, closure
  environments, descriptor-aware root writes, automatic collection,
  shared/cyclic source graphs, and complete cross-backend move/borrow parity
  remain open; #874 stays open.

## Progress receipt — 2026-09-20 (String numeric and char formatting allocation)

- Implementation commit `5393f3dc7` routes fallback allocation in numeric and
  floating-point String formatting (`i32`, `i64`, `f64`), `char_to_string`,
  `push_char`, and `String_new` through `LINEAR_GC_ALLOC`. The i32 path keeps
  its 4-byte size rounding; all paths retain the record-base and
  `base + 4` length-prefix payload ABI. The Wasm GC branches and public API
  remain unchanged.
- Positive fixtures covering `format_i32`, `format_i64`, `format_f64`, boolean
  formatting regression, numeric `to_string`, char conversion, empty String
  construction, and repeated `push_char` compile, validate, and execute
  successfully on both the `wasm32` linear core and `wasm32-gc` component paths.
  Eighteen fixtures produce 36 passing artifacts.
- Verification after the change:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  wasm-tools validate                                       PASS (36 artifacts)
  wasmtime run                                              PASS (36 artifacts)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32 linear format-i32           5468 bytes  339517eb01fbc73193a80e1170dc77334d88d90400e1fe980d26908b1c787e54
  wasm32 linear format-i64           6277 bytes  81b157e852a555c6ec10820ebbd84f368d70a0937e4f20e72ce8a6ece7866479
  wasm32 linear format-f64           7461 bytes  94bdde8f6e21973a7159416e9823b271f0743a08dc67943081dd2e6ea47676d3
  wasm32 linear char-to-string       5391 bytes  6a3d394098e8e28c2934984a83aea168582609a0dccea43f3fef17986d0e1abb
  wasm32 linear String_new           3894 bytes  fea6c024aed19163207886527bccad1c7c1fe83ee3ab0bc0cc7f5de174030dfc
  wasm32 linear push-char            3690 bytes  762d38f98f4dbde285c0777f3eb9e37a89f7c0f5787ba94030845f5fffe7f2fc
  wasm32-gc format-i32 component    14225 bytes 243178f58766a7f462d829a73d91a886a1b06932ac4cc1aab4d5a947b180b70f
  wasm32-gc format-i64 component    15220 bytes 361278ae2a3db674df5a60997dc0aa812df04ef8154663a3ca631f80cd2a977c
  wasm32-gc format-f64 component    16251 bytes 10818f4e15e7c15d9104de24e947e229d227675848901711b33b2a041d56b695
  wasm32-gc char-to-string          13914 bytes 7f44707d02fe1a525780196293e98ebbdad9b89a6859a91beb456311730681c5
  wasm32-gc String_new              12448 bytes af60506bcee10a75aee36bea1fe1cdf265e19e143de3951d3ae7032d40a285f3
  wasm32-gc push-char               11912 bytes 9e62e5910612ab0f647c0642b12d723a4c0dde6be8ffccec6109d292083e48dd
  ```

- This receipt covers the numeric/floating-point and char-oriented formatting
  allocation paths only. Boolean/debug/structured formatting helpers and
  other String allocation helpers, HashMap descriptors, closure environments,
  descriptor-aware root writes, automatic collection, shared/cyclic source
  graphs, and complete cross-backend move/borrow parity remain open; #874
  stays open.

## Progress receipt — 2026-09-20 (Vec backing-growth allocation boundary)

- Implementation commit `9e81a367e` routes both linear fallback Vec growth
  paths through `LINEAR_GC_ALLOC`: concrete `vec_push` growth and the sealed
  `raw_array_grow` path now allocate `grown_capacity * element_stride` bytes
  through the allocator before copying the old backing storage. The existing
  Vec header offsets, capacity doubling, logical length update, element stride,
  and memory-copy behavior remain unchanged. Wasm GC continues to use its
  existing typed array lowering.
- This connects the fallback allocation boundary used by the compatibility
  `std::collections::hash` implementation. HashMap construction and rehashing
  therefore exercise recorded Vec backing allocations, while descriptor-aware
  HashMap scans and automatic collection remain intentionally outside this
  slice.
- Positive fixtures `tests/fixtures/stdlib_hashmap/stress_100.ark`,
  `hashmap_capacity_extend.ark`, and `stress_mixed.ark` compile, validate, and
  execute successfully on both the `wasm32` linear core and `wasm32-gc`
  component paths. The concrete push regression
  `tests/fixtures/stdlib_vec/vec_stress_push_100.ark` also passes on both
  paths; all eight artifacts pass.
- Verification after the change:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/manager.py fmt --check                   PASS (1972 checked)
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  wasm-tools validate                                       PASS (8 artifacts)
  wasmtime run                                              PASS (8 artifacts)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32 linear HashMap stress-100       17094 bytes d174fe4554d8b5d3f7e7e9c64565f004ed476ba42cc0d958bd95621503973262
  wasm32 linear HashMap capacity-extend  21747 bytes b1fe374f65ed1ebc6c0ed75de480301108f9a61a648d648a9eba0fb9b8cc1423
  wasm32 linear Vec push-100              4441 bytes 9eb8cc62572976f901ccc696f7fd72b02268d5f6844c0a44d60c88cc0ba55578
  wasm32-gc HashMap stress-100 component 21770 bytes 9b52f1e890ce9c323b80d1bea259030ef40b3622500d753832a38e0b61a9b6ea
  wasm32-gc Vec push-100 component       12445 bytes 9b7aa6e3f2cb4eb66ce57f6d0ed8ef8977ef236d97aab41909e0b6e4538921b5
  ```

- This receipt covers Vec backing growth and the compatibility HashMap
  allocation boundary only. HashMap descriptors, exact pointer-field write
  metadata, closure environments, automatic collection, shared/cyclic source
  graphs, and complete cross-backend move/borrow parity remain open; #874
  stays open.

## Progress receipt — 2026-09-20 (Vec Option result allocation boundary)

- Implementation commit `0578b95a2` routes fallback `Vec::get` and `Vec::pop`
  `Option<i32>` results through `LINEAR_GC_ALLOC`. Both `Some` and `None`
  preserve the existing fixed 8-byte record layout (tag at offset 0 and
  payload word at offset 4); only the allocation owner changes from direct
  global-bump manipulation to the private allocator. Wasm GC continues to
  construct typed Option variants.
- Positive fixtures `tests/fixtures/stdlib_vec/vec_get.ark`,
  `vec_edge_boundary_indices.ark`, `vec_pop_all.ark`, `vec_pop_empty.ark`,
  and `vec_pop_then_push.ark` compile, validate, and execute successfully on
  both the `wasm32` linear core and `wasm32-gc` component paths. All ten
  artifacts pass, covering Some/None and empty/non-empty transitions.
- Verification after the change:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  wasm-tools validate                                       PASS (10 artifacts)
  wasmtime run                                              PASS (10 artifacts)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32 linear Vec get                 4910 bytes db7c3c84fd6b82da66b7c6138fdad7facc632cd784c06a68f2b892d71428769a
  wasm32 linear Vec pop-all             5372 bytes 5a81c457add6d08ae2982464445160574c79d2e15241ac986e8251a2ef4b682a
  wasm32 linear Vec pop-empty           3842 bytes abe9fb2bb821de33005449e9d33245f1c6a84e0be2e17d4f2553eb270d593905
  wasm32-gc Vec get component          13212 bytes e2f9b6ac557b902f1bbb88ef8edea19e06bb015acdef1b2ef97247cfbac9527b
  wasm32-gc Vec pop-all component      13701 bytes 41d4f2c994983c402c0e243031e27a7087e5340601b7ffdb909fc165e1209367
  ```

- This receipt covers only the fallback `Option<i32>` result records emitted
  by Vec access and pop. Generic Option/Result constructors, String-valued
  result payloads, parse I/O buffers, HashMap descriptors, closure
  environments, automatic collection, shared/cyclic source graphs, and
  complete cross-backend move/borrow parity remain open; #874 stays open.

## Progress receipt — 2026-09-20 (Integer parse Result allocation boundary)

- Implementation commit `f88ce4917` routes the fallback integer parse Result
  records through `LINEAR_GC_ALLOC`: `Ok<i32>` uses its existing 8-byte record,
  `Ok<i64>` its existing 12-byte record, and invalid-integer errors allocate
  the existing 32-byte length-prefix String plus 8-byte `Err` record. Tag,
  payload, length-prefix, and `base + 4` String ABI layouts remain unchanged;
  the Wasm GC branches are unchanged.
- Positive fixtures `tests/fixtures/stdlib_string/parse_i32_basic.ark`,
  `parse_i32_invalid.ark`, `parse_i64.ark`, and `parse_i64_large.ark` compile,
  validate, and execute successfully on both the `wasm32` linear core and
  `wasm32-gc` component paths. All eight artifacts pass.
- Verification after the change:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS (20.5s)
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  targeted fmt --check                                     PASS (2 files)
  wasm-tools validate                                       PASS (8 artifacts)
  wasmtime run                                              PASS (8 artifacts)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32 linear parse-i32-basic       4816 bytes 2ba2498a9055912e0eafb168a9c00450ca01afa0e5f900ba7b01b72ff8c92960
  wasm32 linear parse-i32-invalid     3400 bytes e18f2e1e1982d7ce16fea93986e0f48d5c7a57773c353c2e6986b7dd45ed0052
  wasm32 linear parse-i64              6598 bytes a271cc1df71ed9d005389e0250678f5536a1ca6ac0b0e4a118ce6bbd4cc463b5
  wasm32 linear parse-i64-large        3355 bytes f2d3780d8ed15fb2baaee0700b65afa6c1e8c47caf9887a456429b00c729412a
  wasm32-gc parse-i32-basic component 14291 bytes 2913c0459d659b8074968cd24c3ef3cc9df10f9f3265028ddc3c2d005a47e8c5
  wasm32-gc parse-i32-invalid component 12692 bytes 05111c253c06b4bdf800c704d1e4c5700a975865f7348f61bc7f48e17df83a81
  wasm32-gc parse-i64 component       16360 bytes 48fea7af4e299d09e1b7eff63eff00bad559598cf0a861b4a098c2c989d59c6f
  wasm32-gc parse-i64-large component 12427 bytes aa8f39c638ed101e5e555099508419abc7d483ea5cf1cb0567388296e3b32448
  ```

- This receipt covers only integer parse Result records and invalid-integer
  error values. f64 Result construction, invalid-float errors, parse I/O
  scratch buffers, generic Option/Result constructors, HashMap descriptors,
  closure environments, automatic collection, shared/cyclic source graphs,
  and complete cross-backend move/borrow parity remain open; #874 stays open.

## Progress receipt — 2026-09-20 (Floating-point parse Result allocation boundary)

- Implementation commit `c91051952` routes the fallback `Result<f64, String>`
  success record and invalid-float error values through `LINEAR_GC_ALLOC`.
  The `Ok<f64>` record remains 12 bytes; invalid-float keeps its 30-byte
  requested String allocation (rounded to the existing 32-byte allocator
  boundary) plus the 8-byte `Err` record. Tag, f64 payload, length-prefix,
  and `base + 4` String ABI layouts remain unchanged. The Wasm GC typed
  Result path is unchanged.
- The linear finalizer uses existing i32 scratch locals for the allocated
  record and String pointers, so the numeric scanner's fixed scratch addresses
  remain temporary parsing state rather than managed ownership.
- Positive fixtures `tests/fixtures/stdlib_string/parse_f64.ark`,
  `parse_f64_decimal.ark`, `parse_f64_invalid.ark`, and
  `parse_f64_negative.ark` compile, validate, and execute successfully on both
  the `wasm32` linear core and `wasm32-gc` component paths. All eight artifacts
  pass; the four integer parse fixtures were also revalidated on both paths
  after the shared emitter change.
- Verification after the change:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS (19.7s)
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  targeted fmt --check                                     PASS (4 files)
  wasm-tools validate                                       PASS (8 f64 + 8 integer artifacts)
  wasmtime run                                              PASS (8 f64 + 8 integer artifacts)
  ```

- Representative f64 artifact hashes (SHA-256):

  ```text
  wasm32 linear parse-f64               12346 bytes 0df96faa1b9f5b076ad992c3b64f1a817fae585d89817a500b7f707d3e3c633e
  wasm32 linear parse-f64-decimal        6441 bytes f5b6238b6bc9521b055f9708be87e5d85517c10d1afae55abc919f5a72a8eb78
  wasm32 linear parse-f64-invalid        4587 bytes a7d3f3b356058dd20f5c70f74151bbc5146073e6b39cc2469cf43684b83d8a31
  wasm32 linear parse-f64-negative       5979 bytes 6c1cd50d4f504a57e0f6a47da9ec13f2e24f9337ce84eae31466f75f25a1a23b
  wasm32-gc parse-f64 component         22113 bytes c4c1d7bbc548ebceef8dd2f79940b996fb50cd7482d8b44217be8095e38a3422
  wasm32-gc parse-f64-decimal component 15171 bytes 7c8ad53955f90df744da98564efef227ef24783a2b4c538bb3cc9d34d5c124cd
  wasm32-gc parse-f64-invalid component 13120 bytes e898a0397871c72b9b6f1350edebbe79c34e455b548adbb7a52c4ad1e650c68f
  wasm32-gc parse-f64-negative component 14629 bytes ac6d85457bb8650613b5c17670000b3962a2a2c6726deca6c56559a7a9e27927
  ```

- This receipt covers f64 parse Result records and invalid-float error values
  only. Parse scanner I/O scratch buffers, generic Option/Result constructors,
  HashMap descriptors, closure environments, automatic collection,
  shared/cyclic source graphs, and complete cross-backend move/borrow parity
  remain open; #874 stays open.

## Progress receipt — 2026-09-20 (Range construction allocation boundary)

- Implementation commit `8e720d28f` routes the fallback `core::range_new`
  record through `LINEAR_GC_ALLOC`. The Range struct keeps its existing
  8-byte layout (start at offset 0, end at offset 4); only the allocation
  owner changes from direct global-bump manipulation to the private
  allocator. The Wasm GC path continues to construct the typed Range struct.
- Positive fixtures `tests/fixtures/stdlib_core/range_contains_edges.ark`,
  `range_len_empty.ark`, `range_inclusive_contains.ark`, and
  `range_inclusive_edges.ark` compile, validate, and execute successfully on
  both the `wasm32` linear core and `wasm32-gc` component paths. All eight
  artifacts pass, covering half-open bounds, empty/inverted ranges, and
  inclusive endpoints.
- Verification after the change:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS (19.9s)
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  targeted fmt --check                                     PASS (1 file)
  wasm-tools validate                                       PASS (8 artifacts)
  wasmtime run                                              PASS (8 artifacts)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32 linear range-contains-edges        3152 bytes f90f166515867dee3bf936ba7a01b3ba8ddf4d6c322d70bf6a3bc8197badfaf0
  wasm32 linear range-len-empty             4170 bytes a37b046d8eab31d2a8d058b91f0a120a8bf52bc18582b408e69bcab1aa63bb94
  wasm32 linear range-inclusive-contains    3515 bytes 660af7c07d8582e094e0de9a8243330b9a5f7d45ab58020fd0c13579ea3f64c6
  wasm32 linear range-inclusive-edges       3936 bytes 4eafda16807ab6f630f85ca138039f9ccbc50a86d9b8ef1e3401fa2e2d8db23d
  wasm32-gc range-contains-edges component 11426 bytes 241bc318a94d668ef80cd4e734085948d00887c9533cf4eed834d6205c7f4819
  wasm32-gc range-len-empty component      12261 bytes 27147066358463ac655e3939f2c5a8ed0c7602205fc1355d6fb81ed042b751d7
  wasm32-gc range-inclusive-contains comp. 11770 bytes f38e54aa7a282566934f5930dd183f8c961df0a46f023242702a20ef9baf6830
  wasm32-gc range-inclusive-edges comp.    12037 bytes 75db858c1ad1a750f6ca5a84791901a3c104ff9d0c89a355c8b1faa3065df56f
  ```

- This receipt covers only the `core::range_new` fallback record.
  `RangeInclusive` uses the generic struct constructor already covered by the
  aggregate slice. Sequence take/skip/unique Vec allocation, generic
  Option/Result constructors, HashMap descriptors, closure environments,
  automatic collection, shared/cyclic source graphs, and complete
  cross-backend move/borrow parity remain open; #874 stays open.

## Progress receipt — 2026-09-22 (seq take/skip/unique Vec allocation boundary)

- Implementation commit `15e0bfe75` routes the linear-memory Vec header
  (12 bytes) and backing storage (`result_len * 4` for take/skip,
  `src_len * 4` for unique) created by `seq::take_i32`, `seq::skip_i32`,
  and `seq::unique` through `LINEAR_GC_ALLOC`. The existing
  `[len, cap, data_ptr]` header layout, the `len = 0` initialization for
  unique, and the `memory.copy` payload duplication are unchanged. The
  raw-bump-only `emit_seq_slice_reserve_payload` helper was removed because
  the allocator now reserves the backing payload directly. The Wasm GC typed
  array lowering is unchanged.
- Contract coverage: `test_managed_aggregate_constructors_use_private_allocator`
  now asserts both seq files allocate through the private allocator and no
  longer reference `emit_heap_set`.
- Positive fixtures and outputs: `tests/fixtures/stdlib_seq/seq_take_skip_fold.ark`
  produces `3 1 3 3 3 5 15 120 5 0` on both backends (including the
  over-length take and empty skip edges); `seq_edge_cases.ark` produces
  `2 0 4 -1 1 9 26 42 42 42 true false 3 3 0`; `seq_basic.ark` produces
  `14 1 5 true false 2 3 4`. All three compile, validate, and run on the
  wasm32 linear core and the wasm32-gc P2 component paths.
- Verification receipts:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS (22.6s)
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  targeted fmt --check                                     PASS (2 files)
  wasm-tools validate                                       PASS (6 artifacts)
  wasmtime run                                              PASS (6 artifacts)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32 linear seq_take_skip_fold        12241 bytes 892ff26b01e8f973b16fbe7277e20d9ab5e4aca0a476297f455b73f356fa9a99
  wasm32 linear seq_edge_cases            16288 bytes 88f580ec49741c18fac926dfe8287a62a35298cc256185a8c217dac55bf8bcf3
  wasm32 linear seq_basic                  6590 bytes ad00633087b3733e03b14b5cf91e795cd305c63f14852a2817851191ef5195bc
  wasm32-gc seq_take_skip_fold component  20414 bytes b32bf903e190047790a33a2f5ded772621773f6cabf87dd3708d8b35a3385435
  wasm32-gc seq_edge_cases component      24549 bytes 5a7a62d6efe1e63ced17df1796660444a4dfc7f5dbaaa188a09a5b4356df6da9
  wasm32-gc seq_basic component           15266 bytes 6ab3e411aeda68550c6644d9dcd7497493ef2a1e59bcc88901b1766f42f41932
  ```

- This receipt covers only the seq take/skip/unique Vec allocation boundary.
  Generic Option/Result constructors, HashMap descriptors, closure
  environments, descriptor-aware root writes, automatic collection,
  shared/cyclic source graphs, and complete cross-backend move/borrow parity
  remain open; #874 stays open.

## Progress receipt — 2026-09-22 (env/argv ABI allocation boundary)

- Implementation commit `d752e6a4a` routes the linear-memory env/argv ABI
  allocations through `LINEAR_GC_ALLOC`. The 8-byte
  `Option<String>` record ({tag, payload}) built by
  `emit_env_var_option_none`/`emit_env_var_option_some`, the argv
  pointer table (`argc * 4`) and byte buffer, the `env::args` Vec
  header (12 bytes) and data slots (32 bytes on P1,
  `align4((argc-1) * 4)` on P2), and the cstr->String copies
  (`clen + 4`) now allocate via `emit_heap_alloc_from_stack` instead of
  raw global-0 bump. The raw-bump-only helpers
  `emit_env_var_option_alloc`, `emit_env_var_option_ptr_on_stack`, and
  the dead `emit_env_missing_var_option` were removed.
  `emit_env_var_align_heap` intentionally keeps raw
  `emit_heap_get`/`emit_heap_set` because it aligns the bump pointer
  itself rather than allocating a managed record. The Wasm GC
  `*_gc` variants and the P2 component paths are unchanged.
- Contract coverage: `test_linear_gc_runtime_contract.py` now asserts the
  env option/args/arg-populate/arg-at/has-flag files allocate through the
  private allocator and no longer reference `emit_heap_set` (the lookup
  file is exempted from the `emit_heap_set` assertion because of the
  intentional heap-alignment helper).
- Positive fixtures and outputs: all nine fixtures compile, validate, and
  run on both the wasm32 linear core and the wasm32-gc P2 component paths.
  `stdlib_env/env_basic` prints `3 3` on both;
  `stdlib_env/env_var_lookup` prints `found PATH` / `none as expected`
  on wasm32-gc P2 and `PATH not found` / `none as expected` on wasm32
  P1 (the pre-existing P1 environ-import limitation, unchanged by this
  slice); `stdlib_env/env_var_default` and `env_var_or_default` print
  identical output on both; `stdlib_host/wasi_args` prints
  `wasi_args ok` on both; `stdlib_cli/cli_basic` prints
  `3 true --` on wasm32 P1 and `3 true no arg` on wasm32-gc (the GC
  `arg_at` stub, pre-existing); `stdlib_cli/cli_flag_check` and
  `wasi_p2_native/args`/`env_var` produce matching expected output on
  both backends.
- Verification receipts:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS (20.5s)
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  targeted fmt --check                                     PASS (9 files)
  wasm-tools validate                                       PASS (18 artifacts)
  wasmtime run                                              PASS (18 artifacts)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32 linear cli_basic           2617 bytes 9ed09ee5bdddb6317ab1e9099258d3f5ce910d7ed516360cb344e39bc8856172
  wasm32 linear env_basic           2233 bytes 4de9ad690ef5ffd4ab0328be7a45a2600eb921edc46f394e65c82c4c9a3959de
  wasm32 linear env_var_lookup      2437 bytes c98caa305e9aefebf33846ce7875acced29d097f0346c08b11f805f65ae62a24
  wasm32 linear args                2020 bytes cb2b91d349f8d7f54b213dfba24246bfce25a7b6740d42d887954fab9d8a0ec0
  wasm32-gc cli_basic component    11004 bytes 2c23b25d2c28e8b2e5d19887178eedf0e642e66368d17cc15e8f903e505df780
  wasm32-gc env_basic component    10478 bytes 650c53760b1414db548670b3931d27d951258393c2992fbfad227b2b53b66005
  wasm32-gc env_var_lookup comp.   12079 bytes 5350bc84a48028a3f97ebea2bdb2d7bd53d7b10d05f2ad6b618b940be3fc6a8f
  wasm32-gc args component         10122 bytes 194aff524d7530922fe71eddde78941cf35d54fceecc94de34be048037048fb2
  ```

- This receipt covers only the env/argv ABI allocation boundary. Host Result
  marshalling (`result_helpers`, fs/stdio read-write buffers), generic
  Option/Result constructors, HashMap descriptors, closure environments,
  descriptor-aware root writes, automatic collection, shared/cyclic source
  graphs, and complete cross-backend move/borrow parity remain open; #874
  stays open.

## Progress receipt — 2026-09-23 (fs read/write result records and buffers)

- Implementation commit `26ff0f517` routes the linear-memory fs host-ABI
  allocations through `LINEAR_GC_ALLOC`. The 8-byte {tag, payload}
  `Result` records built by `emit_fs_read_result_pair`,
  `emit_fs_write_success_unit_result`, and `emit_fs_write_error_result`,
  the 19/20-byte error String records from
  `emit_fs_read_open_error_message`/`emit_fs_write_error_message`, and the
  `align4(bcount)` staging buffer in `emit_fs_write_bytes_buffer` now
  allocate via `emit_heap_alloc_from_stack`. The error-message builders
  gate at emit time: wasm32 uses the managed allocator while wasm32-gc keeps
  the raw scratch staging that the GC result builders copy out of. The
  streaming fs_read/stdin chunk loops still claim bytes with per-chunk bump
  commits (single-record consolidation is a follow-up), and the GC-only P2
  canonical result area stays on raw scratch.
- Contract coverage: `test_linear_gc_runtime_contract.py` now asserts the
  fs read result-pair, write unit/error result slices, and the write-bytes
  buffer allocate through the private allocator without `emit_heap_set`.
- Positive fixtures and outputs: `stdlib_io/fs_read_write` prints
  `write ok` / `hello from arukellt` on both backends;
  `stdlib_io/fs_read_error` prints `file open error` on both;
  `quickstart/fs_read` prints `Hello from file!` on both;
  `wasi_fs_p2` prints `hello p2 fs` on both; an ad-hoc probe exercising
  `fs::write_bytes` + read roundtrip + write-error + read-error prints
  `wb ok` / `Hi!` / `file write error` / `file open error` identically
  on the wasm32 linear core and the wasm32-gc P2 component paths.
- Verification receipts:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  targeted fmt --check                                     PASS (6 files)
  wasm-tools validate                                       PASS (11 artifacts)
  wasmtime run                                              PASS (11 artifacts)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32 linear fs_read_write          3852 bytes e32ffa38846aaa04f92b726a2e17a06c659807ae9081205edcc3d2a7a5ff5c34
  wasm32 linear fs_read_error          2411 bytes cf8f784b776a6126bfb07ad35e336bc09c4b60ceaaf4096a3641e477a95839d7
  wasm32 linear fs_slice_probe         6373 bytes 2c0b23c7e6b3386ee59fa6003ba6fe5904ef0247ba1d1f58536d7deccbe46c3d
  wasm32-gc fs_read_write component   14181 bytes 82b2ab55e668a20bbcaaf9dc52e54be4a394cf6e98bab1511da05a0045e3f404
  wasm32-gc fs_read_error component   11121 bytes 6fff5002ca738d05bb2cc44cfd1ac0c9a216e024c80bdd99192d8008bc622522
  wasm32-gc fs_slice_probe component  17856 bytes 1007184b4851ff9efdb55c5f4ae3e114f5fa4fc14b25107439f822280ddc1b52
  ```

- This receipt covers only the fs read/write result-record and staging-buffer
  boundary. The streaming fs_read/stdin read loops still split the assembled
  String across per-chunk bump records (single-record consolidation needed
  before automatic collection), the dead `emit_result_*` marshalling helpers
  in `result_helpers.ark` await removal-or-conversion, generic Option/Result
  constructors, HashMap descriptors, closure environments, descriptor-aware
  root writes, automatic collection, shared/cyclic source graphs, and
  complete cross-backend move/borrow parity remain open; #874 stays open.

## Progress receipt — 2026-09-23 (streaming read buffers as single records)

- Implementation commit `0c4696759` adds
  `helpers_core_heap::emit_heap_advance_uncommitted` (global.set +
  ensure-grown without the commit-top hook) and switches the fs_read P1/P2
  and stdin read loops to it. The 4-byte length-prefix reserve and every
  per-chunk claim now advance the heap without recording; the finalize
  step (`emit_fs_read_success_len_header`,
  `emit_finish_stdin_string_buffer`,
  `emit_finalize_stdin_string_buffer`) fires one `emit_heap_set` that
  commits the whole `[BUFSTART, heap)` range as a single managed record.
  Previously each chunk bump committed its own record, so collection could
  have reclaimed tail chunks of a still-live String. On wasm32-gc the
  emitted shape is effect-identical because the commit index is unset there
  and `heap_set` was already a plain scratch bump.
- Contract coverage: `test_linear_gc_runtime_contract.py` now asserts the
  stdin begin/advance and both fs_read loop slices use
  `emit_heap_advance_uncommitted` and contain no `emit_heap_set`.
- Positive fixtures and outputs: `stdlib_io/fs_read_write`,
  `fs_read_error`, `quickstart/fs_read`, and the write_bytes probe print
  identical output on both backends; `read_stdin_empty` fed piped input
  surfaced the full multi-line content through the single-record buffer on
  both backends (the fixture asserts empty stdin, so the surfaced content
  demonstrates the read path end to end).
- Verification receipts:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  targeted fmt --check                                     PASS (4 files)
  wasm-tools validate                                       PASS (12 artifacts)
  wasmtime run                                              PASS (12 artifacts)
  ```

- This receipt covers only the streaming-read single-record boundary. The
  dead `emit_result_*` marshalling helpers in `result_helpers.ark` await
  removal-or-conversion, generic Option/Result constructors, HashMap
  descriptors, closure environments, descriptor-aware root writes,
  automatic collection, shared/cyclic source graphs, and complete
  cross-backend move/borrow parity remain open; #874 stays open.

## Progress receipt — 2026-09-23 (root-slot coverage for all managed local writes)

- Implementation commit `f3c56bbef` closes the root-update coverage gap
  that blocked automatic collection. Previously only `MIR_LOCAL_SET`
  mirrored writes into the root frame; intrinsic results stored through
  `inst_store::emit_store_action` (SET/TEE to `MirInst_dest`), the linear
  `struct_new`/`array_new` constructors, `array_set` element staging,
  the non-GC `REF_TEST`/`REF_CAST` fallbacks, and `future_new`/`await`
  copies all wrote MIR locals without updating their root slots, so a
  fresh managed pointer could be invisible to the collector (a false
  negative that ADR-075 forbids). `emit_update_after_local_set` is now
  ctx-free: `emit_frame_enter_and_push` installs a per-function
  slot-offset table plus the frame-cursor local into `WasmWriter`, and the
  update helper reads them back, so the generic store paths needed no
  signature changes. Scratch-slot and SIMD-lane writes are unaffected
  (they never carry managed pointers; the table returns -1 for them).
- Contract coverage: `test_linear_gc_runtime_contract.py` still asserts
  the root-frame wiring and now checks the ctx-free update signature; the
  `LINEAR_GC_COLLECT` non-emission assertion is unchanged because
  automatic collection remains disabled.
- Positive fixtures and outputs: `enums/enum_struct_variant` prints
  `75|24`, `collections/vec_option_i32` prints `7`,
  `stdlib_hashmap/hashmap_insert` prints `OK`, and
  `stdlib_io/fs_read_write` prints `write ok|hello from arukellt|`
  identically on the wasm32 linear core and the wasm32-gc P2 component
  paths; `stdlib_string/string_concat` passes its internal assertions on
  both. WAT inspection of the linear artifacts shows
  `local.get frame; i32.const slot; i32.sub; local.get v; call root_update`
  after every `local.set`/`local.tee` into a root local and no such
  sequence after scratch writes.
- Verification receipts:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS (19.9s)
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (1 test)
  targeted fmt --check                                     PASS (9 files)
  wasm-tools validate                                       PASS (9 artifacts)
  wasmtime run                                              PASS (9 artifacts)
  ```

- This receipt covers only managed-local write coverage. Managed pointers
  held only in scratch locals or on the operand stack at an allocation
  point are still not collectible-safe, `LINEAR_GC_COLLECT` is still never
  invoked, HashMap descriptors/exact pointer-field metadata, closure
  environments, automatic collection, shared/cyclic source graphs, and the
  complete move/borrow parity audit remain open; #874 stays open.

## Progress receipt — 2026-09-23 (watermark-gated collection at function entry)

- Implementation commit `2aaadcbf4` turns on automatic collection for the
  linear fallback. A new mutable global `GLOBAL_LINEAR_GC_ALLOC_WATERMARK`
  (index 7) accumulates the aligned payload bytes of every
  `LINEAR_GC_ALLOC` call; `LINEAR_GC_COLLECT` resets it to 0. After a
  function's root pushes, `emit_frame_enter_and_push` emits
  `global.get watermark; i32.const LINEAR_GC_COLLECT_THRESHOLD_BYTES;
  i32.ge_u; if; call collect; end`, so collection fires at function entry
  once 64 MiB of payload has been allocated since the previous sweep.
- Placement is the soundness point: at function entry the operand stack
  is empty and every managed local of every live frame is already
  mirrored into a root slot (previous receipt), so the mark phase cannot
  produce a false negative. The only values suspended on caller operand
  stacks are the call's arguments, which are now rooted params of the
  current frame. An alloc-triggered collection was rejected because
  managed pointers can sit below the size argument on the operand stack
  at allocation sites.
- The 64 MiB threshold amortizes the full-heap mark-sweep for the
  selfhost compiler, which allocates hundreds of MiB per compile on the
  linear path; small programs may never collect, in which case
  `memory.grow` still backs allocation — collection bounds footprint, it
  is not required for correctness.
- E2E coverage: `test_linear_gc_runtime_e2e.py` gains
  `test_function_entry_collects_when_watermark_exceeds_threshold`, which
  appends a probe that allocates unrooted garbage, forces the watermark
  past the threshold, calls the compiled `$main`, and asserts the
  watermark was reset and the free list became non-empty — proving the
  entry gate ran the collector and reclaimed unreachable records. The
  rooted-cycle probe still passes.
- Verification receipts:

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS (20.2s)
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (2 tests)
  targeted fmt --check                                     PASS (6 files)
  wasm-tools validate                                       PASS (probe artifacts)
  wasmtime run --invoke                                     PASS (2 probes)
  ```

- This receipt covers gated automatic collection only. Closure
  environments on the fallback heap, HashMap descriptors / exact
  pointer-field metadata (the scan stays conservative, which ADR-075
  permits for bootstrap), shared/cyclic source-level graphs, and the
  complete move/borrow parity audit remain open; #874 stays open.
