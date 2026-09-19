---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
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
- [ ] #864 — let-generalization / value restriction を形式化する — RFC 27 — depends: 856

### Numeric / text / syntax

- [x] #861 — 数値の暗黙変換を廃止し明示変換を定義する — RFC 20, 21 — depends: none
  - Receipt: completed in `549fc4e1a` plus fixture fix `72c188934`; ADR-067,
    RFC-011, and the language spec are synchronized. Positive and diagnostic
    fixtures prove explicit conversion and rejection of mixed concrete numeric
    arithmetic; GC and linear compile/validate/runtime receipts both pass.
- [ ] #862 — 算術 edge case と評価順序を仕様化する — RFC 22, 23 — depends: none
- [ ] #863 — String / Unicode semantics を確定する — RFC 24 — depends: 854
- [ ] #865 — Hygienic desugaring と f-string lowering を実装する — RFC 28, 29 — depends: 729
- [ ] #866 — source module の import / use 構文を整理する — RFC 30 — depends: none
- [ ] #867 — `pub` / Component export / WIT import 境界を分離する — RFC 11, 31, 38 — depends: 866
- [ ] #868 — Prelude / boolean literal / keyword policy を固定する — RFC 32, 33, 41 — depends: none
- [ ] #869 — multi-clause / where / semicolon grammar を確定する — RFC 34–36 — depends: none

### Targets / backends

- [ ] #870 — target capability / Memory64 / host profile を分解する — RFC 9, 39, 40 — depends: none
- [ ] #871 — Wasm GC aggregate lowering を完成する — RFC 8 (aggregate) — depends: 854, 870
- [ ] #872 — Wasm GC closure / collection lowering を完成する — RFC 8 (closure/collection) — depends: 855, 859, 860, 871
- [ ] #873 — linear-memory tracing GC fallback runtime を実装する — RFC 8 (fallback runtime) — depends: 870
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
- [x] #859: `fd50e0f45`; ADR-066; canonical generic `HashMap<K,V>` constructor/method API, static typed adapters for six supported pairs, internal-only `Any` boundary, and explicit compatibility removal conditions. The positive HashMap fixture compiled/validated/ran on both `wasm32-gc` and `wasm32` fallback; the unsupported `bool,bool` pair produced the expected diagnostic. The canonical compiler rebuild and doc-example subgate remain blocked by the pre-existing #854 `unbox`/`slice_get` mismatch; the targeted bootstrap shim is not claimed as a canonical rebuild. See [#859](../done/859-generic-public-api-any-hashmap.md).
