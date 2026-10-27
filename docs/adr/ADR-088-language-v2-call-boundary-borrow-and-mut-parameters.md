# ADR-088: 言語 v2 の関数呼び出し境界 — 暗黙借用と `mut` 仮引数

ステータス: **ACCEPTED** — 非 `Copy` 引数は呼び出し中の暗黙借用とし、書き換えは `mut` 仮引数と可変 place の組でだけ許可する

決定日: 2026-10-27

関連: [ADR-002](ADR-002-memory-model.md)、[ADR-059](ADR-059-language-v2-ownership-copy-mutability.md)、[ADR-060](ADR-060-language-v2-owned-adts-box-slice.md)、[ADR-061](ADR-061-language-v2-closure-capture-lifetime.md)、[ADR-081](ADR-081-language-v2-borrow-capture-escape-cells.md)、[ADR-014](ADR-014-stability-labels.md)、[#943](../../issues/open/943-ownership-call-boundary-semantics.md)、[#944](../../issues/open/944-move-after-use-not-diagnosed.md)

## 文脈

ADR-059 は、`let` binding を既定で不変とし、mutable place を通した mutation だけを許可し、
move 済み binding の再利用を E0212 とした。ただし §7 で関数引数の境界を後続判断へ送った。

その結果、現行の実装では非 `Copy` の集約値が関数呼び出しで暗黙の共有可変参照として渡されている。
不変の `let p` を `bump(p)` に渡すと callee が `p.x` を書き換え、同じ値を何度渡しても move にならない（#943）。
stdlib とセルフホストコンパイラは、`push(v, x)` で非 `mut` binding の中身を書き換える書き方に依存している。

ADR-002 は GC を採用し、Rust 型の borrow checker を言語に持ち込まない方針をとる。
一方で、不変 binding が任意の呼び出しで書き換わる状態では ADR-059 の不変性が意味を持たない。
呼び出し境界に、書き換えの意図を表す最小限の規則が必要である。

## 決定

### 1. 非 `Copy` 引数は呼び出し中の暗黙借用である

- 非 `Copy` の値を実引数に渡しても、元の binding は move されない。借用は呼び出しの間だけ有効である。
- move 済みの binding を実引数に渡すことは、ほかの利用と同じく E0212 で拒否する（ADR-059 §4）。
- `Copy` の値は従来どおり複製して渡す。

### 2. 仮引数には共有と `mut` の二つの mode がある

- `p: T` は共有借用である。callee は `p` から到達する place への代入、`p` を `mut` 仮引数へ渡すこと、
  `mut self` を要求するメソッドの呼び出しを行ってはならない。
- `mut p: T` は可変借用である。callee は `p` から到達する place を書き換えてよく、その変更は caller から観測できる。
- `mut p` の仮引数自体への再代入（`p = ...`）は callee 内の名前の付け替えであり、caller の binding を変更しない。
- `Copy` 型の `mut p: T` は複製された値を局所的に書き換えるだけで、caller には影響しない。

### 3. receiver も同じ規則に従う

- `self: T` は共有借用であり、`mut self: T` は可変借用である。
- `x.m()` が `mut self` を要求する場合、`x` は §4 の可変 place でなければならない。

### 4. `mut` 仮引数へ渡せるのは可変 place だけである

`mut` 仮引数へ渡せる実引数は次のいずれかである。

- `let mut` で導入した binding
- 外側の関数の `mut` 仮引数、または `mut self`
- 上記を根とする field / element place
- 一時値（呼び出し結果、literal、constructor 式など、名前を持たない新しい値）

不変の binding、共有仮引数、共有 receiver を根とする place を渡すことは compile-time error とする。
診断コードは実装 issue で割り当てる。

### 5. 同じ place の同時借用を禁止する

一つの呼び出しで、同じ根の place を `mut` 仮引数とほかの仮引数の両方へ渡すことは compile-time error とする。
判定は place の構文上の根と経路で行い、alias 解析は要求しない。ADR-061 の E0214 と同じ考え方である。

### 6. 借用した仮引数の escape は共有として許可する

- callee が借用した非 `Copy` 仮引数を返却する、集約値に格納する、`move` closure に捕捉することは許可する。
  このとき値は複製されず、GC reference として共有される。
- 言語が保証するのは「不変の binding と共有仮引数を根とする経路からは書き換えが起きない」ことである。
  escape によって作られた別名から `mut` 経路で書き換えた場合、その変化は元の binding からも観測できる。
  この観測結果は `wasm32-gc` と `wasm32` で同一でなければならない。
- 例外として、一意所有を契約とする `Box<T>`（ADR-060 §2）は共有 escape の対象にしない。
  借用した `Box<T>` 仮引数を返却・格納・`unbox` することは拒否し、所有を移す場合は呼び出し側で値を move した一時値を渡す。

### 7. mode は関数の型の一部である

- 仮引数の mode は関数シグネチャと関数型（`fn(mut Vec<i32>) -> i32`）に含まれる。
- 共有仮引数を持つ関数値は、同じ位置が `mut` の関数型へ代入できる（読み取りだけの関数は可変借用を受けても安全である）。
  逆方向の代入は拒否する。
- closure の仮引数も `|mut v| ...` で同じ規則に従う。capture の規則は ADR-061 / ADR-081 を変更しない。

### 8. stdlib の可変操作は `mut` で宣言する

`push`、`set`、`pop`、`insert`、`clear` などの可変操作は、receiver または対象の仮引数を `mut` で宣言する。
名前による特別扱いでこの規則を免除してはならない（#936）。

### 9. backend から独立した型検査契約とする

mode、可変 place、同時借用の判定は typechecker の契約であり、GC reference や linear pointer の表現から推測してはならない。

## 帰結

- ADR-059 §7 の関数引数境界はこの ADR で確定する。ADR-059 の binding 規則（`let` の move、`Copy`、`let mut`）は維持する。
- 不変の `let v` に対する `push(v, x)` は拒否される。既存コードは `let mut` と `mut` 仮引数を明示する移行が必要である。
- `let b = a` は `a` を move するが、`let b = id(a)` のように共有仮引数の escape を経由すると move せずに別名が生じる。
  これは §6 の選択の帰結であり、「別名がないこと」は保証しない。
- 移行は ADR-014 に従い、警告期間を経てエラー化する。実装状態と移行の進捗は current-state と issue に置く。

## 却下した代替案

- **非 `Copy` 引数を既定で move し、借用を明示構文にする（Rust 型）。**
  ADR-059 / ADR-060 とは最も整合するが、stdlib とコンパイラの全面的な書き換えが必要で、ADR-002 の borrow checker を持たない方針と衝突するため採用しない。
- **純粋な参照意味論にする。**
  `let` の不変を再代入の禁止だけに縮め、E0212 と `Box` の一意所有の主張を取り下げることになり、不変 binding が任意の呼び出しで書き換わる現状を仕様として固定するため採用しない。
- **借用した仮引数の escape に owned mode か `clone()` を要求する。**
  不変性の保証は完全になるが、constructor など仮引数を格納するすべての関数に移行が及ぶため採用しない。
- **中身を書き換えられない型だけを escape 制限から免除する。**
  「不変型」の定義と判定を新たに言語へ持ち込む必要があり、§6 の共有許可で十分なため採用しない。

## 再検討条件

- escape 経由の別名による書き換えが、実際のプログラムで観測可能な不具合の主因になった場合。
- lifetime 解析を導入し、借用の escape を低コストで静的に禁止できるようになった場合。
- Component Model などの外部境界が、引数の所有権移動を公開契約として要求する場合。

## 関連文書

- [ADR-059: 言語 v2 の所有権・Copy・可変性](ADR-059-language-v2-ownership-copy-mutability.md)
- [ADR-060: 言語 v2 の owned ADT・Box・Slice](ADR-060-language-v2-owned-adts-box-slice.md)
- [ADR-061: 言語 v2 のクロージャ捕捉・借用・escape](ADR-061-language-v2-closure-capture-lifetime.md)
- [#943: ownership and mutability rules do not hold at function-call boundaries](../../issues/open/943-ownership-call-boundary-semantics.md)
