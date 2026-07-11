# ADR-037: std::simd — Explicit SIMD Library API

ステータス: **PROPOSED** — portable nominal SIMD 型と raw `std::wasm::V128` の分離導入を提案

提案日: 2026-06-26  
改訂日: 2026-07-11 — nominal surface（const generics 非導入）、Mask/select 分離、capability 軸分離

---

## 文脈

Arukellt には SIMD サポートが存在しない。`std::wasm::valtype_v128` の定数バイトのみが
experimental で存在する。本 ADR は明示的 SIMD ライブラリ API としての導入入口を定義する。

Issue #107 (ループベクトル化ヒント) は reject 済みであり、本 ADR はその代替位置づけである
（hint ではなく explicit library API）。

### 関連 ADR

- ADR-002: Wasm GC 前提 — portable SIMD 値は GC struct/array フィールドに保持可能
- ADR-003: generics は**型パラメータのみ**（本 ADR は const generics を導入しない）
- ADR-006: 公開 ABI 境界 — SIMD の stable 公開は WIT/canonical。raw V128 は非 stable
- ADR-007 / ADR-013: ターゲットと capabilities
- ADR-014: 初期 API stability は experimental
- **ADR-042**: portable semantic op vs `std::wasm` target intrinsic（本 ADR はこれに従う）
- ADR-045: native SIMD はスコープ外

### Wasm 3.0 仕様上の前提

Wasm の `v128` は `valtype` の一種である。ただし **言語の portable SIMD 型と Wasm raw `v128`
は同一視しない**（下記決定）。

---

## 提案する決定

### 1. 二層の型モデル（必須分離）

| 層 | Surface 型 | 役割 | Scalar lowering | Component/WIT 公開 |
|----|------------|------|-----------------|-------------------|
| **Semantic** | nominal SIMD 型（下記） | portable ベクトル意味 | **可能** | 将来は canonical 投影を別途設計 |
| **Raw Wasm** | `std::wasm::V128` | Wasm 128-bit value category の露出 | **不可** | **不可** |

#### 1.1 Surface は nominal built-in 型（const generics なし）

ADR-003 の generic は型パラメータのみである。本 ADR は **const generics /
`Simd<T,N>` 公開構文を導入しない**。

初期 surface（実装核）:

```text
I32x4
F32x4
Mask4          // 4-lane boolean mask（I32x4 / F32x4 比較結果）
```

後続カタログ（仕様上定義してよいが初回実装必須ではない）:

```text
I8x16 U8x16 I16x8 U16x8 U32x4 I64x2 U64x2 F64x2
Mask8 Mask16 Mask2   // 対応 lane 数の mask
```

これらは **互いに異なる nominal built-in 型**である（薄い type alias ではない）。
alias にすると overload / trait impl / 診断表示の分離が難しくなるため採用しない。

#### 1.2 内部 MIR 表現

コンパイラ内部では共通構造でよい（公開構文ではない）:

```text
SimdType { lane_type: I32 | F32 | ..., lanes: u32 }
MaskType { lanes: u32 }
```

`I32x4` は `SimdType{I32,4}` へ、`Mask4` は `MaskType{4}` へ写像する。

#### 1.3 Well-formedness（初期仕様 = 128-bit portable SIMD）

許可される surface 型は次を満たすものに限る:

```text
lane_type ∈ { i8, u8, i16, u16, i32, u32, i64, u64, f32, f64 }
lanes > 0
LaneBits(lane_type) × lanes = 128
```

したがって初期仕様では例えば次は **ill-formed**:

```text
Simd 相当の String×4、i32×3、i64×8、bool×128、f32×1
```

- mask 型は通常 lane 型と別（`MaskN`）。`Simd<bool,N>` は採用しない。
- 256-bit 以上の portable vector は本 ADR のスコープ外（複数 `v128` 分割規則は別決定）。
- 同幅 lane 型間の bit reinterpret は portable な型変換 API（例: `I32x4::from_bits(F32x4)`）とし、
  `std::wasm` に置かない（ADR-042 D8）。
- facade としての `Vec<i32>` 擬似 SIMD は採用しない。

### 2. メモリモデル

- portable SIMD / mask 値は言語の値意味論に従う。GC struct/array フィールドに保持できる。
- `std::wasm::V128` は Wasm 値型の露出。**field / parameter / return / local のいずれかに
  現れた時点で** `wasm_raw_v128` capability を要求する（load/store 呼び出し時だけではない）。

### 3. Mask・select・bit_select（分離）

`select` と `bit_select` は別 API である。

```text
// Lane-wise select: 各 lane を boolean mask で選ぶ
select(mask: Mask4, a: I32x4, b: I32x4) -> I32x4
select(mask: Mask4, a: F32x4, b: F32x4) -> F32x4

// Bit-wise select: 128 bit それぞれで選ぶ（正規化 boolean lane は不要）
bit_select(a: I32x4, b: I32x4, mask_bits: I32x4) -> I32x4
```

- 比較（`==`, `<`, …）の結果型は対応する `MaskN`。
- mask の内部表現は実装詳細（典型: all-zero / all-ones lane）。surface では
  整数 vector への**暗黙変換はしない**。明示 API（例: `Mask4::to_i32x4()`）のみ。
