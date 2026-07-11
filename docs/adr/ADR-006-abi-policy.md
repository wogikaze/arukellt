# ADR-006: 公開 ABI 境界の分類

ステータス: **ACCEPTED** — 安定公開境界は WIT/canonical。raw Wasm GC layout は非 stable

決定日: 2026-03-24  
改訂日: 2026-07-11 — 層番号を廃止し、境界カテゴリ名で記述

---

## 文脈

公開 ABI を無秩序に増やすと保守コストが増大する。一方で raw Wasm の GC 型表現
（`String` / `Vec` の `(ref $…)` layout）を stable に固定すると、内部表現の進化
（inline string、rope、capacity layout、nullable 最適化、recursive type group 等）を
阻害する。

---

## 決定

**安定公開面の種類を増やさない。** 境界は次のカテゴリで分類する
（番号付き「3 層」ではない。compiler-private は公開 ABI ではない）。

| カテゴリ | 公開範囲 | 互換性保証 |
|----------|---------|-----------|
| Compiler-private ABI | コンパイラ私有 | **なし** |
| Stable interoperability ABI | WIT / Canonical ABI | **維持する** |
| Experimental raw Wasm ABI | 素の Wasm import/export（限定） | **experimental** |
| Reserved native ABI | native 外部境界 | **未決定**（ADR-045） |

### Compiler-private ABI

- 関数呼び出し規約、スタックフレーム、レジスタ割り当て
- **`String` / `Vec` / enum / closure / trait object の GC 表現**はここに属する
- バージョン間の互換性保証なし。ADR-040 / ADR-042 の整理で変更してよい

### Stable interoperability ABI（採択）

- Component Model / WIT / Canonical ABI を **stable public ABI** とする
- 言語セマンティクスから canonical lower/lift で投影する
- 相互運用・バージョニングの正本はこちら

### Experimental raw Wasm ABI

- 素の Wasm import/export 面。次のみを experimental 公開の候補とする:
  - スカラー値（`i32` / `i64` / `f32` / `f64`、および `bool`/`char` の整数表現）
  - 明示的に versioned された opaque handle
- **`(ref $string)` / `(ref $vec_T)` / enum・struct の GC type identity は
  stable ABI に含めない**（compiler-private）。別モジュール間で型 identity を共有する契約は未定義
- raw Wasm GC ABI を将来 stable にする場合は、recursive group・subtyping・
  import/export type・バージョニングを含む **独立 ABI 仕様**が必要（本 ADR の範囲外）

### Reserved native ABI

- 予約スロット。具体 ABI・構文・型制約は **未決定**（[ADR-045](ADR-045-llvm-scope-withdrawn.md)）
- portable Ark から直接利用可能とはしない

---

## 禁止事項

1. **安定公開面の無制限増加禁止** — WIT 専用・POSIX 専用などの第4の stable 面を作らない
2. **Stable WIT/canonical と言語セマンティクスの分岐禁止** — 片方のみで成功する API 形を正規にしない
3. **compiler-private の GC layout を experimental raw Wasm の stable 契約として公開しない**

---

## 参考: 現行 emitter の GC 表現（非契約）

実装の理解用。**互換性契約ではない。**

| arukellt の型 | 現行の典型的 Wasm 表現（変わりうる） |
|--------------|--------------------------------------|
| `i32` / `i64` / `f32` / `f64` | Wasm value type |
| `bool` / `char` | `i32` |
| `struct` / `enum` / `String` / `Vec[T]` / `Option` / `Result` | GC ref（layout は compiler-private） |

---

## 関連

- ADR-045: native ABI 詳細は未決定
- ADR-008: component 生成（stable WIT/canonical の成果物）
- ADR-040 / ADR-042: 内部型・intrinsic 整理（compiler-private 変更の根拠）
- [ADR-007: Targets](ADR-007-targets.md)
