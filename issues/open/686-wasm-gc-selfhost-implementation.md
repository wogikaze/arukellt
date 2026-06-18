# Wasm GC Selfhost Implementation

- Track: `gc-native`, `compiler`
- Status: **open**
- Depends on: ADR-035

## Summary

Implement Wasm GC (`struct.new`, `array.new`, `ref.cast`, `br_on_cast`) in the
selfhost compiler. ADR-002 chose GC-native in principle (2026-03-25), and the
Rust prototype proved feasibility (542 tests). The selfhost emitter instead
uses linear memory for all targets. This issue tracks the GC rollout per
ADR-035's phased plan.

## Sub-issues / Phases

### Phase 1: Value Representation GC-化 (`035-gc-value-representation.md`)

- [ ] MIR type system に GC reference type を追加 (`value_types.ark`, `MirLocal`)
- [x] sig_to_wasm_type で GC type を出力 (reference type encoding)
- [x] struct.new/struct.get/struct.set の Wasm GC 命令出力
- [x] array.new/array.get/array.set の Wasm GC 命令出力
- [x] 関数シグニチャの GC reference type 対応 (Wasm バリデーション通過)

2026-06-18: selfhost emitter now emits GC reference type encodings and
GC struct/array instructions for the current `i32` aggregate lowering shape.
`wasm32-wasi-p2` outputs for `tests/fixtures/arrays/array_literal.ark` and
`tests/fixtures/structs/basic_struct.ark` validate with `wasm-tools --features gc`.
The broader MIR type-system item remains open because reference semantics are
still inferred at emit time rather than represented as a complete MIR type model.

### Phase 2: 文字列 GC 表現 (`035-gc-strings.md`)

- [ ] String の GC 表現: `(ref null (array (mut i8)))`
- [ ] concat/substring/char_at の GC 配列操作への移行
- [ ] linear memory 上の length-prefixed 文字列からの移行

### Phase 3: Vec/Enum/Struct GC 表現 (`035-gc-vec-enum-struct.md`)

- [ ] Vec<T> の GC struct + GC array backing
- [ ] Enum subtype hierarchy + br_on_cast dispatch
- [ ] HashMap GC 表現
- [ ] i31ref boxing for small integers in generics

### Phase 4: 検証・最適化 (`035-gc-verification.md`)

- [ ] `--target wasm32-wasi-p2` で全フィクスチャ通過
- [ ] T1 linear memory パス維持確認
- [ ] gc_hint custom section 充実
- [ ] Benchmark 比較 (T1 linear vs T3 GC)

## 関連

- ADR-035: Wasm GC Implementation Plan
- Done: #005-#025 (Rust prototype GC issues)
- Depends on: #036/#037 (jco GC support, external)
