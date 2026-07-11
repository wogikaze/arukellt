# ADR-006: 公開 ABI を 3 層に固定

ステータス: **ACCEPTED** — 安定公開境界は WIT/canonical。raw Wasm GC layout は非 stable

決定日: 2026-03-24  
改訂日: 2026-07-11 — Layer 2A の GC 型 layout を stable から外す

---

## 文脈

公開 ABI を無秩序に増やすと保守コストが増大する。一方で raw Wasm の GC 型表現
（`String` / `Vec` の `(ref $…)` layout）を stable に固定すると、内部表現の進化
（inline string、rope、capacity layout、nullable 最適化、recursive type group 等）を
阻害する。

---

## 決定

**公開境界の枠は最大 3 層まで。これ以上増やさない。**

### 3 層構造

| Layer | 名称 | 公開範囲 | 互換性保証 |
|-------|------|---------|-----------|
| 1 | 内部 ABI | コンパイラ私有 | **なし** |
| 2B | Component / WIT / Canonical ABI | 安定な外部境界 | **維持する** |
| 2A | raw Wasm module ABI | 実験的・限定 | **experimental**（下記） |
| 3 | native 公開 ABI（予約） | native 外部境界 | **未決定**（ADR-045） |

Layer 番号の「2A/2B」は歴史的ラベルである。意味上は別種の境界であり、
「公開 ABI がちょうど 3 つ」という意味ではない。増やさない対象は
**安定公開面の種類**である。

### Layer 1: 内部 ABI（compiler-private）

- 関数呼び出し規約、スタックフレーム、レジスタ割り当て
- **`String` / `Vec` / enum / closure / trait object の GC 表現**はここに属する
- バージョン間の互換性保証なし。ADR-040 / ADR-042 の整理で変更してよい

### Layer 2B: 安定な外部境界（採択）

- Component Model / WIT / Canonical ABI を **stable public ABI** とする
- 言語セマンティクスから canonical lower/lift で投影する
- 相互運用・バージョニングの正本はこちら

### Layer 2A: raw Wasm ABI（experimental）

- 素の Wasm import/export 面。次のみを experimental 公開の候補とする:
  - スカラー値（`i32` / `i64` / `f32` / `f64`、および `bool`/`char` の整数表現）
  - 明示的に versioned された opaque handle
- **`(ref $string)` / `(ref $vec_T)` / enum・struct の GC type identity は
  stable ABI に含めない**（Layer 1）。別モジュール間で型 identity を共有する契約は未定義
- raw Wasm GC ABI を将来 stable にする場合は、recursive group・subtyping・
  import/export type・バージョニングを含む **独立 ABI 仕様**が必要（本 ADR の範囲外）

### Layer 3: native 公開 ABI（予約領域）

- 予約スロット。具体 ABI・構文・型制約は **未決定**（[ADR-045](ADR-045-llvm-scope-withdrawn.md)）
- portable Ark から直接利用可能とはしない

---

## 禁止事項

1. **安定公開面の無制限増加禁止** — WIT 専用・POSIX 専用などの第4の stable 面を作らない
2. **Layer 2B と言語セマンティクスの分岐禁止** — 片方のみで成功する API 形を正規にしない
3. **Layer 1 の GC layout を Layer 2A stable として公開しない**

---

## 参考: 現行 emitter の GC 表現（非契約）

実装の理解用。**互換性契約ではない。**

| arukellt の型 | 現行の典型的 Wasm 表現（変わりうる） |
|--------------|--------------------------------------|
| `i32` / `i64` / `f32` / `f64` | Wasm value type |
| `bool` / `char` | `i32` |
| `struct` / `enum` / `String` / `Vec[T]` / `Option` / `Result` | GC ref（layout は Layer 1） |

---

## 関連

- ADR-045: Layer 3 詳細は未決定
- ADR-008: component 生成（Layer 2B の成果物）
- ADR-040 / ADR-042: 内部型・intrinsic 整理（Layer 1 変更の根拠）
- [ADR-007: Targets](ADR-007-targets.md)
