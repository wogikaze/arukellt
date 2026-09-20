---
Status: open
Created: 2026-09-19
Updated: 2026-09-20
ID: 852
Track: language-v2
Depends on: "880"
Related: "ADR-058, RFC-011, #691, #694, #729"
Orchestration class: blocked-by-upstream
Orchestration upstream: 880
Blocks v{N}: language-v2
Priority: 1
Source: "ADR-058 / RFC-011 sub-issue umbrella"
---

# 852 — 言語 v2 意味論再設計 umbrella

[ADR-058](../../docs/adr/ADR-058-language-v2-semantics-redesign.md) / [RFC-011](../../docs/rfcs/011-language-v2-semantics-redesign.md) の親 tracker。

**この issue では実装しない。** #853–#880 の focused child issue で進め、原則 1 issue = 1 logical commit とする。#880 が完了するまで #852 は close しない。

## Child issues

### Frontend semantics

- [x] #853 — Ownership / move / Copy / mutability を確定・実装する — RFC 1–4 — depends: none
- [x] #854 — ADT / Box / slice の値意味論を統一する — RFC 5–7, 25 — depends: 853
- [x] #855 — Closure capture / escape / lifetime を実装する — RFC 10 — depends: 853

### Trait / API / type system

- [x] #856 — Trait core / coherence / generic arity を確定する — RFC 12, 17 — depends: none
- [x] #857 — `?` / conversion / Error sentinel を整理する — RFC 13, 19 — depends: 694, 856
- [x] #858 — 演算子 Trait へ統一し magic method を退役する — RFC 14 — depends: 856
- [x] #859 — 公開 API を generic Trait 面へ統一する — RFC 15, 16, 18, 26 — depends: 856
- [ ] #860 — Iterator Trait 体系を統合完了する — RFC 37 — depends: 691, 856
  - Progress (not done, 2026-09-20): `b0849dee4` adds the concrete
    `MapTakeIter` composition and verifies GC/linear parity. `6c92fa741`
    extends the generic
    `for` evidence to a composed `MapFilterTakeIter` source and verifies
    identity `IntoIterator` plus canonical `next` on both backends. `e9ee279b5`
    extends the generic
    `for` fixture to `Range` and `RangeInclusive` direct expressions and
    verifies separate monomorphized `into_iter`/`next` sequences on both
    backends. `5492a9d81` extends the generic
    `for` fixture with `sum_into_for<T: IntoIterator>` and verifies the
    monomorphized `Vec<i32>` source through `Vec::into_iter` and canonical
    `VecIter::next` on both GC and linear Wasm. `c50043518` adds a generic
    `sum_for<T: Iterator>` fixture and verifies monomorphized `VecIter`
    lowering through identity `IntoIterator::into_iter` and canonical `next`
    on both GC and linear Wasm. `8384f8115` routes the eager map/filter
    compatibility helpers through the same lazy adapter and canonical
    `collect` path. `33a604056` routes concrete local
    `for` sources with a registered `IntoIterator` implementation through
    `IntoIterator::into_iter` and canonical `next` in both AST and CoreHir
    lowering. Vec, Range, and RangeInclusive direct-for fixtures pass current
    source GC component and linear Wasm compile/validate/runtime checks.
    `0a6cb7b81` adds and verifies the concrete method-shaped
    `iter().map(...).filter(...).take(...).collect()` pipeline on both
    backends, including the nested CoreHir method-lowering fix. Generic item
    representations, additional adapter composition, unresolved generic or
    unregistered source expressions, the required negative/diagnostic fixture,
    and the final RFC-011 residual audit remain open. `c064cb46a` additionally routes
    direct concrete call/method `for` sources through the same AST/CoreHir
    `IntoIterator` and canonical `next` path; both target receipts pass.
    `5a1408aac` adds the concrete `FilterTakeIter` composition and verifies
    lazy collection plus generic `IntoIterator` lowering on GC and linear Wasm.
    `317583de9` adds the concrete `SkipTakeIter` composition and verifies
    skip-before-take collection plus generic `IntoIterator` lowering on both
    Wasm backends. `d31cb9ec3` routes the concrete `std::seq` compatibility
    wrappers through canonical `std::core::iter` consumers and verifies
    identical GC/linear output plus WAT call targets for map/filter/take/skip/
    fold. Arbitrary item representations, broader adapter composition,
    manifest-level deprecation, and the RFC-011 residual audit remain open.
    `da440bc97` extends the concrete String lane with skip-before-callback
    `map`/`filter` adapters and verifies `collect`, `Vec::from_iter`, and
    generic `IntoIterator` consumption on both targets. `0115f6619` adds the
    corresponding numeric `Vec<i32>` adapters, including `FromIterator` and
    numeric `sum` capability evidence. These are bounded iterator receipts;
    they do not complete #691/#860 or the arbitrary item-generic boundary.
