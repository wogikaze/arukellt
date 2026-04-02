# Core API Cookbook

> **Current-first**: 実装の現在地は [../current-state.md](../current-state.md) を参照してください。
> API リファレンスは [reference.md](reference.md) を参照してください。

このページは、現行実装でそのまま使いやすい書き方だけを残した cookbook です。
古い capability API や未確認 helper は削っています。
各レシピは対応するテスト fixture へのリンクを含んでおり、実際に動くコードの出典として参照できます。

## 基本方針

1. 関数呼び出し形式を基準にする
2. `get` / `pop` の戻り値は `Option<T>` として扱う
3. `Result<T, String>` は `match` または `?` で処理する
4. v1 機能があっても、まずは Prelude ベースの書き方を優先する
5. 文字列化は primitive helper より `to_string(x)` を優先する

---

## Collections — Vec

### 作成と追加

> 📎 Fixture: [`tests/fixtures/stdlib_vec/vec_new.ark`](../../tests/fixtures/stdlib_vec/vec_new.ark),
> [`tests/fixtures/stdlib_vec/vec_push.ark`](../../tests/fixtures/stdlib_vec/vec_push.ark)

```ark
use std::host::stdio
fn main() {
    let mut v: Vec<i32> = Vec_new_i32()
    push(v, 10)
    push(v, 20)
    push(v, 30)
    stdio::println(i32_to_string(len(v)))   // 3
}
```

### 安全な取得 — get は Option を返す

> 📎 Fixture: [`tests/fixtures/stdlib_vec/vec_get.ark`](../../tests/fixtures/stdlib_vec/vec_get.ark)

```ark
use std::host::stdio
fn main() {
    let mut v: Vec<i32> = Vec_new_i32()
    push(v, 10)
    push(v, 20)
    match get(v, 1) {
        Some(val) => stdio::println(i32_to_string(val)),
        None => stdio::println("out of bounds"),
    }
    match get(v, 5) {
        Some(val) => stdio::println(i32_to_string(val)),
        None => stdio::println("out of bounds"),
    }
}
```

### pop — 末尾から取り出す

> 📎 Fixture: [`tests/fixtures/stdlib_vec/vec_pop.ark`](../../tests/fixtures/stdlib_vec/vec_pop.ark)

```ark
use std::host::stdio
fn main() {
    let mut v: Vec<i32> = Vec_new_i32()
    push(v, 10)
    push(v, 20)
    let last: Option<i32> = pop(v)
    match last {
        Some(val) => stdio::println(i32_to_string(val)),  // 20
        None => stdio::println("empty"),
    }
    stdio::println(i32_to_string(len(v)))  // 1
}
```

### 安全でない取得 — get_unchecked

```ark
use std::host::stdio
fn main() {
    let mut v: Vec<i32> = Vec_new_i32()
    push(v, 42)
    let x: i32 = get_unchecked(v, 0)
    stdio::println(to_string(x))
}
```

### map / filter / fold

> 📎 Fixtures: [`tests/fixtures/stdlib_vec/vec_map.ark`](../../tests/fixtures/stdlib_vec/vec_map.ark),
> [`tests/fixtures/stdlib_vec/vec_filter.ark`](../../tests/fixtures/stdlib_vec/vec_filter.ark),
> [`tests/fixtures/stdlib_vec/vec_fold.ark`](../../tests/fixtures/stdlib_vec/vec_fold.ark)

```ark
use std::host::stdio
fn double(x: i32) -> i32 { x * 2 }
fn is_even(x: i32) -> bool { x % 2 == 0 }
fn add(acc: i32, x: i32) -> i32 { acc + x }

fn main() {
    let mut v: Vec<i32> = Vec_new_i32()
    push(v, 1)
    push(v, 2)
    push(v, 3)
    push(v, 4)
    push(v, 5)

    let doubled: Vec<i32> = map_i32_i32(v, double)
    let evens: Vec<i32> = filter_i32(doubled, is_even)
    let total: i32 = fold_i32_i32(evens, 0, add)
    stdio::println(i32_to_string(total))  // 12
}
```

