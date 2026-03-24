# std/io — I/O モジュール

資源モデル決定により **DirCap + RelPath 方式** を採用。

---

## 設計方針（確定）

- WASI 名を表面に出さない
- capability を型に直接乗せない（WASI-capability分析.txt の結論）
- effect / capability value / resource type / failure type を分離する

---

## 型定義

```
// capability 値（ユーザーコードでは作成不可）
type DirCap           // アクセス可能なディレクトリ

// パス表現
type RelPath          // DirCap に対する相対パス

// リソースハンドル
type FileHandle       // 開いたファイル

// 時刻型
type WallTime         // 実時刻
type MonotonicTime    // 経過時間計測用
type Duration         // ナノ秒精度の経過時間

// 乱数生成器
type Rng              // シード付き PRNG
```

---

## fs モジュール

```
// ファイル読み書き（バッファ全体）
fn fs.read_file(dir: DirCap, path: RelPath) -> Result[String, IOError]
fn fs.write_file(dir: DirCap, path: RelPath, content: str) -> Result[(), IOError]

// ストリーム API
fn fs.open(dir: DirCap, path: RelPath) -> Result[FileHandle, IOError]
fn fs.read(handle: FileHandle, buf: [u8]) -> Result[usize, IOError]
fn fs.write(handle: FileHandle, buf: [u8]) -> Result[usize, IOError]
fn fs.close(handle: FileHandle) -> Result[(), IOError]

// ユーティリティ
fn fs.exists(dir: DirCap, path: RelPath) -> bool
fn fs.create_dir(dir: DirCap, path: RelPath) -> Result[(), IOError]
fn fs.remove_file(dir: DirCap, path: RelPath) -> Result[(), IOError]
```

### RelPath の構築

```
// 文字列リテラルから作成
let path = RelPath::from("data/input.txt")

// ".." を含むパスは実行時エラー
let bad = RelPath::from("../secret")  // Err(IOError::InvalidPath)

// 結合
let sub = path.join("subdir/file.txt")
```

### DirCap の入手

`main` の引数 `Capabilities` から取得:

```
fn main(caps: Capabilities) -> Result[(), AppError] {
    let cwd = caps.cwd()
    let content = fs.read_file(cwd, RelPath::from("input.txt"))?
    Ok(())
}
```

---

## clock モジュール

wall clock（実時刻）と monotonic clock（経過時間計測用）を分離。

```
// wall clock: 実時刻
fn clock.wall_now() -> WallTime
fn WallTime::to_unix_seconds(self) -> i64
fn WallTime::to_string(self) -> String   // ISO 8601 形式

// monotonic clock: 経過時間計測用（単調増加）
fn clock.monotonic_now() -> MonotonicTime
fn clock.elapsed(start: MonotonicTime) -> Duration

// Duration 操作
fn Duration::as_nanos(self) -> u64
fn Duration::as_millis(self) -> u64
fn Duration::as_secs(self) -> u64
```

wall clock と monotonic clock を混在させる操作は型レベルで防止。

---

## random モジュール

暗号学的乱数（CSPRNG）と通常乱数を分離。

```
// 暗号学的乱数（WASI の get-random-bytes）
fn random.crypto_fill(buf: [u8]) -> Result[(), IOError]
fn random.crypto_u64() -> Result[u64, IOError]

// 通常乱数（シード付き PRNG、arukellt 内部実装）
fn Rng::from_seed(seed: u64) -> Rng
fn Rng::next_u32(self) -> u32
fn Rng::next_u64(self) -> u64
fn Rng::next_f64(self) -> f64            // [0.0, 1.0)
fn Rng::fill(self, buf: [u8])
fn Rng::shuffle[T](self, arr: [T])       // trait 導入後
```

**実装方針**:
- `crypto_*` は WASI の `random_get` を直接使用
- `Rng` は xorshift128+ 等の軽量 PRNG を arukellt で実装

---

## net モジュール

v0 スコープ外。async 設計前に入れない。

将来の方針（参考）:
- 同期 API は入れない
- async/await または明示的な Future 型と組み合わせて設計する
- TCP のみから始める

---

## IOError 型

全 I/O モジュール共通のエラー型。

```
enum IOError {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    InvalidPath,
    UnexpectedEof,
    BrokenPipe,
    OutOfMemory,
    Other(str),     // 将来的に Other は削除したい
}
```
