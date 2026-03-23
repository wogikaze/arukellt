# Arukellt

Arukellt は Rust で実装された、LLM ファーストの実験的な言語ツールチェーンです。
現在の `v0` プロトタイプは、LLM が生成・修復・検証しやすい小さな純粋ロジックプログラム向けに最適化されています。パイプラインは式中心、デフォルトでイミュータブル、そして不透明なコンパイラ失敗ではなく構造化診断を中心に設計されています。

## 現在の状態

このリポジトリには、すでに動作するバーティカルスライスが含まれています。

- `lang-core`: レキサー、耐障害性のあるインデント認識パーサー、AST、型付き AST、構造化診断
- `lang-ir`: High IR と Low IR への lowering
- `lang-interp`: 型付き・高レベルサブセット向けインタプリタ
- `lang-backend-wasm`: 現在のスカラー、リテラル文字列、フィールドを持たない ADT サブセット向け WASM バックエンド
- `arktc`: コンパイラ向けの `check` / `build` コマンド
- `chef`: インタプリタ向けの `run` / `test` / `benchmark` コマンド
- `arkli`: アドホック評価とファイルロードのための対話型 REPL
- `arktfmt`: ソースフォーマッタ
- `arktup`: プロトタイプインストールとデフォルト選択のためのローカルツールチェーン状態マネージャ
- `lang-playground-core`: ブラウザ playground 統合向け JSON / wasm-bindgen API

これはまだプロトタイプです。言語およびツールチェーンは意図的に未完成です。

## 対象サーフェス

対象となる `v0` サーフェスは、LLM にとっての明確さ、回復可能性、低曖昧性の生成を中心にしています。

- インデントベースのブロック
- トップレベルの順序: `import` -> `type` -> `fn`
- 名前指定で import するホスト効果。たとえば `import console` や `import fs`
- ユーザー向け例では `i64` をデフォルト整数型として使用
- `->` の多重用途を避けるため、関数型は `Fn<arg, result>` と記述
- `else` 必須の式中心 `if`
- コンストラクタ呼び出しを持つ ADT
- ワイルドカード警告と網羅性検査を持つ `match`
- `1..=100` のような範囲リテラル
- `n -> expr` のようなラムダ短縮構文。コールバック引数型は文脈から推論
- メソッドチェーンは純粋性を保ち、副作用は末尾パイプで境界に出す。たとえば `value |> console.println`
- 安定 JSON スキーマバージョン `v0.1` を持つ構造化診断
- `null`、例外、マクロ、暗黙的型変換、共有可変状態はなし

例:

```text
import console

type Error =
  DivisionByZero

fn divide(a: i64, b: i64) -> Result<i64, Error>:
  if b == 0:
    Err(DivisionByZero)
  else:
    Ok(a / b)

fn render_error(error: Error) -> String:
  match error:
    DivisionByZero -> "error"

fn main():
  match divide(10, 0):
    Ok(value) -> value |> string |> console.println
    Err(error) -> error |> render_error |> console.println
```

## 例

このリポジトリには、[`example/`](./example) に言語サーフェスの例が含まれています。

- hello world
- `1..=100` 上の fizz buzz。最後の `|> console.println` までチェーンの純粋性を維持
- factorial と fibonacci
- `map` / `filter` / `sum`
- ファイル読み取りと `Result` ベースのエラーハンドリング
- クロージャ
- 無限イテレータ
- 純粋なスカラー WASM フレンドリーなサブセット

同梱されているすべての例は `chef run` で実行でき、`chef test` で検証できます。
各例には [`example/meta/`](./example/meta) 配下に対応する fixture があり、現在のツールチェーンのスナップショット契約として機能します。
同梱例はすべて `arktc check` も通過します。
同梱例契約の機械可読な単一の正とされる情報源は [`example/meta/matrix.json`](./example/meta/matrix.json) にあります。
同梱例を変更したりバックエンド対応を拡張した場合は、そのファイルを更新してから `cargo test -p arktc -p chef --test examples` を再実行して契約を更新してください。

現在の同梱例マトリクスは次の通りです。

| example | `chef run` | `chef test` | `arktc check` | `arktc build --target wasm-js` | `arktc build --target wasm-wasi` |
| --- | --- | --- | --- | --- | --- |
| `closure.ar` | pass | pass | pass | pass | pass |
| `factorial.ar` | pass | pass | pass | pass | pass |
| `fibonacci.ar` | pass | pass | pass | pass | pass |
| `file_read.ar` | pass | pass | pass | fail | pass |
| `fizz_buzz.ar` | pass | pass | pass | pass | pass |
| `hello_world.ar` | pass | pass | pass | pass | pass |
| `infinite_iter.ar` | pass | pass | pass | pass | pass |
| `map_filter_sum.ar` | pass | pass | pass | pass | pass |
| `powers.ar` | pass | pass | pass | pass | pass |
| `result_error_handling.ar` | pass | pass | pass | pass | pass |
| `wasm_scalar.ar` | pass | pass | pass | pass | pass |

