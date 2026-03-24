# WASI 標準 API の資源モデル

ステータス: ★ OPEN — **ADR-002 と並んで最重要の未決定事項**

## 問題

`fs.read_file(path: String)` を出した瞬間に capability 設計の半分が死ぬ。

- String ベースの path は「どのディレクトリにアクセスできるか」を型で保証できない
- safety も portability も消える
- WASI の本質（ability-based security）が形骸化する

「WASI 名を隠す標準 API」は何も偉くない。名前を変えただけの薄い rename ラッパーにしかならない。**決めるべきは名前ではなく資源モデルだ。**

---

## 2つの選択肢

### 選択肢 A: String path ベース

```
fs.read_file(path: String) -> Result[String, IOError]
```

利点:
- 実装が単純
- ユーザーが慣れている
- LLM が書きやすい

欠点:
- capability の意味がない
- 移植性の保証がない
- テスト・サンドボックスが難しい

### 選択肢 B: capability/value/resource type ベース

```
fs.read_file(dir: DirCap, path: RelPath) -> Result[FileHandle, IOError]
```

必要な型:
- `DirCap` — アクセス可能なディレクトリを表す capability 値
- `RelPath` — DirCap に対する相対パス（String ではない）
- `FileHandle` — 開いたファイルの抽象ハンドル
- `IOError` — I/O 失敗の型

利点:
- WASI p2 の設計と整合する
- サンドボックス・テストが型レベルで表現できる
- 移植性が高い

欠点:
- API が複雑
- LLM が最初に書くコードが冗長になる
- DirCap の「入手方法」を言語レベルで設計する必要がある（コマンドライン引数? 環境変数? main の引数?）

---

## 各 API の決定項目

以下を一つずつ決める。選択肢 A / B の二択ではなく、API ごとに決める。

### fs

| 問い | 選択肢 |
|------|--------|
| path は String か DirCap+RelPath か | 未決定 |
| read と write を分けるか（read-only cap） | 未決定 |
| open の返り値は FileHandle か全内容 String か | 未決定 |

### clock

| 問い | 選択肢 |
|------|--------|
| wall clock と monotonic clock を分けるか | 分ける（推奨）/ まとめる |
| 返り値は何か（nanoseconds, Duration 型, etc.） | 未決定 |

WASI p2 では `monotonic-clock` と `wall-clock` は別リソース。これに合わせるのが自然。

### random

| 問い | 選択肢 |
|------|--------|
| 暗号学的乱数と通常乱数を分けるか | 分ける（推奨）/ まとめる |
| API は `fill(buf: &mut [u8])` か `next_u64()` か | 未決定 |

### net

| 問い | 選択肢 |
|------|--------|
| 同期 API か非同期専用か | 非同期専用（async なし v0 ではホスト依存に閉じ込める） |
| v0 スコープに入れるか | **入れない可能性が高い**（async 設計前） |

---

## capability の「入手方法」設計

選択肢 B を採る場合、`DirCap` をどこから取るかを決める必要がある。

候補:
1. `main` の引数として渡す（`fn main(caps: Capabilities) -> Result[(), Error]`）
2. 環境変数から初期化する
3. WASI p2 の `wasi:filesystem` リソースをそのまま wrap する

いずれにせよ、DirCap は「作る」ものではなく「もらう」ものとして設計する。ユーザーコードの中で `DirCap::new("/")` のように作れてしまったら capability の意味がない。

## 決定

**選択肢 B: capability/value/resource type ベースを採用する**

決定日: 2026-03-24

### 決定内容

- fs: DirCap + RelPath 方式を採用
- clock: wall/monotonic を分離
- random: 暗号学的/通常を分離
- net: v0 スコープ外

### capability の入手方法

`main` 関数の引数として受け取る:

```
fn main(caps: Capabilities) -> Result[(), AppError] {
    let dir = caps.cwd()        // カレントディレクトリへの capability
    let content = fs.read_file(dir, RelPath::from("data.txt"))?
    Ok(())
}
```

`Capabilities` 型:
```
struct Capabilities {
    fn cwd(self) -> DirCap           // カレントディレクトリ（読み書き可）
    fn args(self) -> [String]        // コマンドライン引数
    fn env(self) -> Env              // 環境変数アクセス
    fn stdin(self) -> FileHandle     // 標準入力
    fn stdout(self) -> FileHandle    // 標準出力
    fn stderr(self) -> FileHandle    // 標準エラー
}
```

### 根拠

1. WASI p2 の設計と整合する
2. サンドボックス・テストが型レベルで表現できる
3. 移植性が高い
4. capability を「もらう」ものとして設計することで、安全性を担保

---

## 関連

- `ADR-002`: メモリモデルが決まると FileHandle の表現が変わる
- `WASI-capability分析.txt`: この設計の根拠となる分析
- WASI p2 仕様: `wasi:filesystem`, `wasi:clocks`, `wasi:random` を参照
