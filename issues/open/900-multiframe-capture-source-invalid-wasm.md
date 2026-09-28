---
Status: open
Created: 2026-09-28
Updated: 2026-09-28
ID: 900
Parent: 872
Track: language-v2
Depends on: none
Related: "ADR-058, ADR-061, ADR-081, RFC-011, #872, #891, #852"
Orchestration class: design-ready
Orchestration upstream: 872
Blocks v{N}: language-v2
Priority: 2
Source: "#891 close-review residual — nested closure capture of a variable the intermediate closure does not capture emits invalid Wasm"
---

# 900 — 多段 capture（中間 closure が未捕捉の変数）が invalid Wasm を生成

## Scope

nested closure が capture する変数は、直近の enclosing closure が
同じ変数を capture している場合にだけ解決できる。中間 closure が
その変数を捕捉していないとき、capture source の解決が壊れたまま
lowering され、**compiler が invalid core Wasm を生成する**
（`arukellt-selfhost: error — compiler produced invalid core Wasm`）。

意味論の決定が先である: 中間 closure を透過して capture source を
たどる(capture-through)か、直近フレームに無い source を
compile diagnostic で拒否するか。現状はどちらでもなく
invalid emit であり、少なくとも診断への置換が必須。

## Evidence

最小プローブ（shared borrow、inner は read のみ）:

```ark
fn main() {
    let mut count = 5
    let outer = || {
        let inner = || {
            count + 1
        }
        inner()
    }
    stdio::println((outer()).to_string())
}
```

- wasm32-gc / wasm32 の双方で `compile` は成功と報告されるが
  生成 core Wasm が invalid（validation 失敗）。
- mutable-borrow(rebind)版も同一症状。
- **pre-existing**: `331185572`(#891 適用前)でも同一の invalid
  emit を確認済み。#891 の promoted-cell 導入による回帰ではない。
- 中間 closure が同じ変数を capture する形
  (`outer` が `count` を read/rebind している)は正常に動作し、
  #891 以降は mut-borrow 連鎖も cell 共有で write-through する。

## Acceptance

- [ ] 中間 closure が未捕捉の変数を inner closure が参照する場合、
      capture-through で正しく解決するか、明確な compile diagnostic
      で拒否される（invalid Wasm を生成しない）。
- [ ] 採用した仕様に対する positive/negative fixture が
      wasm32-gc / wasm32 の両 backend で等しい結果を持つ。

## Required evidence

- implementation commit SHA または採択 ADR
- 上記プローブ系（read / rebind、escape / non-escape）の
  両 backend 結果

## False-done guard

- [ ] invalid emit を診断へ変えるだけでなく、採用仕様を
      ADR(current-state/spec)へ反映する
