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