### sort / find / any / contains / reverse

> 📎 Fixtures: [`tests/fixtures/stdlib_vec/vec_sort.ark`](../../tests/fixtures/stdlib_vec/vec_sort.ark),
> [`tests/fixtures/stdlib_vec/any_find.ark`](../../tests/fixtures/stdlib_vec/any_find.ark),
> [`tests/fixtures/stdlib_vec_ops/contains_i32.ark`](../../tests/fixtures/stdlib_vec_ops/contains_i32.ark),
> [`tests/fixtures/stdlib_vec_ops/reverse_i32.ark`](../../tests/fixtures/stdlib_vec_ops/reverse_i32.ark)

```ark
use std::host::stdio
fn is_even(x: i32) -> bool { x % 2 == 0 }

fn main() {
    let mut v: Vec<i32> = Vec_new_i32()
    push(v, 30)
    push(v, 10)
    push(v, 20)

    sort_i32(v)                          // [10, 20, 30]
    reverse_i32(v)                       // [30, 20, 10]

    let has_20: bool = contains_i32(v, 20)   // true
    let has_even: bool = any_i32(v, is_even) // true

    let found = find_i32(v, is_even)
    match found {
        Some(x) => stdio::println(i32_to_string(x)),
        None => stdio::println("none"),
    }
}
```

### sum / remove

> 📎 Fixtures: [`tests/fixtures/stdlib_vec_ops/sum_i32.ark`](../../tests/fixtures/stdlib_vec_ops/sum_i32.ark),
> [`tests/fixtures/stdlib_vec_ops/remove_i32.ark`](../../tests/fixtures/stdlib_vec_ops/remove_i32.ark)

```ark
use std::host::stdio
fn main() {
    let mut v: Vec<i32> = Vec_new_i32()
    push(v, 10)
    push(v, 20)
    push(v, 30)
    push(v, 40)

    stdio::println(i32_to_string(sum_i32(v)))  // 100

    remove_i32(v, 1)                           // removes element at index 1
    stdio::println(i32_to_string(len(v)))      // 3
}
```

---

## Collections — HashMap

> 📎 Fixture: [`tests/fixtures/stdlib_hashmap/hashmap_basic.ark`](../../tests/fixtures/stdlib_hashmap/hashmap_basic.ark)

```ark
use std::host::stdio
fn main() {
    let m = HashMap_i32_i32_new()
    HashMap_i32_i32_insert(m, 1, 100)
    HashMap_i32_i32_insert(m, 2, 200)
    HashMap_i32_i32_insert(m, 3, 300)

    match HashMap_i32_i32_get(m, 2) {
        Some(v) => stdio::println(v),       // 200
        None => stdio::println(-1),
    }

    stdio::println(HashMap_i32_i32_len(m))  // 3

    if HashMap_i32_i32_contains_key(m, 1) {
        stdio::println(String_from("found"))
    }

    match HashMap_i32_i32_get(m, 99) {
        Some(v) => stdio::println(v),
        None => stdio::println(String_from("not found")),
    }
}
```

> **Note**: HashMap API は現在 `i32 → i32` 専用バリアントのみ提供されています。
> 他のキー・バリュー型は将来のリリースで追加予定です。

---

## String Operations

### 作成・空チェック・長さ

> 📎 Fixtures: [`tests/fixtures/stdlib_string/string_new.ark`](../../tests/fixtures/stdlib_string/string_new.ark),
> [`tests/fixtures/stdlib_string/string_len.ark`](../../tests/fixtures/stdlib_string/string_len.ark)

```ark
use std::host::stdio
fn main() {
    let s: String = String_new()
    stdio::println(bool_to_string(is_empty(s)))  // true
    stdio::println(i32_to_string(len(s)))         // 0

    let hello: String = String_from("hello")
    stdio::println(i32_to_string(len(hello)))     // 5
}
```

