# std/io — 現在の I/O API

> このページは **現在の実装** に合わせた要約です。より広い capability 設計は `docs/platform/wasi-resource-model.md` を参照してください。

現行実装で end-to-end に使える I/O は、薄い wrapper 経由の次の API です。

## Filesystem

実装ファイル: `std/io/fs.ark`

```ark
pub fn fs_read_file(path: String) -> Result<String, String>
pub fn fs_write_file(path: String, content: String) -> Result<(), String>
```

- WASI Preview 1 ベース
- path は `String`
- capability 引数はまだありません
- `--dir` を渡さない限り filesystem access は無効です

### 例

```ark
fn main() {
    let r = fs_read_file(String_from("input.txt"))
    match r {
        Ok(content) => print(content),
        Err(e) => println(e),
    }
}
```

## Clock

実装ファイル: `std/io/clock.ark`

```ark
pub fn clock_now() -> i64
```

- WASI `clock_time_get` ベース
- ナノ秒タイムスタンプを返します

## Random

実装ファイル: `std/io/random.ark`

```ark
pub fn random_i32() -> i32
```

- WASI `random_get` ベース

## 補足

古い設計文書には capability-based I/O (`Capabilities`, `DirCap`, `RelPath` など) も出てきますが、
それらは現行 API の前提ではありません。現在のコンパイラ向けにはこのページの wrapper 形を基準にしてください。

これらの将来設計を見たい場合は archive 済みの `platform/wasi-resource-model.md` を参照してください。

## 関連

- [Current state](../current-state.md)
- [WASI リソースモデル](../platform/wasi-resource-model.md)
