---
Status: open
Created: 2026-03-28
Updated: 2026-04-22
ID: 121
Track: wasi-feature
Depends on: 510
Orchestration class: implementation-ready
Orchestration upstream: #074-parent-gate
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
---

# WASI P2: Canonical ABI ハンドリングの堅牢化
**Blocks v4 exit**: no

**Status note**: Leaf close-gate issue for #074. Minimum Canonical ABI support required by P2 validate/run must land before #074 closes.

## Reopened by audit

- **Date**: 2026-04-21
- **Reason**: Canonical ABI lift/lower completeness is an active product gap for WASI P2 native/component work. Existing open issues (#074, #124, #510) depend on this but no dedicated active issue tracks the close gate.
- **Audit evidence**:
  - No dedicated active open issue tracked this product gap.
  - The capability is required for the WASI P2 / component product surface, not merely future speculation.
  - Reject placement was inconsistent with current product direction.

## Summary

WASI P2 の Component Model では、Canonical ABI (Lift/Lower 規則) が
全てのインターフェース呼び出しの型変換を定義する。
現在の `ark-wasm/src/component/canonical_abi.rs` の Lift/Lower 実装を
`docs/spec/spec-WASI-0.2.10/OVERVIEW.md` の WIT 型規則に照合して完全性を検証・修正する。

## 受け入れ条件

1. WIT の全型 (`bool`, `u8`〜`u64`, `s8`〜`s64`, `f32`, `f64`, `char`, `string`,
   `list<T>`, `record`, `variant`, `enum`, `option<T>`, `result<T,E>`, `tuple`, `resource`) の
   Lift/Lower が `canonical_abi.rs` に実装されていることを確認
2. 各型についてラウンドトリップテスト (Lower → Lift で元の値に戻ること)
3. 未実装型のパニックを適切なエラーに変換

## Parent gate relationship — 2026-04-22

\#121 is a close-gate leaf for #074, not a downstream feature that waits for #074
to close. For #074 closure, this issue must at minimum provide the Canonical ABI
surface used by the P2 native smoke path: strings/lists for stdio and host-call
arguments, plus the resource-handle behavior needed by the first host capability
fixture selected for the gate. Broader WIT type completeness remains this issue's
full acceptance target.

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md` §WIT形式の読み方
- `crates/ark-wasm/src/component/canonical_abi.rs`
