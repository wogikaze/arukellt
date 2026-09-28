# ADR-083: 言語 v2 の nested closure capture-through — 輸送は透過、所有権は各フレーム

ステータス: **ACCEPTED** — nested closure は lexical scope 全域から capture でき、中間フレームは所有権を伴わない transport slot として値を中継する

決定日: 2026-09-28

関連: [ADR-061](ADR-061-language-v2-closure-capture-lifetime.md)、[ADR-081](ADR-081-language-v2-borrow-capture-escape-cells.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#900](../../issues/open/900-multiframe-capture-source-invalid-wasm.md)、[#891](../../issues/done/891-borrow-capture-escape-env-cells.md)、[#872](../../issues/open/872-language-v2-closure-pair-env.md)

## 文脈

nested closure が capture する変数は、直近の enclosing closure が
同じ変数を capture している場合にだけ backend が解決できた。
中間 closure がその変数を捕捉していないとき、capture source の
解決が壊れたまま lowering され、compiler が invalid core Wasm を
生成していた(#900)。

ADR-061 §1 は「nested closure は内側の closure 自身が capture を
検査し、外側の closure の capture 集合へ暗黙に昇格しない」と定める。
この文は所有権解析の規則であり、値の輸送経路については何も決めて
いなかった。取りうる意味論は 2 つあった。

- **capture-through**: inner は lexical scope 全域から capture でき、
  中間フレームが値を機械的に中継する(Rust / JS と同じ)。
- **diagnostic 拒否**: 直近フレームに capture source が無い参照を
  compile 時診断で拒否する。

## 決定

### 1. capture-through を採用する

- nested closure は通常の lexical scoping と同じく、任意の深さの
  enclosing scope の binding を capture できる。
- 中間フレームは、自分の body が直接参照しない名前を transport
  slot として env に乗せる。transport は backend の受け渡し表現
  であり、ADR-061 の capture 集合(所有権)への昇格ではない。
  同 ADR の「暗黙に昇格しない」はこの所有権規則として維持する:
  中間 frame は transport 名に対して borrow / move を取得しない。
- capture の mode(borrow / mutable borrow / move)と borrow conflict
  診断は、capture を行う closure 自身の検査だけが決める。
  inner が source を mut-borrow する場合、排他性・escape 規則は
  inner に適用され、中間 frame には適用されない。

### 2. transport slot の表現

- read のみを運ぶ transport slot は、その名前の cap と同じ規則で
  値または ref を運ぶ(snapshot / alias は直接 capture と同型)。
- descendant が source を rebind する transport slot は cell ref を
  運ばなければならない。source place は ADR-081 の promoted
  shared cell に昇格され、inner の write は全段を通じて source に
  届く(write-through)。
- 中間 frame が move prefix を持つ場合でも、transport 名は move
  capture ではない。source は引き続き promoted cell に昇格し、
  move env は cell ref を受け取る。中間 frame の move はその frame
  自身の direct capture にだけ効く。

### 3. 診断の適用境界

- nested closure が transport 越しに source を borrow / move
  する場合、そのマークは capture を行う closure 自身の検査が
  決め、source の defining scope の binding へ適用される。
  scope_child は binding 順序を保持するため、nested body の
  検査中に立てられた所有権マークは copied prefix の index
  対応で親 scope へ単調に畳み込まれる(設定のみ、解除は
  伝播しない)。body 内で宣言された binding は prefix の外に
  積まれるため、shadow 名が outer binding のマークを汚染
  することはない。
- この畳み込みにより、nested move capture は enclosing
  function 内の後続 use に対して一段目と同じ move-after-use
  (E0212)を、nested mutable-borrow capture は後続の代入に
  対して一段目と同じ borrow conflict (E0214)を発火する。
  一段目 closure の capture apply と同様、マークは literal の
  位置で保守的に記録され、closure が実際に呼ばれるかは問わ
  ない。

## 帰結

- #900 の最小プローブ(read / rebind、escape / non-escape、3 段 nest、
  move 外側 / move 内側)は wasm32-gc・wasm32 の両 backend で同一の
  観測結果を持つ。fixture: closure_capture/nested_capture.ark、
  closure_capture/nested_capture_escape.ark、診断:
  diagnostics/closure_nested_borrow_conflict.ark。
- lambda lifting の free-variable 収集と rebind flag 伝播は、
  nested closure の body を透過して走査する。shadowing する
  param は transport 名を隠す。

## 却下した代替案

- **直近フレームに source が無い参照を compile 診断で拒否する。**
  lexical scope と矛盾し、中間 frame がたまたま同名を capture する
  かで可・不可が変わる不安定な規則になる。transport は既存の env
  機構で表現できるため採用しない。
- **transport 名を中間 frame の所有権 capture として扱う。**
  ADR-061 の「暗黙に昇格しない」に反し、move 中間 frame が
  transport だけを理由に source を消費する誤った所有権を生む。
  また inner と中間 frame の二重 mut-borrow が正当な program を
  conflict 診断で潰すため採用しない。
- **inner の capture を一段目へ flatten し、中間 frame を
  完全に素通りさせる。** direct call ABI と env record は 1 段の
  supply を前提としており、emit 側の全経路を組み替える必要が
  あるため採用しない。

## 再検討条件

- escape した中間 frame が shared-borrow の transport を運ぶ
  経路で、borrow の有効範囲を超える read が観測可能になった場合、
  transport の寿命契約を再検討する。
- move capture を runtime で二度構成できる nested 形
  (non-once な中間 frame 内の move inner)を所有権として
  拒否したくなった場合、FnOnce 相当の呼び出し制約と合わせて
  再検討する。

## 参照

- [RFC-011 §4.1](../rfcs/011-language-v2-semantics-redesign.md)
- [docs/language/spec.md](../language/spec.md)
