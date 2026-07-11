# ADR-037: std::simd — Portable SIMD 再設計と既存 API からの移行

ステータス: **PROPOSED** — 既存 experimental SIMD API を portable nominal 型と
raw `std::wasm::V128` へ再設計・移行する

提案日: 2026-06-26  
改訂日: 2026-07-11 — 「未実装」前提を撤回。#698 実装済み API からの移行 ADR へ変更。
比較演算・Mask 表現・migration table を確定

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
| `std::wasm::v128_and/or/xor/not` | portable `std::simd` bitwise | deprecate 後削除 |
| `std::wasm::v128_bitselect` | `std::simd::bit_select` | deprecate 後削除 |
| `std::wasm::v128_any_true` | `MaskN::any` / portable any | deprecate 後削除 |
| `std::wasm::v128_reinterpret_*` | portable `from_bits` / reinterpret | deprecate 後削除 |
| raw load/store / relaxed / valtype | `std::wasm` に残置 | 維持（型名は `V128` へ寄せる） |
| 無印 `v128` 型そのもの | `std::wasm::V128`（raw 専用） | portable 用途から撤退 |

**互換期間（提案）:**

1. ADR 採択後: 新 API を追加。旧 API に `deprecated_by`（W0008）
2. 1 リリース以上: 旧・新併存。fixture を新 API へ移行
3. その後: 旧 `std::simd::*` モジュール関数と誤配置 `std::wasm::v128_{and,bitselect,reinterpret,*}` を削除

`std::simd::v128` モジュールは portable に raw を混ぜているため **廃止対象**。
中身は lane 型 API または `std::wasm::V128` へ振り分ける。

### 3. 比較演算と Eq/Ord（ADR-036/038 との統合）

`==` / `<` の結果を `Mask4` にしてはならない。

| 式 | 結果型 | 意味 |
|----|--------|------|
| `a == b` / `a != b`（SIMD） | `bool` | **全 lane 一致**のスカラー等価（`Eq` と整合） |
| `a.cmp_eq(b)` / `lanes_eq(a,b)` | `Mask4` | lane-wise 等価 |
| `a.cmp_lt(b)` / `lanes_lt(a,b)` 等 | `Mask4` | lane-wise 順序 |

- SIMD 型は `Eq` を実装してよい（`eq` → `bool`）。
- lane 比較は `Eq`/`Ord` のメソッド型を変えず、**別 semantic family**（`cmp_*` / `lanes_*`）。
- ADR-038 の演算子 trait は算術・bitwise 向け。SIMD の `==` を Mask 化する規則は導入しない。

### 4. Mask 表現・bitmask・bit_select（観測可能な仕様）

明示変換がある以上、表現は実装詳細にできない。

#### 4.1 `Mask4::to_i32x4()`（および同幅整数への明示変換）

```text
false lane → 0x00000000
true  lane → 0xffffffff
```

逆変換 `Mask4::from_i32x4(v)` は各 lane の MSB（または「非ゼロ」ではなく **all-ones 判定**）を
仕様化する: **lane == 0xffffffff なら true、それ以外は false**（部分ビットは true にしない）。

#### 4.2 `bitmask(mask: Mask4) -> u8`（初期核）

```text
bit i = 1 ⇔ lane i が true
lane 0 → bit 0（LSB）
lane 3 → bit 3
上位ビットは 0
```

（Wasm `i32x4.bitmask` と同じ lane→bit 対応。）

#### 4.3 `bit_select(a, b, mask_bits) -> T`（整数 SIMD）

現行 `std::wasm::v128_bitselect` と同じ式:

```text
(a & mask_bits) | (b & ~mask_bits)
```

各ビットについて mask が 1 なら `a`、0 なら `b`。正規化 boolean lane は不要。
浮動小数 SIMD への直接 `bit_select` は提供しない（必要なら `from_bits` 経由）。

#### 4.4 lane `select`

```text
select(mask: Mask4, a: I32x4, b: I32x4) -> I32x4
```

各 lane について mask が true なら `a`、false なら `b`（boolean lane select）。

### 5. メモリモデル

- portable SIMD / Mask は言語値。GC field に保持可。
- `std::wasm::V128` は **field / parameter / return / local のいずれかに現れた時点で**
  `wasm_raw_v128 = Enabled` を要求。

### 6. Capabilities（三軸）と現行ギャップ

```text
TargetCapabilities {
    portable_simd_lowering: NativeSimd | Scalar | Unsupported
    wasm_raw_v128: Enabled | Disabled
    wasm_relaxed_simd: Enabled | Disabled
}
```

**現行実装ギャップ（採択後も living）:** `is_simd_target()` が全 target で `true` のため、
portable Scalar 経路が選ばれず、raw/portable 判定も未分離。差は `current-state.md`
ADR contract gaps に記録する（本 ADR の理想契約と混同しない）。

### 7. 名前空間（ADR-042）

```text
std::simd:  nominal 型、bitwise、select/bit_select、cmp_*、any/all/bitmask、reinterpret
std::wasm:  V128、v128.load/store、relaxed、Wasm 固有 raw
```

### 8. メモリアクセス / バックエンド / native

- `std::simd` に linear-memory load/store を置かない（現状どおり境界は維持・強化）。
- raw と portable の lowering を混線させない。
- native: #699 / ADR-045。本 ADR の採択条件に含めない。

### 9. stability

既存 #698 API・本提案 API とも当面 `experimental`（ADR-014）。
stable 昇格は移行完了（旧 API 削除）+ NativeSimd/Scalar 同値 + raw 境界固定 + tests。

### 10. #698 / #699 の扱い

| Issue | 扱い |
|-------|------|
| #698 | **実装済み**の現行 experimental 面。本 ADR はその面を移行対象として認識する |
| #699 | native LLVM SIMD。本 ADR 採択後も open。portable 契約の前提にしない |
| #107 | reject のまま（hint は採用しない） |

---

## 禁止事項

1. 「SIMD 未実装」を前提にした新規導入ストーリーで本 ADR を読ませない
2. 旧 API を deprecate なしに削除しない
3. `==` / `<` の結果を `MaskN` にしない
4. Mask の公開変換・bitmask・bit_select のビット意味を backend 依存にしない
5. portable bitwise / bit_select / reinterpret を `std::wasm` に残したまま stable 化しない
6. const generics / 公開 `Simd<T,N>` を本 ADR で導入しない
7. raw `V128` の Scalar emulation / WIT stable 公開をしない

---

## 結果

- 既存 #698 API を認識した移行契約になる
- portable / raw の型・名前空間・capability が分離される
- `Eq` と lane 比較が衝突しない
- Mask / bit_select の観測可能意味が固定される

## 関連

- [ADR-042](ADR-042-intrinsic-layer-separation.md)
- ADR-036 / ADR-038（演算子・Eq）
- `issues/done/698-std-simd-explicit-library.md`
- `issues/open/699-t4-llvm-native-simd-lowering.md`
- `issues/reject/107-runtime-loop-vectorization-hint.md`
- [WebAssembly 3.0 Spec — Types](https://webassembly.github.io/spec/core/syntax/types.html)