`wasm-wasi` は現在、リポジトリ内のすべての同梱例をビルドできます。`wasm-js` も `file_read.ar` を除くすべての同梱例をビルドできるため、例セットに残っているクロスターゲット差分は、リスト・イテレータ・文字列出力ではなくホストファイル I/O です。

リリース向けの参照資料については、[`docs/language-tour.md`](./docs/language-tour.md) と [`docs/std.md`](./docs/std.md) にある実行可能ドキュメントを参照してください。そこにあるスニペットはコミット済み fixture によって支えられ、テストスイートでも実行されています。

## ツール

公開 CLI サーフェスは `arktc`、`chef`、`arkli`、`arktfmt`、`arktup` に分かれています。
各公開バイナリとサブコマンドには、現在のプロトタイプ契約を説明するテスト済みの `--help` パスもあり、WASM サブセット、JSON のみの docs 出力、ローカル状態のみのツールチェーン管理など、意図的に制限されたサーフェスも含まれます。

### Check

```bash
cargo run -p arktc -- check path/to/file.ar --json
```

これはソースを `lang-core` までコンパイルし、構造化診断を出力します。JSON ペイロードには `code`、`stage`、`message`、`expected`、`actual`、`cause`、`suggested_fix`、`alternatives`、`confidence` といったバージョン付きフィールドが含まれます。

### Run

```bash
cargo run -p chef -- run path/to/file.ar --function main --args 3 9 --step
printf '1\n2 3\ntest\n' | cargo run -p chef -- run path/to/practicea.ar
```

これはインタプリタ経路で実行し、オプションでトレースも表示できます。インタプリタは WASM バックエンドより診断しやすいため、デフォルトの開発ループです。
プログラムが `stdin.read_text()` を呼ぶ場合、`chef run` は呼び出し元プロセスの stdin から読み取るため、競技プログラミング風の入力をそのままパイプで渡せます。
実行開始前にコンパイルが失敗した場合、`chef run` は非ゼロ終了し、構造化診断 JSON を stderr に書き出します。

### Test

```bash
cargo run -p chef -- test path/to/file.ar
cargo run -p chef -- test path/to/file.ar --json
```

`test_` で始まる名前の関数が実行され、`Bool(true)` を返す必要があります。
ファイルに `test_` 関数がひとつも定義されていない場合、`chef test` は隣接する `.stdout` fixture に対するスナップショットテストへフォールバックします。

```bash
cargo run -p arkli
```

`arkli` は最小限の GHCi スタイル REPL を提供します。1 行式を評価し、現在のセッション中の対話的 `let` 束縛を保持し、`:load path/to/file.ar`、`:reload`、`:type <expr>` をサポートし、`:quit` または `:q` で終了します。
`--json` は、見つかったテスト名と失敗を列挙するバージョン付き結果ペイロードを出力します。コンパイル失敗は stderr に構造化診断 JSON として出ます。
`--json` なしでも、コンパイル失敗時には非ゼロ終了前に実用的で人間可読な診断を stderr に表示します。

### Format

```bash
cargo run -p arktfmt -- path/to/file.ar
cargo run -p arktfmt -- path/to/file.ar --write
```

これはパース済みモジュールを整形し、結果を stdout に出すかソースファイルへ書き戻します。
レキサーまたはパーサーエラーがある場合、`arktfmt` は明示的に失敗し、プレースホルダの `<error>` ノードを出力せず入力ファイルも変更しません。

### Docs

```bash
cargo run -p arktdoc -- path/to/file.ar --format json
```

これはソースファイルをコンパイルし、型付き関数サーフェスのバージョン付き JSON ドキュメントを出力します。
現在のペイロードには入力ファイルパスに加えて、各関数の名前、可視性、パラメータ一覧、戻り値型が含まれます。
現在サポートされているのは `--format json` のみです。それ以外の format 値は JSON に黙ってフォールバックせず、明示的に失敗します。
ソースがコンパイルできない場合、`--format` に未サポート値が渡されていても、`arktdoc` は部分的な docs を出さず非ゼロ終了し、短い compilation-failure メッセージを出力します。

### Build

```bash
cargo run -p arktc -- build path/to/file.ar --target wasm-js --output out.wasm
cargo run -p arktc -- build path/to/file.ar --target wasm-wasi --output out.wasm
cargo run -p arktc -- build path/to/file.ar --target wasm-js-gc --emit wat
cargo run -p arktc -- build path/to/file.ar --target wasm-component-js --emit wat
cargo run -p arktc -- build path/to/file.ar --target wasm-js --emit wat
cargo run -p arktc -- build path/to/file.ar --target wasm-wasi --emit wat-min
cargo run -p chef -- build path/to/file.ar --target wasm-wasi --output out.wasm
```

