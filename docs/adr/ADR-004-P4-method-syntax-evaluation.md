# ADR-004 P4: メソッド構文の評価

ステータス: **SUPERSEDED** — trait / メソッド構文は導入済みのため退役

後継: [ADR-036-trait-stdlib-redesign.md](ADR-036-trait-stdlib-redesign.md)  
日付: 2026-04-15  
統合日: 2026-07-11

---

## 廃止記録

本 ADR は、当初の trait 延期方針（旧 ADR-004）の下でメソッド構文（P4）を
いつ評価するかを定めていた。trait・`impl`・メソッド呼び出しは既に言語機能として
導入され、stdlib の trait-first 方針は [ADR-036](ADR-036-trait-stdlib-redesign.md)
（および ADR-038、issue #709）が正本である。

評価保留・trigger 待ちの枠組みは不要になったため、本文を削除し廃止記録のみ残す。
番号 `004` は恒久識別子として予約する。

関連: [ADR-036](ADR-036-trait-stdlib-redesign.md), [ADR-038](ADR-038-operator-overload-traits.md),
`issues/done/157-adr004-method-syntax-evaluation.md`, `issues/open/709-stdlib-trait-first-api-policy.md`
