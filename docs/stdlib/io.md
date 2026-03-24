# std/io — I/O モジュール

資源モデル決定により **DirCap + RelPath 方式** を採用。
すべて関数呼び出し形式（メソッド構文なし）。

---

## 設計方針

- WASI 名を表面に出さない
- capability を型に直接乗せない
- effect / capability value / resource type / failure type を分離
- すべて関数呼び出し形式

---

## 型定義

```
// Capability 値（ユーザーコードでは作成不可）
type DirCap           // アクセス可能なディレクトリ
type Capabilities     // main が受け取る初期 capability

// パス表現
type RelPath          // DirCap に対する相対パス

// リソースハンドル
type FileHandle       // 開いたファイル
type Stdin            // 標準入力
type Stdout           // 標準出力
type Stderr           // 標準エラー

// 時刻型
type WallTime         // 実時刻
type MonotonicTime    // 経過時間計測用
type Duration         // ナノ秒精度の経過時間

// 乱数生成器
type Rng              // シード付き PRNG
```

---

## Capabilities（エントリポイント）

`main` 関数の引数として受け取る：

```
fn main(caps: Capabilities) -> Result[(), AppError] {
    // capability から各リソースにアクセス
}
```

### 組み込み関数

```
// ディレクトリ capability
fn cwd(caps: Capabilities) -> DirCap
fn preopened_dir(caps: Capabilities, name: String) -> Option[DirCap]

// 標準入出力
fn stdin(caps: Capabilities) -> Stdin
fn stdout(caps: Capabilities) -> Stdout
fn stderr(caps: Capabilities) -> Stderr

// コマンドライン引数・環境変数
fn args(caps: Capabilities) -> Vec[String]
fn env_var(caps: Capabilities, key: String) -> Option[String]
fn env_all(caps: Capabilities) -> Vec[(String, String)]
```

---

## fs モジュール

### ファイル読み書き

```
// 全体読み込み
fn fs_read_file(dir: DirCap, path: RelPath) -> Result[String, IOError]
fn fs_read_bytes(dir: DirCap, path: RelPath) -> Result[Vec[i32], IOError]

// 全体書き込み
fn fs_write_file(dir: DirCap, path: RelPath, content: String) -> Result[(), IOError]
fn fs_write_bytes(dir: DirCap, path: RelPath, data: Vec[i32]) -> Result[(), IOError]

// 追記
fn fs_append_file(dir: DirCap, path: RelPath, content: String) -> Result[(), IOError]
```

### ストリーム API

```
// 開く
fn fs_open(dir: DirCap, path: RelPath) -> Result[FileHandle, IOError]
fn fs_create(dir: DirCap, path: RelPath) -> Result[FileHandle, IOError]

// 読み書き
fn fs_read(handle: FileHandle, buf: Vec[i32]) -> Result[i32, IOError]  // 読み込んだバイト数
fn fs_write(handle: FileHandle, buf: Vec[i32]) -> Result[i32, IOError]  // 書き込んだバイト数

// 閉じる
fn fs_close(handle: FileHandle) -> Result[(), IOError]
```

### ファイルシステム操作

```
// 情報
fn fs_exists(dir: DirCap, path: RelPath) -> bool
fn fs_is_file(dir: DirCap, path: RelPath) -> bool
fn fs_is_dir(dir: DirCap, path: RelPath) -> bool
fn fs_size(dir: DirCap, path: RelPath) -> Result[i64, IOError]

// ディレクトリ
fn fs_create_dir(dir: DirCap, path: RelPath) -> Result[(), IOError]
fn fs_create_dir_all(dir: DirCap, path: RelPath) -> Result[(), IOError]
fn fs_remove_dir(dir: DirCap, path: RelPath) -> Result[(), IOError]
fn fs_list_dir(dir: DirCap, path: RelPath) -> Result[Vec[String], IOError]

// ファイル
fn fs_remove_file(dir: DirCap, path: RelPath) -> Result[(), IOError]
fn fs_rename(dir: DirCap, from: RelPath, to: RelPath) -> Result[(), IOError]
fn fs_copy(dir: DirCap, from: RelPath, to: RelPath) -> Result[(), IOError]
```

---

## RelPath（相対パス）

### 組み込み関数

```
// 作成
fn RelPath_from(s: String) -> Result[RelPath, IOError]

// 検証
fn relpath_is_valid(path: RelPath) -> bool

// 結合
fn relpath_join(base: RelPath, part: String) -> RelPath

// 分解
fn relpath_parent(path: RelPath) -> Option[RelPath]
fn relpath_filename(path: RelPath) -> Option[String]
fn relpath_extension(path: RelPath) -> Option[String]
```

### 制約

- `..` を含むパスは拒否（実行時エラー）
- 絶対パスは拒否
- 空パス `.` は許可

---

## 標準入出力

### Stdin

```
fn stdin_read_line(stdin: Stdin) -> Result[String, IOError]
fn stdin_read_all(stdin: Stdin) -> Result[String, IOError]
fn stdin_read_bytes(stdin: Stdin, buf: Vec[i32]) -> Result[i32, IOError]
```