現在の WASM バックエンドは、`wasm-wasi` では狭いスカラー＋リスト＋文字列サブセットを、`wasm-js` ではより小さいもののコレクション対応済み `List<i64>` サブセットをサポートします。
`wasm-js-gc` は、将来の GC 対応 JavaScript ホスト向けバックエンドのための明示的な実験ターゲット契約として文書化されていますが、現時点のビルドはそのバックエンドが存在しないため拒否されます。
`wasm-component-js` は、JavaScript または Node ホスト向けの将来の Component Model バックエンドのための別個の明示的実験ターゲット契約ですが、現時点のビルドはそのバックエンドが存在しないため拒否されます。
`chef build` は現在、`arktc build` と同じ target / emit マトリクスを公開しており、run/test/build ワークフローをひとつの CLI にまとめたい場合に有用です。
`--target` は ABI（`wasm-js`、`wasm-wasi`、または予約済みの実験契約 `wasm-js-gc` と `wasm-component-js`）を選び、`--emit` は出力形式（`wasm`、`wat`、`wat-min`）を選びます。
ターゲットごとの API 対応状況については、[`docs/std.md`](./docs/std.md) のターゲットサポートマトリクスを参照してください。上の同梱例マトリクスは例レベルのものですが、`parse.i64`、`split_whitespace`、`stdin.read_line` といった API が特定 WASM ターゲットで lower されるかどうかの単一の正とされる情報源は std ドキュメントです。
`--output` は現在オプションです。省略した場合、`arktc build` は `--emit wat` / `--emit wat-min` では WAT を出力し、それ以外ではコード生成成功後に生成された WASM バイト列を破棄します。
`--target wat` は現在も `--target wasm-js --emit wat` の非推奨エイリアスとして受け付けられます。
`wasm-js` は、Arukel 名でコンパイル済み関数を export する埋め込み可能モジュールを出力します。
`wasm-wasi` は `_start` のみを export するコマンド形式モジュールを出力します。ゼロ引数 `main` 関数が必要で、スカラー戻り値は ABI 境界で破棄されます。
`wasm-js-gc` は `wasm-js` と意図的に分離されています。その最初のスライス契約は、モジュール内部で GC 参照を許可しつつ公開 ABI はスカラーのみとする予定で、現時点のビルドは GC バックエンド実装まで contract error で拒否されます。
`wasm-component-js` も `wasm-js` と意図的に分離されています。その最初のスライス契約は、型付きホストインタフェースとスカラーのみの公開 export を備えた component-aware な JavaScript または Node ホスト ABI になる予定で、現時点のビルドは component バックエンド実装まで contract error で拒否されます。
`String` は現在、読み取り専用 `memory` に置かれた NUL 終端 UTF-8 リテラルへの生の `i32` ポインタとしてのみ lower されます。その ABI スライスでは、リテラル式とユーザー定義関数経由の直接 return がサポートされます。
フィールドを持たないユーザー定義 ADT は現在、生の数値タグとして lower され、`match` は subject がそうした ADT のひとつであり、各 arm が裸の variant 名または最後のワイルドカードである場合にのみ lower されます。
未対応サーフェスは黙って劣化しません。`arktc build` は、選択ターゲットで未対応の string helper、ペイロード付き ADT、パターン束縛、未対応ホスト呼び出しなど、文書化されたサブセット外の型や構文にコード生成が遭遇した時点でハードエラーで失敗し、そのエラーは [`docs/std.md`](./docs/std.md) の API レベル対応表を指し示します。

### Benchmark

```bash
cargo run -p chef -- benchmark benchmarks/pure_logic.json
```

これは JSON ベンチマークマニフェストに対する parse、typecheck、execution、pass の各件数を報告します。
`parse_success` は、後段の型検査に失敗しても、レキシングとパースを lexer/parser エラーなしで通過したケースを数えます。lexer/parser warning は parse success として数えられます。`typecheck_success` は完全にコンパイルされたケースのみを数えます。
[`benchmarks/pure_logic.json`](./benchmarks/pure_logic.json) のサンプルマニフェストが現在の参照セットです。

### Toolchain

```bash
cargo run -p arktup -- show
ARKTUP_HOME=/tmp/arktup cargo run -p arktup -- install v0.1.0
ARKTUP_HOME=/tmp/arktup cargo run -p arktup -- default v0.1.0
```

