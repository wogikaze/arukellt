# ADR-037: std::simd — Portable SIMD 再設計と既存 API からの移行

ステータス: **PROPOSED** — 既存 experimental SIMD API を portable nominal 型と raw `std::wasm::V128` へ再設計・移行する

提案日: 2026-06-26  
改訂日: 2026-07-11 — 移行 ADR 化に加え、Eq/PartialEq 分離・`<` 禁止・any_true 分離・Mask API 名・ADR-036 D2 例外表現・core-ops 統一を確定

---

## 文脈

### 現行状態（実装先行）

SIMD は **すでに experimental として実装・文書化されている**（#698 done）。

現行（`std/manifest.toml` / `docs/stdlib/*` / platform SIMD 節）:

```text
first-class v128（lane 非区別の値型として多用）
std::simd::{i8x16,u8x16,i16x8,...,f32x4,f64x2} モジュール関数
std::simd::v128::{and,or,xor,not,bitselect,...}
std::wasm::{v128_and,v128_bitselect,v128_reinterpret_*,...}
valtype_v128
```

`docs/platform/target-runtime-and-surfaces.md` は 11 lane 型・raw intrinsics・
GC field fixture を実装済みとして扱う。スカラー展開コードは存在するが、
現行 `is_simd_target()` は全ターゲットで `true` を返す。

したがって本 ADR は「SIMD の新規導入」ではない。実態は:

> 既存の実装先行 SIMD API を、portable nominal 型と raw Wasm 型へ再設計する **移行 ADR**

Issue #107（ループベクトル化ヒント）は reject 済み。#698 がその代替実装であり、
本 ADR は #698 面の **再設計契約** を固定する。native LLVM SIMD は #699（open）。

### 関連 ADR

- ADR-002 / ADR-006 / ADR-007 / ADR-013 / ADR-014 / ADR-015
- ADR-003: generics は型パラメータのみ（const generics 非導入）
- ADR-036 / ADR-038: `Eq`/`Ord` と演算子 — SIMD 比較は本 ADR で別規則
- **ADR-042**: portable vs `std::wasm` target intrinsic
- ADR-045 / #699: native SIMD はスコープ外（本 ADR 採択後も別 issue）

---

## 提案する決定

### 1. 二層の型モデル（必須分離）

| 層 | Surface 型 | 役割 | Scalar lowering | Component/WIT |
|----|------------|------|----------------|---------------|
| **Semantic** | nominal SIMD / Mask | portable ベクトル意味 | **可能** | 将来 canonical 投影を別途 |
| **Raw Wasm** | `std::wasm::V128` | Wasm 128-bit value category | **不可** | **不可** |

#### 1.1 Surface は nominal built-in（const generics なし）

公開構文に `Simd<T,N>` は出さない。初期核:

```text
I32x4
F32x4
Mask4
```

後続カタログ（初回必須ではない）: `I8x16` … `F64x2` および対応 `MaskN`。

これらは **互いに異なる nominal built-in**（薄い alias ではない）。
内部 MIR のみ `SimdType { lane_type, lanes }` / `MaskType { lanes }` でよい。

#### 1.1a 型 identity の所有者

| 層 | 置き場 | 役割 |
|----|--------|------|
| Compiler semantic type | TypeTable の `TypeId` + `TypeKind` | 型検査・MIR・lowering の正本 |
| Public stdlib path | `std::simd::I32x4` / `F32x4` / `Mask4`、`std::wasm::V128` | ユーザーが書く名前 |
| Prelude | **自動 import しない**（初期） | 明示 `use std::simd::I32x4` |

**TypeKind（別 identity）:**

```text
TypeKind::Simd { lane_type, lanes }   // I32x4, F32x4, …
TypeKind::Mask { lanes }              // Mask4, …
TypeKind::WasmV128                    // std::wasm::V128 のみ
```

- `I32x4` と `V128` はどちらも 128 bit でも **異なる TypeId**。暗黙変換は禁止。
  明示変換（`from_bits` / raw cast API）のみ。
- parser/typechecker が文字列 `"I32x4"` を個別ハードコード判定する構造にはしない。
  公開 path → TypeId 解決の後、MIR は TypeId / MirValueType のみを持つ。