### Stdout / Stderr

```
fn stdout_write(stdout: Stdout, s: String) -> Result[(), IOError]
fn stdout_write_bytes(stdout: Stdout, data: Vec[i32]) -> Result[(), IOError]
fn stdout_flush(stdout: Stdout) -> Result[(), IOError]

fn stderr_write(stderr: Stderr, s: String) -> Result[(), IOError]
fn stderr_write_bytes(stderr: Stderr, data: Vec[i32]) -> Result[(), IOError]
fn stderr_flush(stderr: Stderr) -> Result[(), IOError]
```

### 簡易版（capability 不要）

```
fn print(s: String)         // stdout に出力
fn eprintln(s: String)      // stderr に出力
fn read_line() -> String    // stdin から1行読み込み
```

**注意**: 簡易版は暗黙の global state を使用。テストには不向き。

---

## clock モジュール

### Wall Clock（実時刻）

```
fn clock_wall_now() -> WallTime
fn wall_to_unix_seconds(t: WallTime) -> i64
fn wall_to_string(t: WallTime) -> String  // ISO 8601 形式
```

### Monotonic Clock（経過時間計測）

```
fn clock_monotonic_now() -> MonotonicTime
fn clock_elapsed(start: MonotonicTime) -> Duration
```

### Duration

```
fn duration_from_nanos(n: i64) -> Duration
fn duration_from_millis(n: i64) -> Duration
fn duration_from_secs(n: i64) -> Duration

fn duration_as_nanos(d: Duration) -> i64
fn duration_as_millis(d: Duration) -> i64
fn duration_as_secs(d: Duration) -> i64
fn duration_as_secs_f64(d: Duration) -> f64
```

---

## random モジュール

### 暗号学的乱数（CSPRNG）

WASI の `random_get` を使用：

```
fn crypto_fill(buf: Vec[i32]) -> Result[(), IOError]
fn crypto_u32() -> Result[i32, IOError]
fn crypto_u64() -> Result[i64, IOError]
```

### 通常乱数（PRNG）

arukellt 内部実装（xorshift128+）：

```
// 作成
fn rng_from_seed(seed: i64) -> Rng
fn rng_from_entropy() -> Result[Rng, IOError]  // crypto_u64 でシード

// 生成
fn rng_next_u32(rng: Rng) -> i32
fn rng_next_u64(rng: Rng) -> i64
fn rng_next_f64(rng: Rng) -> f64              // [0.0, 1.0)
fn rng_next_range_i32(rng: Rng, min: i32, max: i32) -> i32

// バイト列生成
fn rng_fill(rng: Rng, buf: Vec[i32])

// シャッフル（型特化）
fn rng_shuffle_i32(rng: Rng, arr: Vec[i32])
fn rng_shuffle_String(rng: Rng, arr: Vec[String])
```

---

## IOError 型

```
enum IOError {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    InvalidPath,
    UnexpectedEof,
    BrokenPipe,
    OutOfMemory,
    Interrupted,
    InvalidInput,
    Other(String),
}
```

---

## 使用例

### ファイル読み書き

```
fn main(caps: Capabilities) -> Result[(), IOError] {
    let dir = cwd(caps)
    
    // 読み込み
    let content = fs_read_file(dir, RelPath_from("input.txt")?)?
    
    // 処理
    let upper = to_upper(content)
    
    // 書き込み
    fs_write_file(dir, RelPath_from("output.txt")?, upper)?
    
    Ok(())
}
```

### 標準入出力

```
fn main(caps: Capabilities) -> Result[(), IOError] {
    let stdin_h = stdin(caps)
    let stdout_h = stdout(caps)
    
    stdout_write(stdout_h, "Enter your name: ")?
    let name = stdin_read_line(stdin_h)?
    
    let greeting = concat(String_from("Hello, "), name)
    stdout_write(stdout_h, greeting)?
    
    Ok(())
}
```

### 時間計測

```
fn benchmark(f: fn()) -> Duration {
    let start = clock_monotonic_now()
    f()
    clock_elapsed(start)
}

fn main() {
    let d = benchmark(|| {
        let mut sum = 0
        let mut i = 0
        while i < 1000000 {
            sum = sum + i
            i = i + 1
        }
    })
    
    print(concat(String_from("Elapsed: "), duration_as_millis(d)))
}
```

### 乱数生成

```
fn main() -> Result[(), IOError] {
    let rng = rng_from_entropy()?
    
    let mut i = 0
    while i < 10 {
        let n = rng_next_range_i32(rng, 1, 100)
        print(i32_to_string(n))
        i = i + 1
    }
    
    Ok(())
}
```

---

## 関連

- `docs/platform/wasi-resource-model.md`: capability 設計の詳細
- `docs/platform/abi.md`: WASI との ABI 境界
- `docs/design/gc-c-abi-bridge.md`: GC ⇔ C 境界の設計