### 連結と clone

> 📎 Fixtures: [`tests/fixtures/stdlib_string/string_concat.ark`](../../tests/fixtures/stdlib_string/string_concat.ark),
> [`tests/fixtures/stdlib_string/clone.ark`](../../tests/fixtures/stdlib_string/clone.ark)

```ark
use std::host::stdio
fn main() {
    let a: String = String_from("hello")
    let b: String = String_from(" world")
    let c: String = concat(a, b)
    stdio::println(c)           // hello world

    let d: String = clone(a)    // 独立したコピー
    stdio::println(d)
}
```

### 比較 — eq

> 📎 Fixture: [`tests/fixtures/stdlib_string/string_eq.ark`](../../tests/fixtures/stdlib_string/string_eq.ark)

```ark
use std::host::stdio
fn main() {
    let a: String = String_from("test")
    let b: String = String_from("test")
    let c: String = String_from("other")
    stdio::println(bool_to_string(eq(a, b)))  // true
    stdio::println(bool_to_string(eq(a, c)))  // false
}
```

### slice / split / join

> 📎 Fixtures: [`tests/fixtures/stdlib_string/string_slice.ark`](../../tests/fixtures/stdlib_string/string_slice.ark),
> [`tests/fixtures/stdlib_string/string_split.ark`](../../tests/fixtures/stdlib_string/string_split.ark),
> [`tests/fixtures/stdlib_string/string_join.ark`](../../tests/fixtures/stdlib_string/string_join.ark)

```ark
use std::host::stdio
fn main() {
    let s: String = String_from("hello world")
    let sub: String = slice(s, 0, 5)
    stdio::println(sub)  // hello

    let parts: Vec<String> = split(String_from("a,b,c"), String_from(","))
    let joined: String = join(parts, String_from("-"))
    stdio::println(joined)  // a-b-c
}
```

### starts_with / ends_with

> 📎 Fixture: [`tests/fixtures/stdlib_string/string_starts_ends.ark`](../../tests/fixtures/stdlib_string/string_starts_ends.ark)

```ark
use std::host::stdio
fn main() {
    let s: String = String_from("hello world")
    stdio::println(bool_to_string(starts_with(s, String_from("hello"))))  // true
    stdio::println(bool_to_string(ends_with(s, String_from("world"))))    // true
    stdio::println(bool_to_string(starts_with(s, String_from("world"))))  // false
}
```

### to_lower / to_upper / push_char

> 📎 Fixtures: [`tests/fixtures/stdlib_string/to_lower_upper.ark`](../../tests/fixtures/stdlib_string/to_lower_upper.ark),
> [`tests/fixtures/stdlib_string/push_char.ark`](../../tests/fixtures/stdlib_string/push_char.ark)

```ark
use std::host::stdio
fn main() {
    let s = String_from("Hello World")
    stdio::println(to_lower(s))   // hello world
    stdio::println(to_upper(s))   // HELLO WORLD

    let mut greeting = String_from("hello")
    push_char(greeting, '!')
    stdio::println(greeting)      // hello!
}
```

---

## Conversion — 型変換とパース

### to_string — 多態的な文字列化

> 📎 Fixture: [`tests/fixtures/stdlib_io/to_string.ark`](../../tests/fixtures/stdlib_io/to_string.ark)

```ark
use std::host::stdio
fn main() {
    stdio::println(to_string(42))                     // 42
    stdio::println(to_string(9001_i64))               // 9001
    stdio::println(to_string(3.5))                    // 3.5
    stdio::println(to_string(true))                   // true
    stdio::println(to_string('Z'))                    // Z
    stdio::println(to_string(String_from("text")))    // text
}
```

### parse_i32 / parse_i64 / parse_f64 — 文字列をパース

