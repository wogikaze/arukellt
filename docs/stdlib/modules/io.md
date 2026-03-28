# std::io / std::fs / std::path / std::process — 入出力と実行環境

> **状態**: `fs_read_file` / `fs_write_file` / `clock_now` / `random_i32` は部分実装済み。
> 完全な path/process/env は v3 で実装予定。

---

## std::fs (ファイルシステム)

### 現行 API (v2 互換)

```ark
pub fn fs_read_file(path: String) -> Result<String, String>
pub fn fs_write_file(path: String, content: String) -> Result<(), String>
```

### v3 追加 API

```ark
// 読み取り
pub fn fs_read_to_string(path: String) -> Result<String, String>
pub fn fs_read_bytes(path: String) -> Result<Bytes, String>
pub fn fs_read_lines(path: String) -> Result<Vec<String>, String>

// 書き込み
pub fn fs_write(path: String, content: String) -> Result<(), String>
pub fn fs_write_bytes(path: String, bytes: Bytes) -> Result<(), String>
pub fn fs_append(path: String, content: String) -> Result<(), String>

// ディレクトリ
pub fn fs_exists(path: String) -> bool
pub fn fs_is_file(path: String) -> bool
pub fn fs_is_dir(path: String) -> bool
pub fn fs_mkdir(path: String) -> Result<(), String>
pub fn fs_mkdir_all(path: String) -> Result<(), String>
pub fn fs_remove_file(path: String) -> Result<(), String>
pub fn fs_remove_dir(path: String) -> Result<(), String>
pub fn fs_list_dir(path: String) -> Result<Vec<String>, String>

// メタデータ
pub fn fs_file_size(path: String) -> Result<i64, String>
```

---

## std::path (パス操作)

```ark
// 結合・分解
pub fn path_join(base: String, part: String) -> String
pub fn path_join_parts(parts: Vec<String>) -> String
pub fn path_parent(path: String) -> Option<String>
pub fn path_file_name(path: String) -> Option<String>
pub fn path_stem(path: String) -> Option<String>         // without extension
pub fn path_extension(path: String) -> Option<String>

// 正規化
pub fn path_normalize(path: String) -> String            // resolve . and ..
pub fn path_is_absolute(path: String) -> bool
pub fn path_is_relative(path: String) -> bool
pub fn path_to_absolute(path: String) -> Result<String, String>

// その他
pub fn path_with_extension(path: String, ext: String) -> String
pub fn path_components(path: String) -> Vec<String>
```

---

## std::io (Reader / Writer)

```ark
// 標準入出力 (WASI CLI stream)
pub fn stdin_read_line() -> Result<String, String>
pub fn stdin_read_all() -> Result<String, String>
pub fn stdout_write(s: String) -> Result<(), String>
pub fn stderr_write(s: String) -> Result<(), String>

// println / print / eprintln は prelude に残す (既存互換)
```

---

## std::process (プロセス制御)

```ark
// コマンドライン引数
pub fn args() -> Vec<String>
pub fn args_nth(i: i32) -> Option<String>
pub fn args_count() -> i32

// 終了
pub fn exit(code: i32) -> Never       // process.exit 相当

// 環境変数
pub fn env_var(name: String) -> Option<String>
pub fn env_vars() -> Vec<(String, String)>    // v3
```

---

## std::time (時刻・期間)

```ark
// 現行
pub fn clock_now() -> i64    // WASI clock_time_get, ナノ秒 UNIX タイムスタンプ

// v3 追加
pub fn monotonic_now_ns() -> i64         // monotonic clock (ns)
pub fn sleep_ms(ms: i32) -> Result<(), String>   // target-gated (WASI P2 以降)
```

---

## std::random

```ark
// 現行
pub fn random_i32() -> i32    // WASI random_get

// v3 追加
pub fn random_i64() -> i64
pub fn random_f64() -> f64    // [0.0, 1.0)
pub fn random_bytes(n: i32) -> Bytes
pub fn random_i32_range(lo: i32, hi: i32) -> i32   // [lo, hi)
```

---

## ターゲット制約

| API | T1 | T3 (WASI P1) | T3 (WASI P2) |
|-----|----|-------------|-------------|
| `fs_read_file` | ❌ | ✅ | ✅ |
| `fs_write_file` | ❌ | ✅ | ✅ |
| `fs_read_bytes` | ❌ | 🔨 v3 | ✅ |
| `fs_list_dir` | ❌ | ❌ | 🔨 v3 |
| `args()` | ❌ | ✅ (args_get) | ✅ |
| `exit()` | ✅ | ✅ | ✅ |
| `env_var()` | ❌ | ❌ | 🔨 v3 |
| `stdin_read_line()` | ❌ | 🔨 v3 | 🔨 v3 |
| `sleep_ms()` | ❌ | ❌ | 🔮 v4 |

T1 で呼ぶとコンパイルエラー (E0093: API not available on target T1)。

---

## v3 実装 issue

- [#044](../../issues/open/044-fs-path-process.md) — fs/path/process 完全実装
