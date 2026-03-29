# 165: Phase 3 — Wasm Emitter の Arukellt 実装

**Version**: v5 Phase 3
**Priority**: P1
**Depends on**: #164 (Resolver + TypeChecker)

## 概要

Arukellt で書かれた Wasm バイナリエミッターを実装する。MIR から Wasm バイナリを生成し、stdout に出力する。

## タスク

1. `src/compiler/emitter.ark`: Wasm バイナリ生成
   - Wasm バイナリフォーマット: magic number, version, sections
   - LEB128 エンコード (符号付き/符号なし)
   - Type section, Import section, Function section, Memory section, Export section, Code section, Data section
   - GC 拡張: struct.new, array.new, ref.cast, br_on_cast
2. `src/compiler/wasm_types.ark`: Wasm 型定義ヘルパー
   - ValType, FuncType, StructType, ArrayType
3. バイナリ出力: `Vec<i32>` (バイト列) として構築し `fd_write(1, bytes, len)` で stdout に書き出す
4. wasmparser による生成 Wasm の validation (外部コマンド or inline)
5. T1 (wasm32-wasi-p1) と T3 (wasm-gc-p2) の両方の emit モード

## 完了条件

- `arukellt compile src/compiler/*.ark -o arukellt-s1.wasm` が成功する
- `arukellt-s1.wasm` が wasmparser で valid
- `arukellt-s1.wasm` で 全 fixture test を pass する
- Stage 1 → Stage 2 の fixpoint が達成される (sha256 一致)

## 注意事項

- Wasm バイナリの関数インデックス順序を決定的にする (fixpoint のため)
- バイナリ生成は最も低レベルなコンポーネント。LEB128 のオフバイワンエラーに注意
- GC 拡張命令のエンコーディングは wasm-gc spec を厳密に参照する