> 📎 Fixtures: [`tests/fixtures/stdlib_io/parse_int.ark`](../../tests/fixtures/stdlib_io/parse_int.ark),
> [`tests/fixtures/stdlib_option_result/question_mark.ark`](../../tests/fixtures/stdlib_option_result/question_mark.ark)

```ark
use std::host::stdio
fn main() {
    let r: Result<i32, String> = parse_i32(String_from("42"))
    match r {
        Ok(val) => stdio::println(i32_to_string(val)),
        Err(e) => stdio::println(e),
    }
}
```

---

## Math

### sqrt — 平方根 (f64)

> 📎 Fixture: [`tests/fixtures/stdlib_math/sqrt.ark`](../../tests/fixtures/stdlib_math/sqrt.ark)

```ark
use std::host::stdio
fn main() {
    let x: f64 = sqrt(9.0)
    stdio::println(f64_to_string(x))   // 3
    let y: f64 = sqrt(16.0)
    stdio::println(f64_to_string(y))   // 4
}
```

### abs / min / max

> 📎 Fixture: [`tests/fixtures/stdlib_math/abs_min_max.ark`](../../tests/fixtures/stdlib_math/abs_min_max.ark)

```ark
use std::host::stdio
fn main() {
    stdio::println(abs(-5))       // 5
    stdio::println(abs(3))        // 3
    stdio::println(min(3, 7))     // 3
    stdio::println(max(3, 7))     // 7
}
```

### clamp_i32 — 範囲内に制限

> 📎 Fixture: [`tests/fixtures/stdlib_math/clamp_i32.ark`](../../tests/fixtures/stdlib_math/clamp_i32.ark)

```ark
use std::host::stdio
fn main() {
    stdio::println(i32_to_string(clamp_i32(5, 0, 10)))   // 5  (in range)
    stdio::println(i32_to_string(clamp_i32(-3, 0, 10)))  // 0  (clamped to min)
    stdio::println(i32_to_string(clamp_i32(15, 0, 10)))  // 10 (clamped to max)
}
```

---

## Option

### 基本 — Some / None / match

> 📎 Fixtures: [`tests/fixtures/stdlib_option_result/some_unwrap.ark`](../../tests/fixtures/stdlib_option_result/some_unwrap.ark),
> [`tests/fixtures/stdlib_option_result/option_match.ark`](../../tests/fixtures/stdlib_option_result/option_match.ark)

```ark
use std::host::stdio
fn describe(opt: Option<i32>) -> String {
    match opt {
        Some(val) => concat(String_from("got "), i32_to_string(val)),
        None => String_from("nothing"),
    }
}

fn main() {
    stdio::println(describe(Some(42)))  // got 42
    stdio::println(describe(None))      // nothing
}
```

### is_some / is_none

```ark
use std::host::stdio
fn main() {
    let x: Option<i32> = Some(21)
    if is_some(x) {
        stdio::println(to_string(unwrap(x)))
    }
    let y: Option<i32> = None
    stdio::println(bool_to_string(is_none(y)))  // true
}
```

### unwrap_or — デフォルト付き展開

> 📎 Fixture: [`tests/fixtures/stdlib_option_result/none_unwrap_or.ark`](../../tests/fixtures/stdlib_option_result/none_unwrap_or.ark)

```ark
use std::host::stdio
fn main() {
    let x: Option<i32> = None
    stdio::println(i32_to_string(unwrap_or(x, 99)))  // 99

    let y: i32 = unwrap_or(get(v, 100), 0)   // bounds-safe default
    stdio::println(to_string(y))
}
```

### map_option — Option の変換

> 📎 Fixture: [`tests/fixtures/stdlib_option_result/option_map.ark`](../../tests/fixtures/stdlib_option_result/option_map.ark)

