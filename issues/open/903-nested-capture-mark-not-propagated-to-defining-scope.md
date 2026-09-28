---
Status: open
Created: 2026-09-28
Updated: 2026-09-28
ID: 903
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-061, ADR-083, RFC-011, #900, #852"
Orchestration class: design-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 2
Source: "#900 close residual — nested closure borrow/move marks land on scope-child copies and never reach the defining scope"
---

# 903 — nested closure の borrow/move マークが defining scope に伝播しない

## Scope

ADR-083 の capture-through 採用により、nested closure は中間 frame を
透過して source を capture できる。codegen と観測可能意味論は
両 backend で一致するが、typechecker の borrow / move / moved マークは
closure literal が置かれた scope の copy にだけ落ち、source の
defining scope には伝播しない。

原因: `scope_child` が VarBinding を値コピーする構造で、
`apply_closure_capture_summary` / `consume_expr` のマークは
検査時点の scope のコピーに書き込まれ、outer body scope が畳まれると
失われる。

影響（診断精度のみ。codegen / 観測可能意味論は一致）:

- `move` inner が source を消費しても、enclosing function 内の
  後続使用が move-after-use (E0212) で拒否されない。
- inner の mutable borrow が enclosing scope の後続 use/代入との
  conflict (E0214) を起こさない。同じ中間 frame 内の兄弟 closure
  同士の conflict は scope コピー共有により検出される。

## Evidence

```ark
fn main() {
    let source = String_from("s")
    let outer = || {
        let inner = move || {
            text::concat(source, "!")
        }
        inner()
    }
    outer()
    let reused = source   // 一段目なら E0212。nested では未検出。
}
```

一段目の `move` closure では同形が E0212 で拒否される
（`diagnostics/closure_move_after_capture.ark`）。

## Acceptance

- [ ] nested closure の borrow / move マークが source の defining
      scope の binding に適用され、一段目 closure と同一の診断が
      発火する。
- [ ] move-after-use / mut-borrow-conflict の nested 形に対する
      診断 fixture が追加される。
- [ ] ADR-083 §3 の「現行の既知ギャップ」記述が撤去される。

## Required evidence

- defining scope へのマーク伝播を行う implementation commit SHA
- nested 形 diag fixture（上記 move / conflict 系）が発火する出力

## False-done guard

- [ ] scope-child のコピー共有など便宜的な手当てで、兄貴 scope への
      誤ったマーク流出を起こさない（副作用方向の診断で検証）
