# ADR-004 P4: メソッド構文の評価

ステータス: **DEFERRED** — 評価保留（trigger待ち）
日付: 2026-04-15
関連: ADR-004 (trait strategy), Issue #157

---

## 背景

ADR-004 は v0 から trait を延期し、段階的導入計画を定めた:
P1（限定 `for`）、P2（文字列補間）、P3（trait）、**P4（メソッド構文）**、P5（演算子オーバーロード）。

本ドキュメントは、メソッド構文（例: `v.push(x)`）を導入するかどうかを評価し、決定を記録する。

---

## 1. 現状: 関数中心の設計

Arukellt の stdlib は、第1引数に「主語」を取る自由関数を用いる:

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let v: Vec<i32> = Vec_new_i32()
push(v, 42)
let n: i32 = len(v)
let s: String = concat(a, b)
let lower: String = to_lower(s)
let parts: Vec<String> = split(s, delim)
```

高階演算も同じパターンに従う:

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let doubled: Vec<i32> = map_i32_i32(v, |x| x * 2)
let evens: Vec<i32> = filter_i32(v, |x| x % 2 == 0)
let total: i32 = fold_i32_i32(v, 0, |acc, x| acc + x)
```

現時点のユーザーコードには `impl` ブロックはない。仕様は `trait` と `impl` を v1 キーワードとして予約し、メソッド呼び出し構文（`expr.method(args)`）を定義しているが、コンパイラの stdlib 表面はすべて関数ベースである。

---

## 2. 現行アプローチの利点

| 利点 | 詳細 |
|---------|--------|
| **パーサが単純** | `.ident(args)` 式が不要。呼び出し式は一律 `ident(args)`。 |
| **vtable オーバーヘッドなし** | ディスパッチはすべて静的。trait オブジェクトによる間接参照がない。 |
| **所有権が明確** | 第1引数が明示的に渡される。隠れた `self` セマンティクスがない。 |
| **LLM に優しい** | 解決規則が単純 — スコープ内の名前解決のみ。impl 探索が不要。 |
| **フラットな名前空間** | フィールドアクセスとメソッド呼び出しの曖昧さがない。 |
| **エラーメッセージが分かりやすい** | 「function `push` not found」対「no method `push` for type `T`」。 |

---

## 3. 現行アプローチの欠点

| 欠点 | 詳細 |
|----------|--------|
| **チェーンが冗長** | `s.split(",").map(trim).join(";")` の代わりに `join(map_String_String(split(s, ","), trim), ";")`。 |
| **多くのプログラマに馴染みが薄い** | Rust/Python/JS/Go 出身者は `v.push(x)` を期待する。 |
| **型サフィックス付き名前** | メソッド解決がなければ、HOF は単相化名（`map_i32_i32`、`filter_String`）が必要。 |
| **発見しやすさ** | IDE は値に対する `.` 補完で適用可能な操作を列挙できない。 |

---

## 4. 最小メソッド構文案 — UFCS

**Uniform Function Call Syntax**（UFCS）: `v.push(x)` は `push(v, x)` に脱糖される。

### 4.1 セマンティクス

```
expr.name(args…)  ≡  name(expr, args…)
```

- レシーバ `expr` が第1の位置引数になる。
- 解決: `name` を自由関数として検索し、第1パラメータの型が `expr` の型と一致することを確認する。`impl` ブロックは不要。
- フィールドと関数の両方が一致する場合、フィールドアクセスが優先される（struct セマンティクスと整合）。

### 4.2 チェーンの例

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
// Before (function-centric)
let result = join(map_String_String(split(s, ","), trim), ";")