- `any` / `all` / `bitmask` の入力は `MaskN`（整数 SIMD からの暗黙受けはしない）。
- `bit_select` は整数 SIMD 型に限定する（初期は `I32x4`。浮動小数への直接適用はしない）。

### 4. 初期 API 面（実装核）

```text
I32x4 / F32x4 / Mask4
splat / lane get-set
arithmetic / comparison → Mask4
select(Mask4, …) / bit_select（整数）
and / or / xor / not（整数 SIMD）
any / all / bitmask（Mask4）
同幅 reinterpret（I32x4 ↔ F32x4）
```

### 5. Capabilities（軸を分離）

単一の `simd128` enum に過積載しない。

```text
TargetCapabilities {
    portable_simd_lowering: NativeSimd | Scalar | Unsupported
    wasm_raw_v128: Enabled | Disabled
    wasm_relaxed_simd: Enabled | Disabled
}
```

| 軸 | 意味 |
|----|------|
| `portable_simd_lowering` | `I32x4` 等の portable API を native SIMD 命令へ出すか、scalar 同値計算するか |
| `wasm_raw_v128` | `std::wasm::V128` および raw load/store を許可するか |
| `wasm_relaxed_simd` | relaxed SIMD を許可するか |

典型:

```text
simd128 命令を出さない Wasm target:
  portable_simd_lowering = Scalar
  wasm_raw_v128 = Disabled

+simd128 有効:
  portable_simd_lowering = NativeSimd
  wasm_raw_v128 = Enabled
```

- portable 操作は `Scalar` で同値計算してよい（ADR-015: panic しない）。
- raw `V128` / relaxed は Scalar emulation **不可**。`Disabled` なら型出現時点でエラー。

### 6. 機能検出

実行時検出は行わない。コンパイル時の `TargetCapabilities` で決める。

### 7. 名前空間と所属（ADR-042 と一致）

```text
std::simd:
  nominal lane 型 / MaskN
  and / or / xor / not
  select / bit_select
  any / all / bitmask
  同幅 bit reinterpret
  fixed SIMD arithmetic / comparison / shuffle 等の portable 操作

std::wasm:
  std::wasm::V128 / valtype_v128
  linear-memory v128.load / store（load_splat / load_lane 等）
  relaxed SIMD
  Wasm 固有 trap・lane 規則を直接露出する raw 操作
```

`std::simd::bit_select` の backend が Wasm `v128.bitselect` を選ぶのは正しい lowering であり、
公開 API の所属を Wasm 命令名に合わせる必要はない。

### 8. メモリアクセス

1. `std::simd` は明示的な linear-memory load/store API を持たない。
2. GC / 言語値としての読み書きは field/index access に統合する。
3. Wasm `v128.load` / `v128.store` は `std::wasm` のみ。
4. GC Vec から raw pointer を取り出す API は提供しない。

### 9. バックエンド

必要な範囲で SIMD opcode / locals / MIR を追加する。raw `V128` と portable nominal 型の
lowering 経路を混線させない。

### 10. native backends

スコープ外（ADR-045 後継）。

### 11. stability

初期は `experimental`（ADR-014）。stable 昇格条件:

1. portable API（型・演算・mask/select）が破壊的変更なしで固定
2. `NativeSimd` / `Scalar` で portable 意味が一致
3. raw `V128` 境界が確定（本 ADR §1 / §5 / §7）
4. conformance + lowering differential tests

### 12. Issue #107

hint-based autovectorization は採用しない。将来再評価する場合も内部は `SimdType` MIR へ正規化する。

### 13. `valtype_v128`

既存定数は `std::wasm` に残し、portable `std::simd` へ混ぜない。

### 14. 将来の const generics（非決定・非導入）

`Simd<T, const N: usize>` 相当を公開したくなった場合は **別 ADR** で構文・型同値性・
定数式を決める。本 ADR の採択はそれを前提にしない。

---

## 禁止事項

1. `std::simd` に `v128.load` / `v128.store` を混ぜない
2. portable `bit_select` / `reinterpret` / bitwise を `std::wasm` に置かない（ADR-042）
3. raw `V128` を Scalar emulation しない
4. raw `V128` を Component/WIT の stable 公開面に載せない
5. 全 lane 型の同時実装を初回必須にしない
6. 本 ADR で const generics / 公開 `Simd<T,N>` 構文を導入しない
7. `select` と `bit_select` を同一 API に混ぜない
8. mask と整数 SIMD の暗黙変換を許さない

---

## 結果

- portable SIMD が現行型システム（ADR-003）の範囲で表現できる
- raw Wasm value と capability・名前空間が分離される
- Mask / lane-select / bit-select の意味が分離される
- ADR-042 と矛盾しない

## 関連

- [ADR-042](ADR-042-intrinsic-layer-separation.md)
- ADR-002 / ADR-003 / ADR-006 / ADR-007 / ADR-014 / ADR-045
- `issues/reject/107-runtime-loop-vectorization-hint.md`
- [WebAssembly 3.0 Spec — Types](https://webassembly.github.io/spec/core/syntax/types.html)
