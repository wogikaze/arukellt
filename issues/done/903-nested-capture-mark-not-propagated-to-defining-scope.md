---
Status: done
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

- [x] nested closure の borrow / move マークが source の defining
      scope の binding に適用され、一段目 closure と同一の診断が
      発火する。
- [x] move-after-use / mut-borrow-conflict の nested 形に対する
      診断 fixture が追加される。
- [x] ADR-083 §3 の「現行の既知ギャップ」記述が撤去される。

## Required evidence

- defining scope へのマーク伝播を行う implementation commit SHA
- nested 形 diag fixture（上記 move / conflict 系）が発火する出力

## False-done guard

- [x] scope-child のコピー共有など便宜的な手当てで、兄貴 scope への
      誤ったマーク流出を起こさない（副作用方向の診断で検証）

## Done (2026-09-28)

- root cause: scope_child が VarBinding を値コピーするため、
  nested closure literal 位置で apply された borrow / move マークは
  中間 scope のコピーにだけ書き込まれ、outer body scope が畳まれる
  と失われていた。
- implementation: d29094cdb fix(#903)。
  scope::scope_merge_child_marks (src/compiler/typechecker/scope.ark)
  が、scope_child の binding 順序保持を利用して copied prefix を
  index 対応で親 scope へ set-only マージする。body 内宣言の
  shadow binding は prefix 外に積まれるため、name-based な伝播で
  あり得た「shadow 名が outer binding を汚染する」誤流出が構造上
  起きない。伝播は設定方向のみで、nested body 内の rebind が親の
  moved を解除することはない（rebind の効果は literal 位置ではなく
  呼び出し時に現れるため、一段目の capture apply と整合）。
- 適用箇所は scope_child を畳む全地点: infer_closure_expr の
  closure body、infer_match_expr の各 arm、infer_comprehension_expr
  の elem/filter。match arm と comprehension は同じ scope-child
  copy 消失の構造を持ち、closure 内に match/comprehension 経由で
  深く nest した capture も段階的に defining scope まで届く。
- 一段目との同一診断（実測）: nested move →
  `error[E0212|typecheck]: use of moved value `source`; v2 uses
  move semantics; ...`（closure_move_after_capture と同一コード・
  同一文）、nested mut-borrow + enclosing assign →
  `error[E0214|typecheck]: cannot assign to borrowed capture
  `count``（closure_borrow_conflict 経路と同一代碼）。一段目
  probe (first_level_probe.ark) では mut-borrow された count の
  `count.to_string()` が `error[E0213|typecheck]: borrowed
  closure capture cannot escape` を同位置で発火し、nested 版も
  nested_capture.ark:22 で同一挙動になったことを確認した。
- fixture 更新: nested_capture.ark の `println(count.to_string())`
  を repo 慣用の `println((count + 0).to_string())` へ。bare
  ident は escape witness が拾うが BINARY は拾わない既存仕様
  (borrow_rebind_cell.ark と同形)。観測値 5/6/7 の write-through
  検証は維持。
- 追加 fixture: diagnostics/closure_nested_move_after_use.ark
  (E0212)、diagnostics/closure_nested_borrow_assign_conflict.ark
  (E0214) は manifest の diag: へ。closure_capture/
  nested_shadow_move_scope.ark は run:/t3-compile:/t3-run: で、
  `let source` shadow が inner move に capture されても outer
  binding が moved と誤マークされず `outer:` を print することを
  固定（false-done guard の副作用検証）。
- ADR-083 §3 は「現行の既知ギャップ」から解消済み仕様記述へ
  書き換え、達成済みの再検討条件（scope 構造導入時に §3 を
  撤去する）も削除。observable-semantics rule
  closure_nested_capture_through に mark-folding の記述と +3
  fixture/diagnostic を追記 (09392613c docs(#903))。
- verification: selfhost build-compiler 53.5s で新 s2 完走
  （新ロジックで compiler 自身 ~2000 .ark を compile し over-fire
  なし）。fixture-parity 対象 44/44 PASS
  （closure_capture/diagnostics/match_extensions/ownership/
  control/comprehension）。拡大回帰 313/322、fail は全て
  stdlib_vec/*_i64 の E0200 で post891 s2 と同一の pre-existing。
  diag-parity 71 pass / FAIL=1 は capability_stub.ark の golden
  ドリフトで post891 とも同一 NOMATCH の pre-existing。
  verify lane PASS、fmt PASS、docs regenerate + docs check
  6/6 PASS。
- 残余: arm/comprehension の畳み込みは possibly-moved の
  保守的診断を新たに可能にするが、name-based ではなく index
  merge のため shadow 経由の誤検出は構造上排除。escape
  summary（literal AST scan）は今回の mark 系とは別機構で、
  nested closure が返す inner の borrow capture を outer
  literal の escape summary が見えない点は ADR-081/ADR-083
  系の別論点として残る。
