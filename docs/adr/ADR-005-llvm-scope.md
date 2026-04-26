# ADR-005: LLVM IR バックエンドの役割制限

ステータス: **DECIDED** — LLVM IRバックエンドはWasm意味論に従属

決定日: 2026-03-24

---

## 文脈

arukellt は WASM32 を主ターゲットとするが、native 実行のために LLVM IR バックエンドも提供する。

LLVM IR 側で「より良い最適化」を追求すると、Wasm と native で意味論が乖離するリスクがある。

---

## 決定

**LLVM IR バックエンドは Wasm 意味論に従属する**

### 原則

1. **Wasm 意味論が正**
   - 言語の意味論は Wasm 側で定義
   - LLVM IR は Wasm と同じ動作を再現するだけ

2. **未最適化でよい**
   - LLVM の最適化パスに頼りすぎない
   - Wasm emitter と同じ MIR から生成

3. **native 専用機能は追加しない**
   - SIMD は Wasm SIMD を優先
   - スレッドは Wasm Threads を優先
   - native 専用の言語機能は入れない

### 役割

| バックエンド | 役割 |
|-------------|------|
| Wasm Emitter | **主**。言語の意味論を定義 |
| LLVM IR Emitter | **従**。native 実行用。デバッグ用 |

### 使用シーン

LLVM IR バックエンドの正当な使用:

- ローカルでのデバッグ（gdb/lldb との連携）
- 性能比較（Wasm ランタイム vs native）
- Wasm 未対応環境での実行

---

## 制約事項

### LLVM で追加してはいけないもの

- Wasm にない最適化（Wasm の動作と乖離する可能性）
- native 専用の言語機能
- LLVM 依存の型表現

### FFI の扱い

```
// extern "C" は LLVM バックエンドでのみ有効
extern "C" fn my_c_function(x: i32) -> i32

// Wasm では WASI import として解決
```

native FFI は C ABI のみサポート。arukellt 独自の ABI 拡張は行わない。

---

## 関連

- ADR-006: 公開 ABI（3 層構造）
- `docs/compiler/pipeline.md`: MIR からの分岐
- `docs/platform/abi.md`: native ABI 詳細
