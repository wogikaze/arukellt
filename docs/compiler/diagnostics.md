# 診断システム設計

LLMが壊れたコードを修正できる診断出力の仕様。

> **⚠️ 本文書は設計仕様**。現行実装の診断は以下の範囲で動作:
> - ✅ エラーコード付き報告（E0001〜E0304）
> - ✅ ソース位置（ファイル:行:列）表示
> - ✅ スニペット表示（ariadne 使用）
> - ✅ 複数エラーの一括報告
> - ✅ expected/actual 表示（型エラー時）
> - ✅ 警告の表示（コンパイル成功時も出力）
> - 🔲 fix-it hint（設計済み・v1 で実装予定）
> - 🔲 LLM 向け定型文パターン（設計済み・v1 で実装予定）

---

## 設計原則

1. **1エラー1原因**: 連鎖エラーは抑制し、根本原因のみ報告
2. **expected/actual 必須**: 型エラーは必ず期待型と実際型を示す
3. **fix-it 提供**: 機械的に修正可能なエラーには修正案を提示
4. **位置の明確化**: span情報を正確に、複数箇所関連時は全て示す
5. **簡潔な説明**: LLMが解釈しやすい定型文

---

## エラーフォーマット

### 基本形式

```
error[E0001]: type mismatch
  --> src/main.ark:5:10
   |
 5 |     let x: i32 = "hello"
   |                  ^^^^^^^ expected `i32`, found `String`
   |
help: change the type annotation
   |
 5 |     let x: String = "hello"
   |            ^^^^^^
```

### 構成要素

| 要素 | 必須 | 内容 |
|------|------|------|
| エラーコード | ✅ | `E0001` 形式 |
| メッセージ | ✅ | 簡潔な1行説明 |
| 位置 | ✅ | `ファイル:行:列` |
| スニペット | ✅ | 該当行と前後1行 |
| アノテーション | ✅ | `^^^` でマーク |
| expected/actual | 型エラー時 | 必須 |
| help | 可能な場合 | 修正案 |

---

## エラー分類（v0）

### E00xx: 構文エラー

| コード | 説明 | 例 |
|--------|------|-----|
| E0001 | unexpected token | `let x = ;` |
| E0002 | missing token | `fn foo() -> i32` (body なし) |
| E0003 | invalid construct | `for x in vec` (v0禁止) |

**fix-it 例**:
```
error[E0001]: unexpected token
  --> src/main.ark:3:10
   |
 3 |     let x = ;
   |             ^ expected expression
   |
help: add a value
   |
 3 |     let x = 0;
   |             ^
```

### E01xx: 名前解決エラー

| コード | 説明 | 例 |
|--------|------|-----|
| E0100 | unresolved name | `unknown_func()` |
| E0101 | duplicate definition | 同名関数定義 |
| E0102 | private access | `other_module.private_fn()` |

**fix-it 例**:
```
error[E0100]: unresolved name
  --> src/main.ark:8:5
   |
 8 |     unknown_func()
   |     ^^^^^^^^^^^^ not found in this scope
   |
help: did you mean `known_func`?
   |
 8 |     known_func()
   |     ^^^^^^^^^^
```

### E02xx: 型エラー

| コード | 説明 | 例 |
|--------|------|-----|
| E0200 | type mismatch | `let x: i32 = "hello"` |
| E0201 | missing type annotation | `let x = Vec_new_T()` で T が推論不能 |
| E0202 | wrong argument count | `foo(1, 2)` when expects 1 |
| E0203 | invalid generic argument | `Vec[Vec[i32]]` (ネスト禁止) |

**fix-it 例**:
```
error[E0200]: type mismatch
  --> src/main.ark:10:14
   |
10 |     add(x, "5")
   |            ^^^ expected `i32`, found `String`
   |
   = note: function `add` expects: fn(i32, i32) -> i32
help: parse the string to integer
   |
10 |     add(x, parse_int("5"))
   |            +++++++++++  +
```

```
error[E0201]: missing type annotation
  --> src/main.ark:12:9
   |
12 |     let v = Vec_new_T()
   |         ^ cannot infer type `T` in generic constructor
   |
help: use a type-specific constructor
   |
12 |     let v = Vec_new_i32()
   |             ~~~~~~~~~~~~~
```

### E03xx: v0制約違反

| コード | 説明 | 例 |
|--------|------|-----|
| E0300 | trait not available | `impl Display for T` |
| E0301 | method call not available | `v.push(x)` |
| E0302 | nested generic | `Vec[Vec[i32]]` |
| E0303 | ~~for loop not available~~ | **廃止**: `for` ループは v0 で実装済み |
| E0304 | operator overload / `impl` | `impl Add for T` — **✅ 実装済み** |

**fix-it 例**:
```
error[E0301]: method call syntax not available in v0
  --> src/main.ark:15:7
   |
15 |     v.push(42)
   |       ^^^^ v0 does not support method syntax
   |
help: use function call syntax instead
   |

### W0xxx: 警告

| コード | 説明 | 例 |
|--------|------|-----|
| W0001 | unintended sharing | 可変参照型を代入後に両方から変更 — **✅ 実装済み** |

**fix-it 例**:
```
warning[W0001]: possible unintended sharing of mutable reference
  --> src/main.ark:3:9
   |
 2 |     let a = Vec_new_i32()
 3 |     let b = a
   |         ^ `b` aliases `a`; mutations to `b` also affect `a`
 4 |     push(b, 42)
   |     ----------- mutates shared Vec
   |