- [x] #864 — let-generalization / value restriction を形式化する — RFC 27 — depends: 856

### Numeric / text / syntax

- [x] #861 — 数値の暗黙変換を廃止し明示変換を定義する — RFC 20, 21 — depends: none
  - Receipt: completed in `549fc4e1a` plus fixture fix `72c188934` and
    post-close migration fix `2bfeeede3`; ADR-067,
    RFC-011, and the language spec are synchronized. Positive and diagnostic
    fixtures prove explicit conversion and rejection of mixed concrete numeric
    arithmetic; GC and linear compile/validate/runtime receipts both pass.
- [x] #862 — 算術 edge case と評価順序を仕様化する — RFC 22, 23 — depends: none
  - Receipt: completed in `a720c2cc7`; ADR-068, RFC-011, and the language spec are synchronized. Integer/float edge rules and place-before-RHS evaluation are covered by positive, diagnostic, and GC/linear runtime fixtures. Canonical `build-compiler` remains blocked by the pre-existing #854 `unbox`/`slice_get` bootstrap mismatch; the targeted shim receipt is explicitly not a canonical rebuild.
- [x] #863 — String / Unicode semantics を確定する — RFC 24 — depends: 854
  - Receipt: completed in `e4becb596`; ADR-069 and RFC-011 item 24 are synchronized. Valid UTF-8 String/scalar semantics, byte/scalar API separation, strict/lossy UTF-8 construction, boundary-aware slicing, and `char_at` migration are covered by positive/diagnostic fixtures. Current-source `wasm32-gc` and `wasm32` targeted compile/validate/runtime parity passes; canonical bootstrap remains blocked by the pre-existing #854 `unbox`/`slice_get` mismatch.
- [ ] #865 — Hygienic desugaring と f-string lowering を実装する — RFC 28, 29 — depends: 729
  - Progress (implementation complete, close blocked by upstream, 2026-09-20):
    `9783627b5` plus fixture equivalence follow-up `b83d87ccf`; ADR-077,
    RFC-011 items 28–29, language spec, and current-state are synchronized.
    F-string receiver lowering, compiler-owned iteration / comprehension calls,
    shadowing immunity, evaluation order, and core-form equivalence are covered
    by positive and diagnostic fixtures. Current-source GC core/component and
    linear compile/validate/runtime outputs match. #729 remains open, so #865
    is not a done receipt; the final cross-backend audit remains under #875.
- [x] #866 — source module の import / use 構文を整理する — RFC 30 — depends: none
  - Receipt: completed in `963d544a6`; `use` is the canonical source-module
    syntax, legacy local `import` emits W0101 during migration, and alias,
    selective import, public-only `pub use`, loader/resolver/formatter/LSP,
    and the normative RFC/spec documents are synchronized. Positive,
    negative, and diagnostic fixtures pass the targeted selfhost checks; GC
    and linear current-source compile/validate/runtime receipts agree. The
    canonical bootstrap remains blocked by the pre-existing #854
    `unbox`/`slice_get` mismatch. WIT/component-boundary import work remains
    explicitly scoped to #867.
