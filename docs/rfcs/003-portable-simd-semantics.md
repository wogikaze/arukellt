# RFC-003: Portable SIMD operation semantics（NativeSimd ↔ Scalar）

ステータス: **DRAFT**  
関連: [ADR-037](../adr/ADR-037-std-simd.md)、[ADR-042](../adr/ADR-042-intrinsic-layer-separation.md)  
提案日: 2026-07-11  
改訂日: 2026-07-11 — 初期 catalog・overflow/NaN・differential 三者比較を固定

---

## 目的

ADR-037 の `portable_simd_lowering: NativeSimd | Scalar` について、
**どの操作が portable 契約に入り、何をもって同値とするか**を固定する。

---

## 決定（提案）

### 1. 意味論の基準

portable `std::simd` 操作の意味は、**各 lane に Arukellt scalar 演算を適用した結果**を正とする。

- Scalar lowering はその定義をそのまま実行する。
- NativeSimd（Wasm SIMD 等）は観測可能に一致しなければならない。
- Wasm 命令の癖を言語仕様の正本にしない。
- 「実装済み Wasm 命令だから」と未定義 op を自動公開しない。

### 2. 初期 operation catalog（有限）

#### `I32x4`

```text
splat
extract_lane / replace_lane
add / sub / mul          — lane ごと 2^32 modulo wrapping（scalar i32 と同じ）
and / or / xor / not
cmp_eq / cmp_ne / cmp_lt / cmp_le / cmp_gt / cmp_ge  → Mask4
select(mask: Mask4, a, b)
bit_select(a, b, mask_bits: I32x4)  — (a & mask_bits) | (b & ~mask_bits)
```

#### `F32x4`

```text
splat
extract_lane / replace_lane
add / sub / mul / div    — lane ごと scalar f32 と同じ
cmp_*                    → Mask4（scalar 比較）
select(mask: Mask4, a, b)
from_bits / to_bits      — ビットパターンを正確に保存
```

#### `Mask4`

```text
any / all / bitmask
to_i32x4
try_from_canonical_bits / from_nonzero_lanes
```

### 3. 初期仕様から外す（RFC 改訂まで公開しない）

```text
min / max
narrow / widen / saturating
float↔int convert
shift（count mask が ISA 依存）
horizontal reduction
swizzle / shuffle
relaxed SIMD（std::wasm のみ）
```

### 4. 整数規則

```text
add / sub / mul: wrapping（2^width modulo）
overflow で trap / saturate しない
```

shift は初期核外。

### 5. 浮動小数規則

```text
非 NaN の有限値・±∞・符号付きゼロ: scalar f32/f64 規則と一致
NaN であること: 保存する
NaN payload の一致: 保証しない（演算結果 NaN を to_bits で観測してよい）
from_bits / to_bits / bitwise（整数経由）: 入力ビットを正確に保存
PartialEq（全 lane bool）: 各 lane の scalar == の積
```

### 6. select / bit_select

```text
select(mask, a, b):
  mask lane true  → a の lane
  mask lane false → b の lane

bit_select(a, b, mask_bits): 整数 SIMD のみ
  (a & mask_bits) | (b & ~mask_bits)
```

`Mask4` を受け取るのは lane `select`。`bit_select` は `I32x4` mask_bits。

### 7. 同値性検証（stable 昇格）

各初期核 op について三者比較:

```text
1. reference scalar evaluator（lane ごとの scalar 意味）
2. Scalar lowering backend
3. NativeSimd backend
```

差分があれば portable 契約違反。NaN payload 以外の不一致は fail。

---

## 非目標

- #698 全 lane 型・全 op の一括 portable 契約化
- raw `V128` の意味論（Wasm 仕様）

---

## 関連

- ADR-037 / ADR-015