```ark
use std::host::stdio
fn double(x: i32) -> i32 { x * 2 }

fn main() {
    let a: Option<i32> = Some(21)
    let b: Option<i32> = None
    let mapped_a: Option<i32> = map_option_i32_i32(a, double)
    let mapped_b: Option<i32> = map_option_i32_i32(b, double)
    match mapped_a {
        Some(val) => stdio::println(i32_to_string(val)),  // 42
        None => stdio::println("none"),
    }
    match mapped_b {
        Some(val) => stdio::println(i32_to_string(val)),
        None => stdio::println("none"),                    // none
    }
}
```

---

## Result / Error Handling

### match で処理

> 📎 Fixture: [`tests/fixtures/stdlib_option_result/result_ok_unwrap.ark`](../../tests/fixtures/stdlib_option_result/result_ok_unwrap.ark),
> [`tests/fixtures/stdlib_option_result/result_err_match.ark`](../../tests/fixtures/stdlib_option_result/result_err_match.ark)

```ark
use std::host::stdio
fn main() {
    let r: Result<i32, String> = Err(String_from("bad input"))
    match r {
        Ok(val) => stdio::println(i32_to_string(val)),
        Err(e) => stdio::println(concat(String_from("error: "), e)),
    }
}
```

### `?` で伝播

> 📎 Fixture: [`tests/fixtures/stdlib_option_result/question_mark.ark`](../../tests/fixtures/stdlib_option_result/question_mark.ark)

```ark
use std::host::stdio
fn parse_and_double(s: String) -> Result<i32, String> {
    let val: i32 = parse_i32(s)?
    Ok(val * 2)
}

fn main() {
    match parse_and_double(String_from("21")) {
        Ok(val) => stdio::println(i32_to_string(val)),  // 42
        Err(e) => stdio::println(e),
    }
    match parse_and_double(String_from("abc")) {
        Ok(val) => stdio::println(i32_to_string(val)),
        Err(e) => stdio::println(e),                     // parse error
    }
}
```

### is_ok / is_err / unwrap_or — エラー判定とデフォルト値

> 📎 Fixture: [`tests/fixtures/stdlib_option_result/error_conventions.ark`](../../tests/fixtures/stdlib_option_result/error_conventions.ark)

```ark
use std::host::stdio
fn try_parse(s: String) -> Result<i32, String> {
    if eq(s, "42") {
        Ok(42)
    } else {
        Err(concat("parse error: ", s))
    }
}

fn main() {
    let r1 = try_parse("42")
    stdio::println(bool_to_string(is_ok(r1)))     // true
    stdio::println(bool_to_string(is_err(r1)))     // false
    stdio::println(i32_to_string(unwrap(r1)))      // 42

    let r2 = try_parse("bad")
    stdio::println(bool_to_string(is_ok(r2)))      // false
    stdio::println(i32_to_string(unwrap_or(r2, 99)))  // 99

    // match でエラーメッセージを取り出す
    let r3 = try_parse("xyz")
    let msg = match r3 {
        Ok(v) => concat("got: ", i32_to_string(v)),
        Err(e) => concat("fail: ", e),
    }
    stdio::println(msg)  // fail: parse error: xyz
}
```

---

## I/O — 標準入出力

> ⚠️ **Target constraint**: `std::host::stdio` は **wasm32-wasi** ターゲットが必要です。

### println / eprintln

> 📎 Fixtures: [`tests/fixtures/stdlib_io/print_hello.ark`](../../tests/fixtures/stdlib_io/print_hello.ark),
> [`tests/fixtures/stdlib_io/eprintln.ark`](../../tests/fixtures/stdlib_io/eprintln.ark),
> [`tests/fixtures/stdlib_io/println_multi.ark`](../../tests/fixtures/stdlib_io/println_multi.ark)

```ark
use std::host::stdio
fn main() {
    stdio::println("normal output")
    stdio::eprintln("error message")   // stderr に出力
}
```

---

## I/O — Filesystem

> ⚠️ **Target constraint**: `std::host::fs` は **wasm32-wasi** ターゲットが必要です。