// After (UFCS sugar)
let result = s.split(",").map_String_String(trim).join(";")
```

### 4.3 trait（P3）との相互作用

後から trait が導入された場合、UFCS と trait メソッドは共存する:

1. trait メソッド（`impl` 経由）を先に解決する。
2. trait メソッドが一致しなければ、UFCS の自由関数検索にフォールバックする。

この順序により、trait 追加時に既存コードを壊さない。

---

## 5. 影響分析

| コンパイラフェーズ | 必要な変更 | 複雑さ |
|---------------|----------------|------------|
| **Parser** | `.ident(args)` を後置式として追加。`MethodCall(receiver, name, args)` としてパース。 | 低 — 新しい式バリアントが1つ。 |
| **Resolver** | `MethodCall` → `Call(name, [receiver, …args])` に脱糖。スコープで `name` を検索。第1引数の型がレシーバと一致することを検証。 | 中 — 新しい検索経路。フィールドとの曖昧さの可能性。 |
| **Type checker** | レシーバ型を推論して関数検索を駆動。型サフィックスによるオーバーロード（`Vec<i32>` 用 `push` vs `Vec<String>` 用）の場合、resolver が正しい単相化バリアントを選択する必要がある。 | 中 — 型指向の名前解決が必要。現行チェッカーは未対応。 |
| **HIR / CoreHIR** | 構造的変更なし — lowering 前に脱糖される。 | なし。 |
| **Emitter (Wasm)** | 変更なし — `Call` ノードのみを見る。 | なし。 |
| **Stdlib surface** | 変更不要。既存の `push`、`len`、`concat` などはそのまま動作。型指向検索があれば、HOF 名から型サフィックスを外せる可能性がある。 | なし（即時）/ 中（クリーンアップ）。 |
| **Docs / Migration** | UFCS 規則を文書化。例を更新。 | 低。 |

---

## 6. 推奨

**v5 以降に延期する。**

根拠:

1. **セルフホスティング優先。** v5 マイルストーンはコンパイラの Arukellt によるセルフホスティングを目標とする。セルフホスティングが安定する前にメソッド構文を導入すると、parser/resolver の複雑さが増し、その目標を遅らせる。

2. **現行 API は一貫している。** 関数中心の stdlib は機能する。すべての操作は呼び出し可能で合成可能。冗長さのコストは実在するが、セルフホスティングコンパイラには管理可能。

3. **後方互換の糖衣構文。** UFCS は純粋な構文糖。既存コードを壊さず、セマンティクスを変えずにいつでも追加できる。

4. **型指向解決が前提条件。** 現行 resolver は名前のみの検索。単相化関数名（`push` が正しい型バリアントに解決）を伴う UFCS には型指向解決が必要で、これはセルフホスティングで型システムが実戦投入された後に取り組むのが望ましい非自明な変更である。

5. **trait（P3）を先に着地させるべき。** メソッド構文は trait ベースのディスパッチと組み合わせたとき最も価値がある。trait より前に UFCS を導入すると、2つの別々のメソッド解決システムになる。まとめて導入する方がきれい。

---

## 7. 正式な評価決定

**決定: DEFERRED — 評価は trigger 待ちで延期**

ADR-004 P4 の評価は、下記の trigger 条件が満たされるまで開始できない。それまでは実装や設計のコミットを行わない。本節は trigger、スコープ、決定木を形式化し、trigger が発火したときに曖昧さなく評価を進められるようにする。

---

### 7.1 Trigger 条件（開始条件）

評価は次の**すべて**が真のときに開始する:

| # | 条件 | 測定可能な基準 |
|---|-----------|----------------------|
| T1 | すべての MIR 最適化パスが安定 | `Track: mir-opt`（または同等）の issue がすべて `issues/done/` にある。`--opt-level 1` での full verify-harness 合格率がベースライン以上。 |
| T2 | Core v4 パススイートが連続 2 回の CI で回帰なし | MIR opt 有効で `python scripts/manager.py verify` が連続 2 回 exit 0。 |
| T3 | Stdlib API 表面が安定 | 評価スコープ内のメソッドの名前/シグネチャ変更を計画する `Track: stdlib` の open issue がない。 |

---

### 7.2 評価スコープ

評価は明示的に**最小メソッド集合**に限定される。P4 でメソッド呼び出し構文の採用を評価するのは次の操作のみである:

| メソッド | 自由関数相当 | 優先度 |
|--------|--------------------------|----------|
| `.push(x)` | `push(v, x)` — `Vec<T>` | High |
| `.pop()` | `pop(v)` — `Vec<T>` | High |
| `.len()` | `len(v)` — `Vec<T>`, `String` | High |
| `.map(f)` | `map_T_U(v, f)` — `Vec<T>` | High |
| `.filter(f)` | `filter_T(v, f)` — `Vec<T>` | High |
| `.to_lower()` | `to_lower(s)` — `String` | Medium |
| `.split(d)` | `split(s, d)` — `String` | Medium |
| `.join(d)` | `join(parts, d)` — `Vec<String>` | Medium |

完全な trait システム（`impl` ブロック、trait オブジェクト、演算子オーバーロード）は P4 の**スコープ外**。P3（trait）は別途評価する。

---

### 7.3 入退場の決定木

```
Trigger fires (T1+T2+T3 met)
│
├─ Evaluate: can type-directed name resolution be added in a
│   bounded-scope PR without breaking existing tests?
│   │
│   ├─ YES ──→ Prototype UFCS desugaring (§4) for scope in §7.2
│   │           │
│   │           ├─ All fixture tests still pass? ──→ ADOPT-UFCS
│   │           └─ Regressions found   ──────────→ DEFER-AGAIN /
│   │                                               REJECT and document
│   │
│   └─ NO  ──→ DEFER: complexity cost exceeds benefit for P4 scope.
│               Record blocking issues and next-review milestone.
│
└─ Evaluate: is function-centric API sufficient for the remaining
    v5 self-hosting goals without method-call sugar?
    │
    ├─ YES ──→ REJECT: close P4; record that UFCS is a post-v5 option.
    └─ NO  ──→ Must resolve via one of the above paths.
```

この決定木の結果は次のいずれかでなければならない:

- **`ADOPT-UFCS`**: §4 で述べた最小脱糖を実装する。ADR-004 の修正として記録する。`src/compiler/parser.ark`、`src/compiler/resolver.ark`、`src/compiler/typechecker.ark` に対する実装 issue を起票する。
- **`ADOPT-FULL`**: `impl` ブロックが必要（P3 の着地に依存）。P3 が着地済み、または確定スケジュールがある場合のみ有効。
- **`DEFER-AGAIN`**: 複雑さのコストが高すぎる。新しい明示的 trigger を設定し、本ドキュメントを更新する。
- **`REJECT`**: 関数中心 API が長期的に十分である理由を文書化する。本 ADR を最終的な REJECTED 決定としてクローズする。

---

### 7.4 本 ADR を再オープンする条件

次の場合のみ再オープン（ステータスを `DEFERRED` から `IN REVIEW` に変更）する:
- §7.1 のすべての trigger 条件が満たされ、**かつ**
- 1 スプリント以内に決定を主導するレビュアがアサインされている。

---

### 7.5 参照

- `docs/process/roadmap-v4.md` §6 item 9, §12 item 1 — 本評価のマンデート
- `docs/language/spec.md` §2.8, §3.6 — trait/メソッド構文仕様
- `std/prelude.ark` — 現行の関数中心 stdlib
- `issues/done/157-adr004-method-syntax-evaluation.md` — 追跡 issue（クローズ済み）
