---
Status: done
Created: 2026-09-28
Updated: 2026-09-28
ID: 900
Parent: 872
Track: language-v2
Depends on: none
Related: "ADR-058, ADR-061, ADR-081, ADR-083, RFC-011, #872, #891, #852"
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

- [x] 中間 closure が未捕捉の変数を inner closure が参照する場合、
      capture-through で正しく解決するか、明確な compile diagnostic
      で拒否される（invalid Wasm を生成しない）。
- [x] 採用した仕様に対する positive/negative fixture が
      wasm32-gc / wasm32 の両 backend で等しい結果を持つ。

## Required evidence

- implementation commit SHA または採択 ADR
- 上記プローブ系（read / rebind、escape / non-escape）の
  両 backend 結果

## False-done guard

- [x] invalid emit を診断へ変えるだけでなく、採用仕様を
      ADR(current-state/spec)へ反映する

## Done (2026-09-28)

- 採択仕様: ADR-083 capture-through(transport は透過、所有権は
  各フレーム)。ADR-061 の「外側の capture 集合へ暗黙に昇格しない」
  は所有権規則として維持し、中間 frame は transport 名に borrow /
  move を取得しない。diagnostic 拒否は却下(lexical scope と矛盾し、
  中間 frame が同名を捕捉するかで可・不可が変わる不安定な規則)。
- root cause: free-variable 収集が NK_CLOSURE で停止していたため、
  nested closure が必要とする名前が中間 frame の env に乗らず、
  inner body の read が空のまま emit され invalid Wasm になった。
  AST 経路(closure_collect_free)と CoreHIR 経路
  (core_closure_collect_free)で同じ穴があり、wasm32-gc は AST
  fallback、wasm32 linear(wasi-p1)は CoreHIR 経路で発症した。
- implementation: 84c5ebcfd fix(#900)。両経路の collect が nested
  closure body を透過降下し(clos=nested で param shadow を維持)、
  rebind flag も透過伝播(nested param を shadowed set で除外)。
  direct/transport を区別して promote 判定を修正: move 中間 frame
  の transport 名も source を promoted cell へ昇格する
  (move は direct capture にだけ効く)。typechecker 側の capture
  summary への混入は ADR-061 違反+二重 mut-borrow conflict を
  起こすため revert(所有権マークは inner 自身の検査が責務)。
- probe matrix(両 backend で同一結果、全て wasm-tools validate OK):
  read mut/imm 非 escape = 6、rebind 非 escape = 6(write-through)、
  3 段 nest = 6、read escape(return)= 6、rebind escape = 6,7
  (cell が escape env と共に write-through 継続)、move 外側+
  rebind 内側 = 6、move 内側 String transport = "s!"。
- positive fixture: closure_capture/nested_capture.ark → 5/6/7、
  nested_capture_escape.ark → 5/10(両 backend 同一、manifest に
  run: 登録。closure_capture は cross-backend parity domain)。
- negative fixture: diagnostics/closure_nested_borrow_conflict.ark
  → error[E0214|typecheck](同一中間 frame 内の兄弟 nested の
  二重 mut-borrow が正しく検出される)。
- verification: fixture-parity --filter-dir closure_capture
  PASS=13 FAIL=0、verify lane PASS(ratchet OK)、docs check 6/6、
  fmt 6 files clean。
- 残余(#903): nested closure の borrow/move マークは scope-child
  copy に落ち defining scope へ伝播しない(例: move inner の
  move-after-use 未検出)。codegen/観測可能意味論は一致、静的診断
  の精度のみの gap — ADR-083 §3 に明記。