### read_to_string / write_string

> 📎 Fixture: [`tests/fixtures/stdlib_io/fs_read_write.ark`](../../tests/fixtures/stdlib_io/fs_read_write.ark)

```ark
use std::host::fs
use std::host::stdio
fn main() {
    let w: Result<(), String> = fs::write_string("test_output.txt", "hello from arukellt")
    match w {
        Ok(_) => stdio::println(String_from("write ok")),
        Err(e) => stdio::println(e),
    }

    let r: Result<String, String> = fs::read_to_string("test_output.txt")
    match r {
        Ok(content) => stdio::println(content),
        Err(e) => stdio::println(e),
    }
}
```

### エラーハンドリング — 存在しないファイル

> 📎 Fixture: [`tests/fixtures/stdlib_io/fs_read_error.ark`](../../tests/fixtures/stdlib_io/fs_read_error.ark)

```ark
use std::host::fs
use std::host::stdio
fn main() {
    let r: Result<String, String> = fs::read_to_string("nonexistent_file.txt")
    match r {
        Ok(content) => stdio::println(content),
        Err(e) => stdio::println(e),         // エラーメッセージが出力される
    }
}
```

---

## I/O — Environment / Process

> ⚠️ **Target constraint**: `std::host::env` と `std::host::process` は **wasm32-wasi** ターゲットが必要です。

### コマンドライン引数

> 📎 Fixture: [`tests/fixtures/stdlib_env/env_basic.ark`](../../tests/fixtures/stdlib_env/env_basic.ark)

```ark
use std::host::env
use std::host::stdio
fn main() {
    let a = env::args()
    stdio::println(i32_to_string(len(a)))
    stdio::println(i32_to_string(env::arg_count()))
}
```

### 環境変数の取得

> 📎 Fixture: [`tests/fixtures/stdlib_env/env_var_lookup.ark`](../../tests/fixtures/stdlib_env/env_var_lookup.ark)

```ark
use std::host::env
use std::host::stdio
fn main() {
    let v = env::var("PATH")
    match v {
        Some(s) => stdio::println("found PATH"),
        None => stdio::println("PATH not found"),
    }
    let w = env::var("ARUKELLT_NONEXISTENT_XYZ")
    match w {
        Some(s) => stdio::println("unexpected"),
        None => stdio::println("none as expected"),
    }
}
```

### プロセス終了

> 📎 Fixture: [`tests/fixtures/stdlib_process/exit_zero.ark`](../../tests/fixtures/stdlib_process/exit_zero.ark)

```ark
use std::host::process
use std::host::stdio
fn main() {
    stdio::println("before exit")
    process::exit(0)
    // この行以降は実行されない
}
```

---

## I/O — Clock / Random

> ⚠️ **Target constraint**: `std::host::clock` と `std::host::random` は **wasm32-wasi** ターゲットが必要です。

> 📎 Fixture: [`tests/fixtures/stdlib_io/clock_random.ark`](../../tests/fixtures/stdlib_io/clock_random.ark)

```ark
use std::host::stdio
use std::host::clock
use std::host::random as host_random
fn main() {
    let t = clock::monotonic_now()
    stdio::println(i64_to_string(t))

    let r = host_random::random_i32()
    stdio::println(i32_to_string(r))
}
```

---

## Integration Recipes

実際のユースケースに近い、複数の stdlib 機能を組み合わせたレシピです。

### 単語カウンター — String + Vec

> 📎 Fixture: [`tests/fixtures/integration/word_counter.ark`](../../tests/fixtures/integration/word_counter.ark)