- **不変条件:** 同一の generated CoreTypeSpec（または同等の registry）から
  TypeTable 登録と stdlib 公開情報を生成し、手作業で二重登録しない。
  型エントリの正本は [`data/core-ops.toml`](../../data/core-ops.toml) の `[[types]]`
  （ADR-042 D5）。manifest は `type_id` で参照するのみ。

#### 1.2 Well-formedness（初期 = 128-bit）

```text
lane_type ∈ { i8,u8,i16,u16,i32,u32,i64,u64,f32,f64 }
lanes > 0
LaneBits(lane_type) × lanes = 128
```

`MaskN` は通常 lane 型と別。`Simd<bool,N>` は採用しない。256-bit+ はスコープ外。

### 2. 現行 API → 提案 API（migration）

破壊的変更を無言で行わない。移行表:

| Current（#698 / manifest） | Proposed | 扱い |
|----------------------------|----------|------|
| `std::simd::i32x4::*` モジュール関数 | `I32x4` のメソッド / 演算子 | deprecate → 削除 |
| `std::simd::f32x4::*` 等 lane モジュール | 対応 nominal 型 | 同上 |
| `std::simd::v128::{and,or,xor,not,bitselect}` | `I32x4` 等の portable bitwise / `bit_select` | deprecate → 削除（portable へ） |
| first-class 無印 `v128` を portable 戻り値に使う面 | nominal lane 型 | 段階的に型を分離 |
| `std::wasm::v128_and/or/xor/not` | portable 整数 SIMD の bitwise **および** raw `V128` 上の同名操作 | portable 側へ意味を移す。raw `V128::{and,or,xor,not}` は raw value category 操作として `std::wasm` に**残してよい** |
| `std::wasm::v128_bitselect` | `std::simd::bit_select`（整数 SIMD） | deprecate 後、portable へ。raw に残す場合は `V128::bit_select` として型を分ける |
| `std::wasm::v128_any_true` | **`V128::any_bit_set()`**（raw 128-bit reduction） | `MaskN::any` へ写さない（意味が異なる）。整数 SIMD には任意で `any_nonzero_bits()` を別途提供可 |
| `std::wasm::v128_reinterpret_*` | portable `from_bits` / reinterpret | deprecate 後削除（portable へ） |
| raw load/store / relaxed / valtype | `std::wasm` に残置 | 維持（型名は `V128` へ寄せる） |
| 無印 `v128` 型そのもの | `std::wasm::V128`（raw 専用） | portable 用途から撤退 |

**互換期間（提案）— ADR-036 D2 より厳しい個別移行:**

experimental API は直接削除可能という ADR-036 D2 の既定より **厳しい個別移行方針**
を採用する。SIMD は型 identity と名前空間を変更し、raw/portable の意味分割を伴うため、
少なくとも 1 リリースの deprecation 期間を設ける。

1. ADR 採択後: 新 API を追加。旧 API に `deprecated_by`（W0009）
2. 1 リリース以上: 旧・新併存。fixture を新 API へ移行
3. その後: 旧 `std::simd::*` モジュール関数と、portable へ移した誤配置
   `std::wasm::v128_*`（reinterpret 等）を削除。raw として正当な `V128` 操作は残す

`std::simd::v128` モジュールは portable に raw を混ぜているため **廃止対象**。
中身は lane 型 API または `std::wasm::V128` へ振り分ける。

### 3. 比較演算と Eq/Ord（ADR-036/038 との統合）

演算子の結果を `MaskN` にしてはならない。さらに `<` / `>` / `<=` / `>=` は
SIMD 型に対して **提供しない**（意味が曖昧なため）。

| 式 | 結果型 | 意味 | 備考 |
|----|--------|------|------|
| `a == b` / `a != b` | `bool` | 全 lane について scalar `==` が true | 下記 trait と整合 |
| `a < b` 等 | — | **禁止**（型エラー） | lexicographic が必要なら明示 API |
| `a.cmp_eq(b)` / `lanes_eq(a,b)` | `MaskN` | lane-wise 等価 | |
| `a.cmp_lt(b)` / `lanes_lt(a,b)` 等 | `MaskN` | lane-wise 順序 | IEEE lane 比較（NaN は false） |
| `a.lexicographic_cmp(b)` | `Ordering` | 任意・後続 | 初期核では不要 |

