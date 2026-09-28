---
Status: open
Created: 2026-09-28
Updated: 2026-09-28
ID: 899
Parent: 875
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #875, #892, #898, #852"
Orchestration class: design-ready
Orchestration upstream: 875
Blocks v{N}: language-v2
Priority: 2
Source: "#898 verification residual — unmatched non-catchall last arm diverges GC vs linear"
---

# 899 — 非 catchall last arm が非マッチ時に GC/linear で発散

## Scope

i32 など builtin 以外の scrutinee では typechecker の exhaustive
チェックが無い(`check_builtin_match_exhaustive` は
bool/Option/Result のみ、enum は別経路)。そのため
`match x { 1 => "one", 2 => "two" }` のような
catchall を持たない match が受理されるが、**全 arm が非マッチ**
のときの観測結果が backend で発散する。

## Evidence

最小プローブ(`x = 9`):

```ark
let r = match x {
    1 => "one",
    2 => "two",
}
stdio::println(r)
```

- wasm32 linear core: `two` を出力(last arm が無条件で実行される)。
- wasm32-gc component: `wasm trap: null reference`
  (条件付き arm が skip され result が null ref のまま println へ)。
- or-pattern last arm(`2 | 3 => "two-three"`)も同一の発散を示す。
  `check-cross-backend-parity.py` の 223 fixture ではこの形を
  カバーする fixture が無く diverge=0 として観測されない。

## Design notes

- 言語意味論の確定が先: 非 exhaustive match は compile 時診断か、
  runtime trap か、defined default か。match lowering の所産ではなく
  意味論決定の問題なので ADR 判断に従う。
- last arm の lowering 形状が backend 間で異なる可能性
  (CoreHIR 経路の last arm は non-catchall でも condition を
  emit しない構造)を調査対象に含める。
- exhaustive 診断を i32 等の literal scrutinee へ拡張する場合は
  `check_builtin_match_exhaustive` と coverage 記録の拡張が要る。

## Acceptance

- [ ] 非 catchall match の非マッチ時の意味論が確定し、
      wasm32-gc / wasm32 linear で同一の観測結果になる。
- [ ] 採用した仕様(診断拒否 or runtime trap or default)に対する
      positive/negative fixture が両 backend で等しい出力を持つ。

## Required evidence

- implementation commit SHA または採択 ADR
- 両 backend の非マッチ時挙動の実行結果
- parity receipt での当該ケース非発散
