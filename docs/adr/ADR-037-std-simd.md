# ADR-037: std::simd — Explicit SIMD Library API

ステータス: **PROPOSED** — portable `Simd<T,N>` API と raw `std::wasm::V128` の分離導入を提案

提案日: 2026-06-26  
改訂日: 2026-07-11 — ADR-042 と整合（bitselect/reinterpret は portable）。型・fallback 境界を定義

---

## 文脈

Arukellt には SIMD サポートが存在しない。`std::wasm::valtype_v128` の定数バイトのみが
experimental で存在する。本 ADR は明示的 SIMD ライブラリ API としての導入入口を定義する。

Issue #107 (ループベクトル化ヒント) は reject 済みであり、本 ADR はその代替位置づけである
（hint ではなく explicit library API）。

### 関連 ADR

- ADR-002: Wasm GC 前提 — SIMD 値は GC struct/array フィールドに保持可能（portable 表現）
- ADR-006: 公開 ABI 境界 — SIMD の stable 公開は WIT/canonical。raw V128 は非 stable
- ADR-007 / ADR-013: ターゲットと capabilities
- ADR-014: 初期 stability は experimental
- **ADR-042**: portable semantic op vs `std::wasm` target intrinsic（本 ADR はこれに従う）
- ADR-045: native SIMD はスコープ外

### Wasm 3.0 仕様上の前提

Wasm の `v128` は `valtype` の一種であり、GC struct/array の field/element にも入りうる。
ただし **言語の portable SIMD 型と Wasm raw `v128` は同一視しない**（下記決定）。

---

## 提案する決定

### 1. 二層の型モデル（必須分離）

| 層 | 型 | 役割 | `simd128` ScalarEmulation | Component/WIT 公開 |
|----|-----|------|---------------------------|-------------------|
| **Semantic** | `Simd<T, N>` | portable ベクトル意味 | **可能**（scalar tuple 等） | 将来は canonical 投影を別途設計 |
| **Raw Wasm** | `std::wasm::V128` | Wasm 128-bit value category の露出 | **不可**（capability 必須） | **不可** |

- `f32x4` / `i32x4` 等は **`Simd<T,N>` の nominal wrapper（または薄い alias）** とする。
  すべてが同じ raw `v128` の表示違いではない。
- 同幅 lane 型間の bit reinterpret は portable な型変換 API（`Simd` 上）とし、
  `std::wasm` に置かない（ADR-042 D8）。
- facade としての `Vec<i32>` 擬似 SIMD は採用しない。scalar lowering は
  `Simd<T,N>` の正規表現の一つであり、raw `V128` の emulation ではない。

### 2. メモリモデル

- portable `Simd<T,N>` 値は言語の値意味論に従う。GC struct/array フィールドに保持できる。
- `std::wasm::V128` は Wasm 値型の露出であり、linear-memory load/store および
  Wasm 固有操作に使う。GC field に載せる場合も raw capability が必要。

### 3. 初期 API 面（仕様上の最小核）

**実装の最初の検証核**（同時に全部を実装しなくてよい）:

```text
Simd<i32, 4> / i32x4
Simd<f32, 4> / f32x4
mask / splat / lane access
arithmetic / compare / select（bitselect 含む）
```

**仕様カタログとして後続で定義してよい lane 型**（初回実装必須ではない）:

```text
i8x16 u8x16 i16x8 u16x8 u32x4 i64x2 u64x2 f64x2
```

「仕様上すべての lane 型を列挙する」と「初回実装で全 lane を同時出荷する」は分ける。

### 4. ターゲット適用範囲と capabilities

SIMD 可否は **target × target feature** で決まる。

```text
TargetCapabilities {
    simd128: Enabled | ScalarEmulation | Unsupported
    relaxed_simd: Enabled | Unsupported
}
```

| ターゲット | fixed SIMD (`simd128`) | relaxed SIMD |
|------------|------------------------|--------------|
| `wasm32` / `wasm32-gc` | Enabled→直接 emit / ScalarEmulation→`Simd` のみ scalar / Unsupported→エラー | `std::wasm` で別判定 |
| `native-*` | スコープ外（ADR-045） | — |

- portable `std::simd` 操作は ScalarEmulation で同値計算してよい（ADR-015: panic しない）。
- `std::wasm::V128` および raw load/store / relaxed は ScalarEmulation **不可**。

### 5. 機能検出

実行時検出は行わない。コンパイル時の `TargetCapabilities` で決める。

### 6. 名前空間と所属（ADR-042 と一致）

```text
std::simd:
  and / or / xor / not
  bitselect
  any / all / bitmask
  同幅 lane 型間の bit reinterpret
  fixed SIMD arithmetic / comparison / shuffle 等の portable 操作

std::wasm:
  linear-memory v128.load / store（および load_splat / load_lane 等）
  relaxed SIMD
  Wasm 固有 trap・lane 規則を直接露出する raw 操作
  std::wasm::V128 / valtype_v128（encoder/decoder・raw 境界）
```

`std::simd::bitselect` の backend が Wasm の `v128.bitselect` を選ぶのは正しい lowering であり、
**公開 API の所属を Wasm 命令名に合わせる必要はない**。

### 7. メモリアクセス

1. `std::simd` は明示的な linear-memory load/store API を持たない。
2. GC / 言語値としての読み書きは field/index access に統合する。
3. Wasm `v128.load` / `v128.store` は `std::wasm` のみ（`LinearPtr` / `LinearSlice`）。
4. GC Vec から raw pointer を取り出す API は提供しない。

### 8. バックエンド

必要な範囲で SIMD opcode / locals / MIR を追加する。raw `V128` と portable `Simd` の
lowering 経路を混線させない。

### 9. native backends

スコープ外（ADR-045 後継）。

### 10. stability

初期は `experimental`（ADR-014）。stable 昇格条件:

1. portable API（型・演算・mask）が破壊的変更なしで固定
2. `+simd128` / ScalarEmulation で `Simd` の意味が一致
3. `std::wasm::V128` との境界が確定（本 ADR §1 / §6）
4. conformance + lowering differential tests

### 11. Issue #107

hint-based autovectorization は採用しない。将来再評価する場合も内部は `Simd<T,N>` MIR へ正規化する。

### 12. `valtype_v128`

既存定数は `std::wasm` に残し、portable `std::simd` へ混ぜない。

---

## 禁止事項

1. `std::simd` に `v128.load` / `v128.store` を混ぜない
2. portable `bitselect` / `reinterpret` / bitwise を `std::wasm` に置かない（ADR-042）
3. raw `V128` を ScalarEmulation しない
4. raw `V128` を Component/WIT の stable 公開面に載せない
5. 全 lane 型の同時実装を初回必須にしない

---

## 結果

- portable SIMD と Wasm raw value が型・capability・名前空間で分離される
- ADR-042 の intrinsic 境界と矛盾しない
- 初期実装核（i32x4 / f32x4）で型モデルを検証できる

## 関連

- [ADR-042](ADR-042-intrinsic-layer-separation.md)
- ADR-002 / ADR-006 / ADR-007 / ADR-014 / ADR-045
- `issues/reject/107-runtime-loop-vectorization-hint.md`
- [WebAssembly 3.0 Spec — Types](https://webassembly.github.io/spec/core/syntax/types.html)