- [x] #867 — `pub` / Component export / WIT import 境界を分離する — RFC 11, 31, 38 — depends: 866
  - Receipt (done, 2026-09-20): implementation `d1f3014d9`; ADR-071,
    RFC-011, language spec, and import-system spec synchronized. Current-source
    s2 positive/negative/diagnostic fixtures prove source visibility, explicit
    Component export, full-version WIT identity, and source/WIT namespace
    separation. Official P2 packaging and WIT import gates 654, 666, and 670
    pass; the linear `wasm32/wasi-p1` fallback also compiles, validates, and runs
    the positive source fixture. See [#867](../done/867-component-wit-visibility-boundary.md).
- [x] #868 — Prelude / boolean literal / keyword policy を固定する — RFC 32, 33, 41 — depends: none
  - Receipt (done, 2026-09-20): implementation `c19a95bb1`; ADR-072,
    RFC-011, language spec, machine-readable keyword policy, stdlib manifest,
    playground, and parser/formatter/diagnostic fixtures are synchronized.
    Current s2 diagnostics for boolean and future-reserved names pass, the
    formatter and playground suite pass, and the guarded bootstrap overlay
    permits current-source `build-compiler`. The unrelated capability-stub
    diagnostic-column golden remains an open parity follow-up. See
    [#868](../done/868-prelude-booleans-keywords.md).
- [x] #869 — multi-clause / where / semicolon grammar を確定する — RFC 34–36 — depends: none
  - Receipt (done, 2026-09-20): implementation `203029f18`; ADR-073,
    RFC-011, language spec/syntax, positive/negative fixtures, and formatter
    fixture are synchronized. Current s2 component compile/validate/runtime
    produces `high` / `low`, the semicolon boundary returns the expected
    duplicate-definition diagnostic, and the formatter fixture is idempotent.
    The broad fixture gate retains six unrelated legacy failures; they are not
    hidden by this child receipt. See
    [#869](../done/869-multi-clause-where-semicolon-grammar.md).

### Targets / backends

- [x] #870 — target capability / Memory64 / host profile を分解する — RFC 9, 39, 40 — depends: none
  - Receipt: completed in `b6b7b2e0f` plus backend correction `10fdbe597`; ADR-074 and the target contract data are synchronized. GC/Memory64, WASI profile, and Component artifact axes are independent; the same HashMap source now packages and runs as the default GC P2 component, while the linear fallback remains validated. See [#870](../done/870-target-capability-configuration.md).
- [x] #871 — Wasm GC aggregate lowering を完成する — RFC 8 (aggregate) — depends: 854, 870
  - Receipt (done, 2026-09-20): implementation commits `402326820`,
    `c107e7b42`, and `38cc7cc71` provide the explicit TypeId-owned aggregate
    plan, GC instruction path, and Memory64 scalar-width correction.
    `f38ffe5b3` extends the executable guard to all nine enum fixtures plus
    struct/Vec, Option/Result, and tuple fixtures. The GC P2/Memory64
    components contain concrete `struct.new/get` and `ref.cast` operations;
    the linear P1/memory32 artifacts contain no GC instructions. All 13
    fixtures validate and run with matching stdout on both targets, and the
    non-exhaustive enum diagnostic is rejected before backend emission. See
    [#871](../done/871-wasm-gc-aggregate-lowering.md).
- [ ] #872 — Wasm GC closure / collection lowering を完成する — RFC 8 (closure/collection) — depends: 855, 859, 860, 871
- [ ] #873 — linear-memory tracing GC fallback runtime を実装する — RFC 8 (fallback runtime) — depends: 870
  - Progress (not done, 2026-09-20): implementation `6cb9bb01f`; P2 Memory64 boundary adapter `47995d5bf`; guarded pinned-bootstrap overlay `8ecb6b2a6`; fixture migration `8ac3714b2`; runtime e2e receipt `055de4955`; fixture registration `2b4180c8b`. ADR-075, the linear-memory lowering plan, RFC-011 section 2.2, and the executable private-ABI runtime probe are synchronized. The probe validates rooted-cycle retention and unreachable-cycle reuse, while current-source GC/component and wasm32 fallback artifacts validate without Wasm GC instructions. `verify quick` was run for close review but remains red on unrelated #652/#798/quality/MIR/T3 baseline checks; pin refresh/fixpoint and managed-value/root lowering remain separate follow-up boundaries, so #873 stays open.
- [ ] #874 — fallback managed-value lowering を実装する — RFC 8 (fallback lowering) — depends: 854, 855, 859, 860, 873
- [ ] #875 — wasm32-gc / fallback の意味論 parity を証明する — RFC 8–45 parity — depends: 857, 858, 861, 862, 863, 864, 865, 867, 868, 869, 871, 872, 874

### Spec / migration / close

- [ ] #876 — stability / current-state / redesign の分類を整合する — RFC 42, 43 — depends: none
- [ ] #877 — normative examples と操作的意味論を実行可能にする — RFC 44, 45 — depends: 857, 858, 860, 875, 876
- [ ] #878 — Language v2 compatibility migration を完了する — RFC migration — depends: 875, 876
- [ ] #879 — Wasm GC / fallback の性能・サイズ receipt を取得する — RFC performance — depends: 871, 872, 874, 875
- [ ] #880 — Language v2 最終 close audit を実施する — RFC final — depends: 877, 878, 879

## Dependency DAG

矢印は upstream → blocked child。

~~~mermaid
graph TD
853-->854
853-->855
856-->857
694-->857
856-->858
856-->859
691-->860
856-->860
854-->863
856-->864
729-->865
866-->867
854-->871
870-->871
855-->872
859-->872
860-->872
871-->872
870-->873
854-->874
855-->874
859-->874
860-->874
873-->874
857-->875
858-->875
861-->875
862-->875
863-->875
864-->875
865-->875
867-->875
868-->875
869-->875
871-->875
872-->875
874-->875
857-->877
858-->877
860-->877
875-->877
876-->877
875-->878
876-->878
871-->879
872-->879
874-->879
875-->879
877-->880
878-->880
879-->880
880-->852
~~~

## RFC-011 coverage

| RFC item | child |
|---:|---:|
| 1–4 | #853 |
| 5–7, 25 | #854 |
| 8 | #871–#875 |
| 9, 39–40 | #870 |
| 10 | #855 |
| 11, 31, 38 | #867 |
| 12, 17 | #856 |
| 13, 19 | #857 |
| 14 | #858 |
| 15–16, 18, 26 | #859 |
| 20–21 | #861 |
| 22–23 | #862 |
| 24 | #863 |
| 27 | #864 |
| 28–29 | #865 |
| 30 | #866 |
| 32–33, 41 | #868 |
| 34–36 | #869 |
| 37 | #860 |
| 42–43 | #876 |
| 44–45 | #877 |

## Parent close gate

- [ ] #880 done
- [ ] RFC 1–45 coverage complete
- [ ] no child closed with unexplained TODO/skip
- [ ] close note links child commits, ADRs, parity receipts, migration evidence, benchmark receipts

## Completed child receipts

- [x] #853: `ad0c4f21d`, `7fda6273c`; ADR-059; GC/fallback positive parity; `verify fixtures` PASS=1692 FAIL=0. See [#853](853-ownership-move-copy-mutability.md).
- [x] #854: `ca6b85ad8`; ADR-060; owned ADT / Box / Slice implementation; GC component and linear-memory parity output `42, 1, 1, 8`; `verify lane` and docs/ADR gates PASS. See [#854](../done/854-adt-box-slice-value-semantics.md).
- [x] #855: `093e2e914`; ADR-061; shared/mutable borrow, move capture, E0213/E0214 escape/conflict diagnostics, and GC/linear positive parity (`borrow:3`, `move:4`). The targeted current-artifact quality lane passed; pinned bootstrap refresh remains an existing #854/#878 boundary. See [#855](../done/855-closure-capture-lifetime.md).
- [x] #856: `782c34b2980368400a5a076ea72f20b369234bfd`; ADR-062; structural trait identity with ordered type arguments, `Self`, associated type/function projection, coherence and blanket termination rules, static-dispatch/`dyn Trait` boundary, and unrestricted semantic generic arity. Positive GC/linear fixtures produced `79` and `7`; diagnostic fixtures cover E0215–E0218. The current artifact registry/core-op audits reported zero mismatches; the canonical compiler rebuild remains blocked by the pre-existing #854 generic-enum bootstrap mismatch, and the fixture gate retains one unrelated pre-existing `match_extensions/struct_pattern.ark` failure. See [#856](../done/856-trait-core-coherence-generics.md).
- [x] #857: `d6ce3f5d4ee1e8ca82fdbff7292579a30e7edaf3`; ADR-063; `?` conversion and Error sentinel boundary, with positive and diagnostic fixtures and GC/linear targeted receipts. See [#857](../done/857-question-mark-conversion-error-sentinel.md).
- [x] #858: `31c76ce33`; ADR-065; canonical operator Trait mapping, coherence-aware typechecking, generic/builtin/user-defined fixtures, and GC/linear parity receipts. The canonical compiler rebuild remains blocked by the pre-existing #854 generic-enum mismatch; targeted quality/docs/ADR gates pass. See [#858](../done/858-operator-traits-magic-method-retirement.md).
- [x] #859: `fd50e0f45` plus follow-ups `2bfeeede3` and `10fdbe597`; ADR-066; canonical generic `HashMap<K,V>` constructor/method API, static typed adapters for six supported pairs, internal-only `Any` boundary, and explicit compatibility removal conditions. The positive HashMap fixture compiles and validates on both `wasm32-gc` and `wasm32` fallback with the current s2 artifact; the fallback artifact runs successfully, and the default GC P2 component packages, validates, and runs successfully. The unsupported `bool,bool` pair produces the expected diagnostic, while `build-compiler` and the docs example gate pass through the guarded source-only bootstrap overlay. See [#859](../done/859-generic-public-api-any-hashmap.md).
- [x] #862: `a720c2cc7`; ADR-068; integer/float edge semantics, source evaluation order, place-before-RHS lowering, side-effect and diagnostic fixtures, and GC/linear compile/validate/runtime parity. The canonical compiler rebuild remains blocked by the pre-existing #854 `unbox`/`slice_get` mismatch; the targeted shim receipt is explicitly not a canonical rebuild. See [#862](../done/862-arithmetic-edge-and-evaluation-order.md).
- [x] #863: `e4becb596`; ADR-069; valid UTF-8 and Unicode scalar contract, byte/scalar API split, strict/lossy UTF-8 conversion, boundary-aware slicing, `char_at` compatibility migration, and GC/linear parity fixtures. The canonical compiler rebuild remains blocked by the pre-existing #854 `unbox`/`slice_get` mismatch; the targeted current-source s2 receipt is explicit and does not claim a canonical rebuild. See [#863](../done/863-string-unicode-semantics.md).
- [x] #864: `f9393be42`; ADR-070; immutable non-expansive let generalization, environment-aware TypeScheme subtraction, mutable/expansive negative fixtures, and GC/linear targeted runtime parity. `selfhost diag-parity` and `verify lane` pass; full current fixture parity remains an existing 176-failure backlog, and the canonical compiler rebuild remains blocked by #854. See [#864](864-let-generalization-value-restriction.md).
- [x] #870: `b6b7b2e0f` plus `10fdbe597`; ADR-074; independent GC, memory-width, host-profile, and artifact-kind capability axes, target-specific diagnostics, and GC/linear/component validation and runtime receipts. See [#870](../done/870-target-capability-configuration.md).