**Trait 実装（整数 vs 浮動小数）:**

| 型 | `PartialEq` | `Eq` | `PartialOrd` / `Ord` |
|----|-------------|------|----------------------|
| `I32x4` 等整数 SIMD | ✅ | ✅ | 初期は不要（`<` 禁止）。必要なら後続 |
| `F32x4` / `F64x2` | ✅（全 lane scalar `==`） | **❌** | **❌**（`<` 禁止。lane 比較は `cmp_*` のみ） |

- `F32x4 == F32x4 → bool` は「全 lane の scalar 浮動小数 `==`」と定義する。
  NaN を含むと反射律が壊れるため **`Eq` にはしない**（`PartialEq` のみ）。
- bitwise equality（`+0.0`≠`-0.0`、同一 NaN payload なら等しい）は通常の `==` に使わない。
  必要なら `I32x4::from_bits(a) == I32x4::from_bits(b)` 等の明示経路。
- lane 比較は `Eq`/`Ord` のメソッド型を変えず、**別 semantic family**（`cmp_*`）。
- ADR-038 の演算子 trait は算術・bitwise 向け。SIMD の `==` を Mask 化しない。

### 4. Mask 表現・bitmask・bit_select・any（観測可能な仕様）

明示変換がある以上、表現は実装詳細にできない。

#### 4.1 canonical bits

```text
Mask4::to_i32x4():
  false → 0x00000000
  true  → 0xffffffff
```

逆変換は次の二つだけを公開する（lossy な `from_canonical_i32x4` は採用しない）:

```text
Mask4::try_from_canonical_bits(v) -> Result<Mask4, InvalidMaskBits>
  全 lane が 0x00000000 または 0xffffffff のときのみ Ok
  それ以外（0x1, 0x80000000, 0xfffffffe 等）→ Err

Mask4::from_nonzero_lanes(v) -> Mask4
  lane != 0 → true（別セマンティクス。canonical とは混同しない）
```

backend 内部で検証済み canonical bits を載せる必要がある場合は、
公開 API に出さない unchecked 経路（実装詳細）とする。

#### 4.2 `bitmask(mask: MaskN) -> u32`

初期核も後続 `Mask16` も **常に `u32`**（generic / 実装単純化のため）:

```text
bit i = 1 ⇔ lane i が true
lane 0 → bit 0（LSB）
上位の未使用 bit は 0
```

（Wasm `i32x4.bitmask` と同じ lane→bit 対応。）

#### 4.3 `bit_select(a, b, mask_bits) -> T`（整数 SIMD）

```text
(a & mask_bits) | (b & ~mask_bits)
```

#### 4.4 lane `select` と any の分離

```text
select(mask: Mask4, a: I32x4, b: I32x4) -> I32x4   // boolean lane select
Mask4::any() / all()                                 // boolean lane reduce
V128::any_bit_set()                                  // raw: 128bit に 1 が一つでもあれば true
I32x4::any_nonzero_bits()                            // 任意: portable 整数の非ゼロ bit reduce
```

`v128_any_true` の移行先は **`V128::any_bit_set`** であり `MaskN::any` ではない。

### 5. Portable 意味論の範囲（NativeSimd ↔ Scalar）

`portable_simd_lowering` の同値性は、**lane ごとの Arukellt scalar 演算**を正とする
（詳細は [RFC-003](../rfcs/003-portable-simd-semantics.md)）。

**初期核（本 ADR の portable 契約）:**

```text
splat / lane access
integer add/sub/mul（wrapping）
bitwise / bit_select / select
cmp_* → MaskN
any / all / bitmask
同幅 portable from_bits（例）:
  F32x4::from_bits(I32x4) -> F32x4
  F32x4::to_bits() -> I32x4
```

`I32x4 ↔ V128` は portable reinterpret ではなく raw 境界なので、別の明示 raw cast API とする
（`std::wasm` 側）。portable `from_bits` は同幅 nominal SIMD 同士に限定する。

