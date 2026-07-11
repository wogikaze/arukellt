# RFC-003: Portable SIMD operation semantics（NativeSimd ↔ Scalar）

ステータス: **DRAFT**  
関連: [ADR-037](../adr/ADR-037-std-simd.md)、[ADR-042](../adr/ADR-042-intrinsic-layer-separation.md)  
提案日: 2026-07-11

---

## 目的

ADR-037 の `portable_simd_lowering: NativeSimd | Scalar` について、
**どの操作が portable 契約に入り、NativeSimd と Scalar で何をもって同値とするか**を固定する。

本 RFC が採択されるまで、ADR-037 の stable 昇格条件にある
「NativeSimd / Scalar 同値」は初期核（下記）に限定して解釈する。

---

## 決定（提案）

### 1. 意味論の基準

portable `std::simd` 操作の意味は、**各 lane に Arukellt scalar 演算を適用した結果**を正とする。

- Scalar lowering は、その定義をそのまま実行する。
- NativeSimd（Wasm SIMD 命令等）は、その結果と観測可能に一致しなければならない。
- Wasm 命令の癖を言語仕様の正本にしない（命令は lowering の候補）。

例外（初期核では扱わない／別節）:

- relaxed SIMD → `std::wasm` のみ（portable 契約外）
- float `min`/`max` の NaN 規則、narrowing/widening、saturating convert、
  shift count mask、float→int trap/sat → **初期核外**。追加するなら本 RFC を改訂してから。

### 2. 初期核（ADR-037 実装核と一致）

次のみが portable 同値契約の対象:

```text
splat / lane get-set
integer add / sub / mul（wrapping、scalar i32/i64 と同じ）
bitwise and / or / xor / not
cmp_eq / cmp_ne / cmp_lt / … → MaskN（lane ごとの scalar 比較）
select(MaskN, a, b)
bit_select（整数、式は ADR-037）
MaskN::any / all / bitmask
同幅 from_bits / reinterpret（bit パターン保存）
```

初期核に **含めない**（追加は本 RFC 改訂が必要）:

```text
f32/f64 min/max、div、sqrt、ceil/floor
narrowing / widening / saturating
shift（count mask 規則が ISA 依存になりやすい）
float↔int convert（trap vs sat）
shuffle / swizzle（将来）
relaxed SIMD
```

浮動小数の `==`（`PartialEq` / 全 lane bool）は scalar `==` の lane 積。
lane 順序比較は `cmp_*` のみ（演算子 `<` は ADR-037 で禁止）。

### 3. 検証

各初期核 op について:

1. Scalar reference（Ark またはホスト）で期待値を生成
2. NativeSimd emit 結果と比較（differential）
3. NaN / ±0 を含むケースは、初期核に入る op についてのみ必須

---

## 非目標

- 既存 #698 の全 lane 型・全 op を一括で portable 契約化すること
- raw `std::wasm::V128` 操作の意味論（Wasm 仕様に従う）

---

## 関連

- ADR-037 § portable 意味論 / 初期核
- ADR-015（panic しない Scalar 経路）