`arktup` は現在、ローカルツールチェーンメタデータのみを管理します。インストール済みバージョンと選択されたデフォルトバージョンを `ARKTUP_HOME/state.json` に、`ARKTUP_HOME` が未設定の場合は現在の作業ディレクトリ配下の `.arktup/state.json` に記録します。

## Browser Playground API

`crates/lang-playground-core` は、ブラウザ統合向けに JSON 指向の 2 つの関数を公開しています。

- `analyze_source_json(source)` はバージョン付き診断 JSON を返す
- `run_source_json(source, function, args_json, step)` は結果とオプションのトレース JSON を返す

これらは `wasm-bindgen` 経由で `analyze_source` と `run_source` としても export されます。

## リポジトリ構成

```text
.
├── benchmarks/
├── crates/
│   ├── arktc/
│   ├── arkli/
│   ├── arktdoc/
│   ├── arktfmt/
│   ├── arktup/
│   ├── chef/
│   ├── lang-core/
│   ├── lang-ir/
│   ├── lang-interp/
│   ├── lang-backend-wasm/
│   └── lang-playground-core/
└── Cargo.toml
```

## 制限事項

実行可能なプロトタイプは、完全な言語計画より意図的に狭い範囲にとどまっています。

- `lang-core` は内部的には依然として整数を `Int` として扱う一方、表面上の例では明示的な `i64` 表記を使っています
- 同梱例はインタプリタ経路で実行できますが、その大半は意図的に現在の WASM サブセット外にあります
- サポートされる標準ライブラリは依然として小さく、同梱例を中心に目的特化しています
- WASM バックエンドはプレースホルダモジュールを出さず、未対応サーフェスでハード失敗します
- WASM バックエンドは両方の WASM ターゲットで、リテラル文字列と `string()` / `join()` 由来のヒープ文字列をサポートします。より広い文字列操作と一般的な文字列 ABI ツールは未対応です
- WASM バックエンドは `wasm-wasi` 上で、ペイロード付き `Result` 値、ヒープベースのユーザー定義 ADT、パターン束縛、単項クロージャ、`List<i64>` コレクションヘルパ、`iter.unfold(...).take(n)` による最小限の `Seq<i64>` 実体化、`console.println`、`fs.read_text` をサポートします。より広いイテレータ / ホスト呼び出しコード生成は未対応です
- ホスト統合は現在、例指向の `console.println` と `fs.read_text` shim に限られます
- `clock`、`random`、`process`、パッケージ管理、ビルダー、より豊かな標準ライブラリはまだ未実装です
- `arktfmt` は現在、正規化された AST ベースの整形を再出力するのではなく、ソースを保存する形です
- `arktdoc` は現在 JSON のみを出力します。非 JSON の `--format` 値は、別の出力契約が実装されるまで拒否されます

## 作業キュー

[`issues/index.md`](./issues/index.md) は、このリポジトリ作業のための共有 AI 管理キューです。
AI エージェントは、作業開始・ブロック・分割・完了に応じて、対応する [`issues/open/`](./issues/open) および [`issues/done/`](./issues/done) 配下のファイルと一緒にこれを更新します。
インデックスは意図的に高密度のまま保たれており、人間と LLM が各 issue ファイルをすべて開き直さなくても、優先度・依存関係・要約を確認できます。

## Agent Harness

最小実用ハーネスのエントリポイントは [`docs/agent-harness.md`](./docs/agent-harness.md) です。
ここではエージェント向け指示を短く保ち、キュー、実行可能テスト、現在の ADR に裏付けられたワークフロー判断へ誘導します。
エージェントが完了を主張する前に、次を実行してください。

```bash
./scripts/verify-harness.sh
```

このガードレールは `cargo fmt --check`、`cargo clippy --workspace --lib --bins -- -D warnings`、`cargo test -p arktc --test issues`、`cargo test` を実行します。
現在 clippy ゲートが対象としているのはワークスペースの lib と bin です。test ターゲットへの clippy 強制は別の追跡判断として残っています。

## Development

リポジトリルートからワークスペースのテストスイートを実行します。

```bash
cargo test
```

静的 docs シェルに対するブラウザレベルのスモーク確認を行うには、次を実行します。

```bash
cargo test -p arktc --test docs_site docs_site_routes_render_in_headless_browser -- --exact --ignored --nocapture
```

このコマンドは `docs/` をルートにしたローカル `python3 -m http.server` を起動し、ヘッドレス `google-chrome` を使って `#/language-tour`、`#/std`、およびハッシュなしフォールバックルートを手動確認なしで検証します。

このプロジェクトは現在、主に次によって検証されています。

- コアパイプラインテスト
- IR lowering テスト
- インタプリタ評価テスト
- CLI 統合テスト
- playground API テスト
- ベンチマークループのスモークテスト