初期核に含めない（追加前に RFC-003 改訂が必要）: float min/max、narrowing/widening、
saturating、shift count mask、float↔int convert、shuffle、relaxed。

stable 昇格条件の「NativeSimd / Scalar 同値」は、上記初期核に限定して解釈する。

### 6. メモリモデル

- portable SIMD / Mask は言語値。GC field に保持可。
- `std::wasm::V128` は **field / parameter / return / local のいずれかに現れた時点で**
  `wasm_raw_v128 = Enabled` を要求。

### 7. Capabilities（三軸）と現行ギャップ

```text
TargetCapabilities {
    portable_simd_lowering: NativeSimd | Scalar | Unsupported
    wasm_raw_v128: Enabled | Disabled
    wasm_relaxed_simd: Enabled | Disabled
}
```

**現行実装ギャップ（PROPOSED 期間中）:** `is_simd_target()` が全 target で `true`。
差は `current-state.md` の **Proposed migration gaps** に記録する
（Accepted ADR の公開契約ギャップと混同しない）。

### 8. 名前空間（ADR-042）

```text
std::simd:  nominal 型、bitwise、select/bit_select、cmp_*、any/all/bitmask、reinterpret
std::wasm:  V128、v128.load/store、relaxed、Wasm 固有 raw
```

### 9. メモリアクセス / バックエンド / native

- `std::simd` に linear-memory load/store を置かない（現状どおり境界は維持・強化）。
- raw と portable の lowering を混線させない。
- native: #699 / ADR-045。本 ADR の採択条件に含めない。

### 10. stability

既存 #698 API・本提案 API とも当面 `experimental`（ADR-014）。
stable 昇格は移行完了（旧 API 削除）+ 初期核の NativeSimd/Scalar 同値（RFC-003）+
raw 境界固定 + tests。

### 11. #698 / #699 の扱い

| Issue | 扱い |
|-------|------|
| #698 | **実装済み**の現行 experimental 面。本 ADR はその面を移行対象として認識する |
| #699 | native LLVM SIMD。本 ADR 採択後も open。portable 契約の前提にしない |
| #107 | reject のまま（hint は採用しない） |

---

## 禁止事項

1. 「SIMD 未実装」を前提にした新規導入ストーリーで本 ADR を読ませない
2. 旧 API を deprecate なしに削除しない
3. `==` の結果を `MaskN` にしない。SIMD に `<` / `>` / `<=` / `>=` を提供しない
4. `F32x4` / `F64x2` に `Eq` を実装しない
5. `v128_any_true` を `MaskN::any` へ写さない
6. lossy な `from_canonical_i32x4` を公開しない（`try_from_canonical_bits` / `from_nonzero_lanes` のみ）
7. Mask の公開変換・bitmask・bit_select のビット意味を backend 依存にしない
8. portable 意味と raw `V128` 操作を型なしに同一 API へ混同しない
9. const generics / 公開 `Simd<T,N>` を本 ADR で導入しない
10. raw `V128` の Scalar emulation / WIT stable 公開をしない
11. experimental 直接削除の既定を理由に SIMD 旧 API を deprecate なし削除しない（本 ADR の個別移行方針に従う）

---

## 結果

- 既存 #698 API を認識した移行契約になる
- portable / raw の型・名前空間・capability が分離される
- 整数 SIMD は `Eq`、浮動小数 SIMD は `PartialEq` のみ。`<` は禁止、lane 比較は `cmp_*`
- Mask / bit_select / `any_bit_set` の観測可能意味が固定される

## 関連

- [RFC-003](../rfcs/003-portable-simd-semantics.md) — NativeSimd ↔ Scalar 同値の範囲
- [ADR-042](ADR-042-intrinsic-layer-separation.md)
- ADR-036 / ADR-038（演算子・Eq）
- `issues/done/698-std-simd-explicit-library.md`
- `issues/open/699-t4-llvm-native-simd-lowering.md`
- `issues/reject/107-runtime-loop-vectorization-hint.md`
- [WebAssembly 3.0 Spec — Types](https://webassembly.github.io/spec/core/syntax/types.html)
