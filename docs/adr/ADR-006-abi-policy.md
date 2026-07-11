# ADR-006: 公開 ABI を 3 層に固定

ステータス: **ACCEPTED** — 公開 ABI は最大 3 層（Layer 3 native は予約・詳細未決定）

決定日: 2026-03-24

---

## 文脈

公開 ABI を無秩序に増やすと保守コストが増大する。以下のリスクがある:

- Layer の無制限増加
- バージョン間の互換性維持が困難
- 各公開面でのテストが必要

---

## 決定

**公開 ABI は 3 層まで。これ以上増やさない。**

### 3 層構造

| Layer | 名称 | 公開範囲 | 互換性保証 |
|-------|------|---------|-----------|
| 1 | 内部 ABI | 非公開 | なし |
| 2 | WASM 公開 ABI | Wasm モジュール間 | 互換性を維持する |
| 3 | native 公開 ABI（予約） | native 外部境界 | **未決定**（ADR-045） |

### Layer 1: 内部 ABI

- arukellt コンパイラ独自
- バージョン間の互換性保証なし
- 関数呼び出し規約、スタックフレーム、レジスタ割り当て

### Layer 2: WASM 公開 ABI

- 2 つの公開面を持つ:
  - **Layer 2A: raw Wasm ABI**（素の import/export）
  - **Layer 2B: Component Model / WIT ABI**（WASI Preview 2、canonical ABI）
- どちらも Layer 2 の範囲に含める
- 独立した Layer 4 にはしない
- raw Wasm 面と WIT 面は同じ言語セマンティクスから生成する

### Layer 3: native 公開 ABI（予約領域）

- Layer 3 は **native 外部境界のための予約スロット**である。
- 具体的な ABI（C ABI か否か）、ソース構文（`extern "C"` 等）、型制約、
  target module 境界は **未決定**（[ADR-045](ADR-045-llvm-scope-withdrawn.md)）。
- portable な Ark コードから直接利用可能とはしない。
- scaffold 実装の実験は妨げないが、本層の契約を ACCEPTED として固定しない。
---

## 禁止事項

1. **Layer 4 の追加禁止**
   - 「WIT 専用層」「POSIX 専用層」は作らない
   - 必要なら Layer 2 または Layer 3 の拡張として吸収

2. **Layer 2A / 2B の意味論分岐禁止**
   - raw Wasm 面と WIT 面で言語仕様を分岐させない
   - 片方のみで成功する API 形は正規 API にしない

3. **ABI の独自拡張禁止**
   - 標準から外れる呼び出し規約は入れない
   - 他ツールとの相互運用性を維持

---

## 型の ABI 表現（WASM 公開 ABI）

Layer 2A / 2B で共有する型意味論:

- Layer 2A（raw Wasm）は GC 参照や value type を直接使う
- Layer 2B（WIT）は canonical ABI の lower/lift で同値な値表現に落とす

| arukellt の型 | ABI 表現 |
|--------------|---------|
| `i32` / `i64` / `f32` / `f64` | Wasm value type そのまま |
| `bool` | `i32`（0 or 1） |
| `char` | `i32`（Unicode scalar value） |
| `struct` | `(ref $struct_type)` |
| `enum` | `(ref $enum_type)` |
| `String` | `(ref $string)` |
| `Vec[T]` | `(ref $vec_T)` |
| `Option[T]` where T is ref | `(ref null $T)` |
| `Option[T]` where T is value | `(ref $option_T)` |
| `Result[T, E]` | `(ref $result_T_E)` |

---

## 関連

- ADR-045: 旧 LLVM 方針の撤回（Layer 3 詳細は再開まで未決定）
- `docs/platform/abi.md`: ABI 詳細
- [ADR-007: Targets](ADR-007-targets.md): 使用する Wasm 機能
