---
Status: done
Created: 2026-09-25
Updated: 2026-09-26
ID: 881
Parent: 691
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #691, #874"
Orchestration class: implementation-ready
Orchestration upstream: 691
Blocks v{N}: none
Priority: 2
Source: "#691 PeekIter implementation — 2026-09-25"
---

# 881 — Option<T> record field と literal の GC type index を一致させる

## Scope

wasm32-gc で `Option<T>` を record field に持つ struct を struct literal で
構築すると、field 宣言側の `Option<T>` type index と `None`/`Some`
literal が生成する type index がずれ、invalid な core module になる。

## Reproduction

2026-09-25 に `PeekIter<T>` で再現:

```ark
pub struct PeekIter<T> {
    vec: Vec<T>,
    index: i32,
    length: i32,
    peeked: Option<T>,
    has_peeked: bool,
}

PeekIter { vec: v, index: 0, length: n, peeked: None, has_peeked: false }
```

同一コンパイル内で `Option<i32>` が 3 つの異なる type index
(field 宣言=78 field 3 の `(ref null 28)`、`peekable` 内 `None`
literal の `struct.new_default 79`、`peek` 内 `Some` 構築の
`struct.new_default 80`) として生成され、`wasm-tools validate` が
`type mismatch: expected (ref null \$type), found (ref null \$type)`
で func を拒否する。linear target では同じコードが通る。

## Workaround in tree

`PeekIter<T>` は `peeked` を single-slot `Vec<T>` で表現している
(`519f5c4c6`)。この issue は record field 型としての `Option<T>`
一般対応が必要になったときに解消する。`Option<T>` field を必要とする
他の実装が現れなければ YAGNI で閉じてよい。

## Acceptance

- [x] `Option<T>` を field に持つ record を `None`/`Some` で構築した
      module が wasm32-gc で validate する
- [x] 同一コンパイル内の `Option<T>` が単一の type index に正規化される
- [x] linear backend での回帰がない

## Required evidence

- implementation commit SHA
- positive fixture (Option<T> record field build on wasm32-gc)
- validate output

## Second repro — 2026-09-26 (generic Vec<T> field, same family)

#883-A の調査中に同族の type index ずれを \`Vec<T>\` field で確認した。

```ark
pub struct ScanBox<T> {
    vec: Vec<T>,
    index: i32,
    length: i32,
    state: T,
    f: fn(T, T) -> Option<T>,
}

let v: Vec<i32> = Vec::new<i32>()
let it = ScanBox { vec: v, index: 0, length: 3, state: 0, f: sum2 }
```

mono 化された \`ScanBox_T\` (T=i32) の field 0 が erased-generic
\`{ (ref null (array structref)), i32 }\` layout を引く一方、main 側の
\`v: Vec<i32>\` は \`{ (ref null (array i32)), i32 }\`。
\`struct.set ScanBox_T 0\` が
\`type mismatch: expected (ref null $type), found (ref null $type)\`
で func 20 (main) を拒否する。linear では正しく動く (1,3 と出力)。
scalar field (index/length/state) と fnsig field は mono 解決済みで、
nested container field だけが未解決 — #882 で直した payload bind 系と
同じく、type-arg map が nested \`Vec<T>\` の field layout 構築に
適用されていない。

## Progress receipt — 2026-09-26 (done; issue closing)

Commit: \`a6bc67d39\` on \`docs/adr-058-language-v2-semantics\`.

Root cause and fix:

- \`mir_struct_lit_instance_name\` returned the base
  (erased-generic) struct layout whenever
  \`struct_lit_unify_segments\` could not align a field's declared
  type with the local's value type.  Two field shapes could never align
  segment-wise even when every type parameter was already bound by
  other fields:
  - fn-valued fields: declared \`fnsig:T,T;option:T\` vs the opaque
    pair value type (\`fnref\` / \`fnref2\` /
    \`fnsig:\` spellings).  ScanBox<T> fell back to the base
    layout, so \`vec:Vec<T>\` stayed erased
    (\`{array structref,i32}\`) against a concrete \`Vec<i32>\`
    store -> GC validate type mismatch (second repro).
  - enum fields: declared \`option:T\` / \`result:\` /
    \`enum:X\` vs the variant-literal local type
    \`enum:Option\` / \`enum:Result\`.  PeekBox<T>
    \`peeked:Option<T>\` vs \`None\` / \`Some\` hit
    the same fallback (first repro).
- New fallbacks \`struct_lit_fn_field_already_bound\` and
  \`struct_lit_enum_field_already_bound\` let the field pass iff
  every type parameter mentioned in the field type is already bound
  (\`struct_lit_params_bound\`, \`struct_lit_decl_is_fn_value\`,
  \`struct_lit_enum_head_matches\` mapping \`option:\` ->
  \`enum:Option\`, \`result:\` -> \`enum:Result\`,
  \`enum:X\` -> \`enum:X\`).  Both the fn-pair ABI and the
  shared enum base accept the value as-is, so the binding only needs
  the inst's params already fixed; \`ctx_derive_generic_struct_layout\`
  then token-substitutes the concrete field layout.

Evidence:

- Positive fixture \`tests/fixtures/generics/struct_option_vec_fn_fields.ark\`
  (run + t3-compile + t3-run): \`ScanBox<i32>\` with Vec/fnsig
  fields including \`(sb.f)(2,3)\` -> Some(5), and
  \`PeekBox<i32>\` built with \`None\` and \`Some(7)\`.
  Prints \`0 5 none 7\` identically on wasm32 linear memory32
  and wasm32-gc component; \`wasm-tools validate --features all\`
  clean on both backends.
- Type-index normalization: in the GC module the \`Option<i32>\` field
  declaration is \`(ref null 28)\` (shared enum base), \`None\`
  -> \`struct.new_default 54\` (sub 28), \`Some\` ->
  \`struct.new_default 55\` (sub 28); PeekBox field 2 stores and
  reads agree.  The reported three-index split (78/79/80) is gone.
- Regression: generics 8/8 (includes new fixture), functions 16/16,
  collections 15/15, closure_capture 6/6 compile-validates;
  stdlib_trait FAIL=25 - identical pre-existing failure list
  (i64 typecheck + Ordering enum) as the #883 baseline.
- verify lane PASS (quality changed).

Residuals (outside this issue's scope):

- \`PeekIter<T>\`'s Vec-based \`peeked\` workaround
  (\`519f5c4c6\`) stays in tree; reverting to \`Option<T>\` is
  now unblocked but optional.
- Parameterized enum heads beyond \`option:\` / \`result:\` /
  self-name \`enum:X\` do not use the new fallback (YAGNI until
  a repro exists).
