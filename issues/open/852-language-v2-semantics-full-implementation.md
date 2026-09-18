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

- [ ] #853 — Ownership / move / Copy / mutability を確定・実装する — RFC 1–4 — depends: none
- [ ] #854 — ADT / Box / slice の値意味論を統一する — RFC 5–7, 25 — depends: 853
- [ ] #855 — Closure capture / escape / lifetime を実装する — RFC 10 — depends: 853

### Trait / API / type system

- [ ] #856 — Trait core / coherence / generic arity を確定する — RFC 12, 17 — depends: none
- [ ] #857 — `?` / conversion / Error sentinel を整理する — RFC 13, 19 — depends: 694, 856
- [ ] #858 — 演算子 Trait へ統一し magic method を退役する — RFC 14 — depends: 856
- [ ] #859 — 公開 API を generic Trait 面へ統一する — RFC 15, 16, 18, 26 — depends: 856
- [ ] #860 — Iterator Trait 体系を統合完了する — RFC 37 — depends: 691, 856
- [ ] #864 — let-generalization / value restriction を形式化する — RFC 27 — depends: 856

### Numeric / text / syntax

- [ ] #861 — 数値の暗黙変換を廃止し明示変換を定義する — RFC 20, 21 — depends: none
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