```ark
use std::host::stdio
fn count_words(s: String) -> i32 {
    if is_empty(s) {
        return 0
    }
    let words: Vec<String> = split(s, String_from(" "))
    let mut count: i32 = 0
    let mut i: i32 = 0
    while i < len(words) {
        let word: String = get_unchecked(words, i)
        if !is_empty(word) {
            count = count + 1
        }
        i = i + 1
    }
    count
}

fn main() {
    stdio::println(i32_to_string(count_words(String_from("hello world"))))           // 2
    stdio::println(i32_to_string(count_words(String_from("the quick brown fox"))))   // 4
    stdio::println(i32_to_string(count_words(String_from(""))))                       // 0
}
```

### Vec ソートと結合 — sort + join

> 📎 Fixture: [`tests/fixtures/integration/sort_vec.ark`](../../tests/fixtures/integration/sort_vec.ark)

```ark
use std::host::stdio
fn main() {
    let mut v: Vec<i32> = Vec_new_i32()
    push(v, 5)
    push(v, 3)
    push(v, 8)
    push(v, 1)
    sort_i32(v)

    let mut parts: Vec<String> = Vec_new_String()
    let mut i: i32 = 0
    while i < len(v) {
        push(parts, i32_to_string(get_unchecked(v, i)))
        i = i + 1
    }
    stdio::println(join(parts, String_from(" ")))  // 1 3 5 8
}
```

### 列挙型 + パターンマッチ — 式の評価

> 📎 Fixture: [`tests/fixtures/integration/calculator.ark`](../../tests/fixtures/integration/calculator.ark)

```ark
use std::host::stdio
enum Expr {
    Num(i32),
    Add(i32, i32),
    Sub(i32, i32),
    Mul(i32, i32),
}

fn eval(e: Expr) -> i32 {
    match e {
        Expr::Num(n) => n,
        Expr::Add(a, b) => a + b,
        Expr::Sub(a, b) => a - b,
        Expr::Mul(a, b) => a * b,
    }
}

fn main() {
    stdio::println(i32_to_string(eval(Expr::Num(42))))       // 42
    stdio::println(i32_to_string(eval(Expr::Add(10, 20))))   // 30
    stdio::println(i32_to_string(eval(Expr::Mul(6, 7))))     // 42
}
```

### フィボナッチ数列 — 再帰とループ

> 📎 Fixture: [`tests/fixtures/integration/fibonacci.ark`](../../tests/fixtures/integration/fibonacci.ark)

```ark
use std::host::stdio
fn fib(n: i32) -> i32 {
    if n <= 1 {
        return n
    }
    let mut a: i32 = 0
    let mut b: i32 = 1
    let mut i: i32 = 2
    while i <= n {
        let next: i32 = a + b
        a = b
        b = next
        i = i + 1
    }
    b
}

fn main() {
    let mut i: i32 = 0
    while i <= 10 {
        stdio::println(i32_to_string(fib(i)))
        i = i + 1
    }
}
```

### 連結リスト — Enum + Box + 再帰

> 📎 Fixture: [`tests/fixtures/integration/linked_list.ark`](../../tests/fixtures/integration/linked_list.ark)

```ark
use std::host::stdio
enum List {
    Nil,
    Cons(i32, Box<List>),
}

fn prepend(list: List, val: i32) -> List {
    List::Cons(val, Box_new(list))
}

fn list_sum(list: List) -> i32 {
    match list {
        List::Nil => 0,
        List::Cons(head, tail) => head + list_sum(unbox(tail)),
    }
}

fn main() {
    let list: List = List::Nil
    let list: List = prepend(list, 3)
    let list: List = prepend(list, 2)
    let list: List = prepend(list, 1)
    stdio::println(i32_to_string(list_sum(list)))  // 6
}
```

---

## v1 feature note

このブランチではメソッド構文や拡張構文も入っていますが、
共通で通しやすいサンプルとしてこの cookbook では関数呼び出し形式を優先しています。

## 関連

- [reference.md](reference.md) — stdlib 全関数リファレンス（自動生成）
- [core.md](core.md)
- [io.md](io.md)
- [../quickstart.md](../quickstart.md)
- [../current-state.md](../current-state.md)
