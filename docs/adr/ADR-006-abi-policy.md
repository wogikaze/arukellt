# ADR-006: 公開 ABI の上限

ステータス: **DECIDED**

決定日: 2026-03-24

---

## 文脈

公開 ABI を増やすと保守コストが増大する。以下のリスクがある:

- WIT / native-posix / C ABI の三重化
- バージョン間の互換性維持が困難
- 各 ABI でのテストが必要

---

## 決定

**公開 ABI は 3 層まで。これ以上増やさない。**

### 3 層構造

| Layer | 名称 | 公開範囲 | 互換性保証 |
|-------|------|---------|-----------|
| 1 | 内部 ABI | 非公開 | なし |
| 2 | WASM 公開 ABI | Wasm モジュール間 | v0 以降 |
| 3 | native 公開 ABI | C ライブラリとの FFI | C ABI 準拠 |

### Layer 1: 内部 ABI

- arukellt コンパイラ独自
- バージョン間の互換性保証なし
- 関数呼び出し規約、スタックフレーム、レジスタ割り当て

### Layer 2: WASM 公開 ABI

- 素の Wasm import / export
- 数値型は Wasm value type そのまま
- 複合型は GC heap 上の参照で渡す

将来の拡張:
- Component Model / WIT は Layer 2 の拡張として扱う
- 独立した Layer 4 にはしない
- v0 では対応しない

### Layer 3: native 公開 ABI

- C ABI（System V AMD64 / Windows x64）のみ
- POSIX / Windows の差異は platform 抽象層が吸収
- arukellt 独自の拡張は入れない
- LLVM IR バックエンドからのみ使用

---

## 禁止事項

1. **Layer 4 の追加禁止**
   - 「WIT 専用層」「POSIX 専用層」は作らない
   - 必要なら Layer 2 または Layer 3 の拡張として吸収

2. **ABI の独自拡張禁止**
   - 標準から外れる呼び出し規約は入れない
   - 他ツールとの相互運用性を維持

---

## 型の ABI 表現（WASM 公開 ABI）

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

- ADR-005: LLVM IR の役割制限
- `docs/platform/abi.md`: ABI 詳細
- `docs/platform/wasm-features.md`: 使用する Wasm 機能
