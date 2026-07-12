# ADR-044: trait とメソッド構文を言語機能として採択する

ステータス: **ACCEPTED** — trait / `impl` / メソッド呼び出しを言語の正規機能とする

決定日: 2026-07-11  
廃止: [ADR-004-method-syntax-evaluation.md](ADR-004-method-syntax-evaluation.md)

---

## 文脈

旧 ADR-004 系は trait を延期し、メソッド構文（P4）を trigger 待ちの評価対象としていた。
その後、言語には `trait` / `impl` / メソッド呼び出しが導入され、stdlib は trait-first
方針（issue #709）へ移行中である。延期・評価保留の枠組みは現行決定と矛盾する。

stdlib の具体的な trait 再設計（Iterator、演算子 trait 群など）は
[ADR-036](ADR-036-trait-stdlib-redesign.md) 以降の **PROPOSED** 作業であり、
本 ADR の範囲外である。

---

## 決定

1. **`trait` / `impl` / メソッド呼び出し構文は言語の正規機能である。**
2. 旧「trait 延期」「メソッド構文は P4 評価まで着手しない」方針は**撤回**する。
3. ユーザー向け API は trait-first / メソッド構文を正規とし、公開 free function は
   根絶する（[ADR-046](ADR-046-free-function-eradication.md)、issue #709。
   再設計計画は ADR-036 系）。

---

## 帰結

- ADR-004 は本 ADR により `SUPERSEDED` とする。
- ADR-036 / ADR-038 / ADR-039 は stdlib・演算子・`?` の設計提案として独立に採否する。

## 関連

- [ADR-036](ADR-036-trait-stdlib-redesign.md)
- [ADR-038](ADR-038-operator-overload-traits.md)
- [ADR-046](ADR-046-free-function-eradication.md)
- `issues/open/709-stdlib-trait-first-api-policy.md`
- `issues/done/157-adr004-method-syntax-evaluation.md`
