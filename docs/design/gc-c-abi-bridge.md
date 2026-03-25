# GC ⇔ C ABI 境界設計

Wasm GC 参照と C ポインタの変換規則を定義する。

---

## 問題

ADR-006 は 3 層 ABI を定義:
- Layer 1: 内部 ABI
- Layer 2: Wasm 公開 ABI（GC 参照）
- Layer 3: native 公開 ABI（C ポインタ）

Layer 2 と Layer 3 の接続点が未定義:
- GC 参照を C に渡す方法
- C ポインタを GC に取り込む方法
- 寿命管理
- pinning

---

## 設計方針

**v0 では GC ⇔ C の直接変換を提供しない。**

理由:
- 安全な変換規則の設計が困難
- pinning の実装が複雑
- v0 の優先度は Wasm 動作

**代わりに、linear memory を介したデータ交換のみ提供。**

---

## データ交換パターン

### パターン 1: 値のコピー（推奨）

GC ヒープのデータを linear memory にコピーして渡す。

**重要**: `mem.alloc` / `mem.free` は **std 内部専用**。ユーザーコードからは呼べない。

```
// ❌ ユーザーコードではこう書けない
fn call_c_function(s: String) -> i32 {
    let ptr = mem.alloc(s.len())    // コンパイルエラー: 非公開 API
    mem.free(ptr)                   // コンパイルエラー: 非公開 API
}

// ✅ std が提供する安全な FFI ラッパーを使う
// （v0 では WASI import のみ、ユーザー定義 FFI は v1 以降）
```

std 内部での実装（参考）：
```
// std/internal/ffi.ark（非公開モジュール）
fn __ffi_string_to_linear(s: String) -> (i32, i32) {
    let ptr = mem.__alloc(len(s))   // 内部 API
    mem.__copy_from_string(ptr, s)
    (ptr, len(s))
}

fn __ffi_linear_cleanup(ptr: i32) {
    mem.__free(ptr)                 // 内部 API
}
```

**設計根拠**: `mem.alloc` / `mem.free` をユーザーに露出すると：
- free 忘れ → リーク
- 二重 free → UB
- サイズミス → UB

これは LLM フレンドリ設計に真っ向から反する。
v0 では WASI の FFI 変換はすべて std が内部で行う。
```

```c
// C 側
int extern_c_function(const char* ptr, size_t len) {
    // ptr は linear memory 上のアドレス
    // 直接読める
}
```

### パターン 2: ハンドル経由

GC オブジェクトに「ハンドル」を発行し、ハンドル経由で操作。

```
// arukellt 側で handle table を管理
global handle_table: Vec[ref object]

fn export_handle(obj: ref object) -> i32 {
    let handle = handle_table.len()
    handle_table.push(obj)
    handle
}

fn import_handle(handle: i32) -> ref object {
    handle_table.get(handle).unwrap()
}
```

C 側はハンドル（整数）だけを保持。GC オブジェクトへの直接アクセスは不可。

---

## 禁止事項

### 1. GC 参照の直接エクスポート

```
// NG: GC 参照を直接 C に渡す
extern "C" fn bad_export() -> ref String {
    String::from("hello")
}
```

理由: GC が参照を移動する可能性。C 側のポインタが無効になる。

### 2. C ポインタの GC 参照化

```
// NG: C のポインタを GC 参照として扱う
extern "C" fn bad_import(ptr: *const u8) -> ref object {
    unsafe { transmute(ptr) }
}
```

理由: GC が管理していないメモリを参照することになる。

---

## 型の変換規則

| arukellt 型 | C 型 | 変換方法 |
|------------|------|---------|
| `i32` | `int32_t` | そのまま |
| `i64` | `int64_t` | そのまま |
| `f32` | `float` | そのまま |
| `f64` | `double` | そのまま |
| `bool` | `int32_t` | 0/1 |
| `String` | `(ptr, len)` | linear memory にコピー |
| `Vec[T]` | `(ptr, len)` | linear memory にコピー |
| `struct` | `(ptr)` | linear memory にコピー |
| `enum` | `(tag, data)` | linear memory にコピー |
| `Option[T]` | `(has_value, value)` | 展開 |
| `Result[T, E]` | `(is_ok, ok_or_err)` | 展開 |

---

## FFI 宣言構文（v1 以降）

```
// 将来の構文案
extern "C" {
    fn strlen(s: *const u8) -> usize;
    fn memcpy(dst: *mut u8, src: *const u8, n: usize);
}

// 呼び出し
let len = strlen(s.as_ptr())
```

v0 では FFI は WASI import のみ。ユーザー定義 FFI は v1 以降。

---

## WASI との関係

v0 は WASI p1 / p2 を併用する。WASI p1 は linear memory ベースのため、GC との変換が必要:

```
// std/internal/wasi.ark（非公開モジュール — ユーザーコードには見えない）
fn __wasi_print(s: String) {
    // 1. String を linear memory にコピー
    let buf_ptr = mem.__alloc(len(s))
    mem.__copy_from_string(buf_ptr, s)
    
    // 2. iovec を構築
    let iovec_ptr = mem.__alloc(8)
    mem.__store_i32(iovec_ptr, buf_ptr)
    mem.__store_i32(iovec_ptr + 4, len(s))
    
    // 3. fd_write 呼び出し
    let nwritten_ptr = mem.__alloc(4)
    wasi.fd_write(1, iovec_ptr, 1, nwritten_ptr)
    
    // 4. 解放（すべて std 内部で完結）
    mem.__free(buf_ptr)
    mem.__free(iovec_ptr)
    mem.__free(nwritten_ptr)
}
```

**ユーザーが呼ぶ API**:
```
// ユーザーコード
fn main(caps: Capabilities) -> Result<(), IOError> {
    let stdout_h = stdout(caps)
    stdout_write(stdout_h, "Hello\n")?  // std が内部で WASI 変換
    Ok(())
}
```

この変換は std が内部で行う。ユーザーは `mem.__alloc` / `mem.__free` に一切触れない。

WASI p2（Component Model / WIT）経由の公開面では canonical ABI の lower/lift を使うが、言語の意味論は p1 面と分岐させない。

---

## Pinning（将来）

v1 以降で検討:
- GC に「このオブジェクトを移動しない」と指示
- pinning 中は C 側から直接アクセス可能
- pin scope を抜けたら移動可能に戻る

```
// 将来の構文案
pin s {
    // この中では s は移動しない
    c_function(s.as_ptr())
}
```

---

## v0 での制限まとめ

| 機能 | v0 | v1 以降 |
|------|----|----|
| Wasm ↔ Wasm (GC) | ✅ | ✅ |
| arukellt → WASI | ✅ (std 内部で変換) | ✅ |
| arukellt → C (linear memory コピー) | ✅ | ✅ |
| arukellt → C (ハンドル) | ⚠️ 手動実装 | ✅ 組み込み |
| arukellt → C (直接参照) | ❌ | ⚠️ pinning |
| C → arukellt | ⚠️ 値渡しのみ | ✅ |

---

## 関連

- ADR-005: LLVM IR の役割制限
- ADR-006: 公開 ABI
- `docs/language/memory-model.md`: GC 型の表現