help: if independent copy is needed, use clone
   |
 3 |     let b = clone(a)
   |             ~~~~~~~~
```
15 |     vec_push(v, 42)
   |     ~~~~~~~~ ~  ~~
```

```
error[E0303]: for loop not available in v0
  --> src/main.ark:20:5
   |
20 |     for x in items {
   |     ^^^ v0 does not support `for` loops
   |
help: use `while` loop with index
   |
20 |     let mut i = 0
21 |     while i < len(items) {
22 |         let x = get(items, i)
23 |         i = i + 1
   |
```

---

## LLM向けエラーパターン

### パターン1: 型推論失敗

**LLMが書きやすいコード**:
```
let v = Vec_new_T()  // T が推論不能
push(v, 42)
```

**診断**:
```
error[E0201]: missing type annotation
  --> src/main.ark:1:9
   |
 1 |     let v = Vec_new_T()
   |         ^ cannot infer generic type parameter
help: use concrete type constructor
   |
 1 |     let v = Vec_new_i32()
   |             ~~~~~~~~~~~~~
```

**修正後**:
```
let v = Vec_new_i32()
push(v, 42)
```

### パターン2: メソッド構文の誤用

**LLMが書きやすいコード**:
```
let s = String::from("hello")
s.push('!')
```

**診断**:
```
error[E0301]: method call not available
  --> src/main.ark:2:3
   |
 2 |     s.push('!')
   |       ^^^^ use function syntax
help: rewrite as function call
   |
 2 |     string_push(s, '!')
   |     ~~~~~~~~~~~ ~  ~~~
```

**修正後**:
```
let s = String::from("hello")
string_push(s, '!')
```

### パターン3: for ループの旧制約（v0 で解消済み）

> **注**: `for` ループは v0 で実装済み。以下は参考として残す。
> `for i in 0..n` と `for item in values(v)` が使用可能。

**v0 で使えるコード**:
```
for item in values(items) {
    println(item)
}

for i in 0..10 {
    println(i)
}
```

### パターン4: Result unwrap 忘れ

**LLMが書きやすいコード**:
```
let content = fs_read_file(dir, path)
print(content)
```

**診断**:
```
error[E0200]: type mismatch
  --> src/main.ark:2:7
   |
 2 |     print(content)
   |           ^^^^^^^ expected `String`, found `Result[String, IOError]`
   |
help: handle the Result with `?` or `unwrap`
   |
 1 |     let content = fs_read_file(dir, path)?
   |                                           +
   or
 1 |     let content = unwrap(fs_read_file(dir, path))
   |                   +++++++                       +
```

**修正後**:
```
let content = fs_read_file(dir, path)?
print(content)
```

### パターン5: ネストしたジェネリクス

**LLMが書きやすいコード**:
```
let matrix: Vec<Vec<i32>> = Vec_new_Vec()  // ネスト禁止
```

**診断**:
```
error[E0302]: nested generic not allowed
  --> src/main.ark:1:17
   |
 1 |     let matrix: Vec<Vec<i32>> = Vec_new_Vec()
   |                     ^^^^^^^^^ v0 prohibits nested generics
   |
help: flatten to single Vec with manual indexing
   |
   |     let matrix: Vec<i32> = Vec_new_i32()
   |     let rows = 10
   |     let cols = 10
   |     // access: get(matrix, row * cols + col)
```

**修正後**:
```
let matrix: Vec<i32> = Vec_new_i32()
let rows = 10
let cols = 10
// access: get(matrix, row * cols + col)
```

---

## エラー抑制戦略

### 連鎖エラーの抑制

型エラーが発生した変数は `ErrorType` として扱い、後続エラーを抑制:

```
let x: i32 = "hello"  // E0200
let y = x + 1         // エラー報告しない（x は ErrorType）
```

### 複数定義時の報告

最初のエラーのみ報告:

```
fn foo() { }  // OK
fn foo() { }  // E0101 報告
fn foo() { }  // 報告しない
```

---

## 実装ガイドライン

### 型チェッカー

```rust
fn check_expr(&mut self, expr: &Expr, expected: Option<Type>) -> Type {
    match expr {
        Expr::IntLit(_) => Type::I32,
        Expr::Call(func, args) => {
            let func_ty = self.check_expr(func, None);
            match func_ty {
                Type::Fn(params, ret) => {
                    if args.len() != params.len() {
                        self.error(E0202, "wrong argument count", expr.span);
                        return Type::Error;
                    }
                    for (arg, param) in args.iter().zip(params.iter()) {
                        let arg_ty = self.check_expr(arg, Some(param.clone()));
                        if !self.unify(&arg_ty, param) {
                            self.error_with_help(
                                E0200,
                                format!("expected `{}`, found `{}`", param, arg_ty),
                                arg.span,
                                "check the function signature"
                            );
                        }
                    }
                    ret
                }
                Type::Error => Type::Error, // 連鎖抑制
                _ => {
                    self.error(E0200, "not a function", func.span);
                    Type::Error
                }
            }
        }
        _ => todo!()
    }
}
```

---

## テストケース

各エラーコードに対して:
- 最小再現コード
- 期待される診断出力
- 修正後コード

例: `tests/diagnostics/E0301_method_call.ark`

---

## 関連

- `docs/compiler/pipeline.md`: 型チェッカーの位置
- `docs/language/error-handling.md`: エラー処理の言語機能
- `docs/process/llm-readiness-plan.md`: LLM対応計画
