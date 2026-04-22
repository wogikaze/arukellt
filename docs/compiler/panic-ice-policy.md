# Panic / ICE Policy

> **Current-first**: 実装の現在地は [../current-state.md](../current-state.md) を参照してください。
> このドキュメントは、Arukellt のコンパイラ・ランタイム・CLI における
> 「失敗の四分類」を定義する契約 (contract) です。
> Stream 3 of [#592](../../issues/) (panic/ICE convergence) — see [#615](../../issues/).

## なぜこの分類が必要か

現在のコードベースは、次の四つを混同しがちである:

1. ユーザーが書いたコードに起因する **回復可能な失敗** (`Result`)
2. 「絶対に起きないはず」を表明する **コンパイラ内部 assertion** (ICE)
3. ユーザーコードからの **明示的 / 暗黙の panic** (runtime trap)
4. CLI から見える **エラー出力**

それぞれ責任の所在と出力フォーマットが異なるため、この四分類を明確に分けて
扱うことを Arukellt の方針とする。

## 四つのカテゴリ

### 1. ICE — Internal Compiler Error

**定義**: コンパイラ自身の不変条件 (invariant) が壊れたことを示す。
ユーザーの入力 (どれほど壊れていても) では絶対に発火してはならない。

**契約**:

- 出力は必ず `[BUG] internal compiler error: ...` で始める
- 出力先は stderr
- 終了ステータスは **101**
- メッセージには次を含めること:
  - 不変条件の説明
  - issue 報告先 (URL またはコマンド)
  - 必要なら短い backtrace ヒント (full backtrace は `RUST_BACKTRACE=1` 経由)
- ICE はバグである。CI で発火した場合は P1 として扱う

**例 (期待される出力形)**:

```text
[BUG] internal compiler error: unexpected MIR shape after lowering
  please report this at: https://github.com/.../issues/new
  hint: re-run with RUST_BACKTRACE=1 for a full trace
```

**STOP_IF**: 「ユーザーがこの入力を書くとコンパイラがクラッシュする」状態は
ICE ではなく **構造化された診断** (parse/resolve/typecheck エラー) で
ハンドリングすべき (criterion 3 of #615 audit slice)。

### 2. User panic — ユーザーコードからの panic / unreachable

**定義**: ユーザーが書いた Arukellt コードが、
`panic!` / `unreachable` / 範囲外アクセス / ゼロ除算などで
runtime trap を起こすケース。

**契約**:

- runtime は **trap** を発生させる
- CLI / ホストは trap を捕まえ、ユーザー向けに整形して表示する
  - panic の文字列メッセージ
  - 可能ならソース位置 (line:col)
  - 終了ステータスは非ゼロ (具体的な値はランタイム規約に従う)
- Rust 側の panic backtrace は **絶対に表示しない**
- これはバグではなくユーザーコードの動作であるため、
  `[BUG]` プレフィックスは付けない

### 3. Result — 回復可能な失敗

**定義**: 失敗することが正常な動作の一部である関数の戻り値。
標準ライブラリ API、ホスト境界呼び出し、I/O などが該当する。

**契約**:

- 戻り値は `Result<T, E>` を返す
- panic / unwrap / expect で表現してはならない
- 呼び出し元は `?` または `match` で扱う
- スタイル / 命名 / エラー型の選び方は [#613](../../issues/) の
  stdlib Result surface で別途定義される (この doc では型は定義しない)

### 4. CLI error reporting — CLI の人間向けエラー出力

**定義**: `arukellt` CLI が失敗を報告するとき、
ユーザーが画面で見る出力フォーマット。

**契約**:

- Rust の panic message / backtrace を素のまま表示してはならない
- 構造化された 1 行サマリ + 必要なら詳細
- ファイル / コマンド / フィールドなど、原因の特定に必要な context を含める
- 終了ステータスは非ゼロ
- 可能なら fix hint や doc へのリンクを含める
- 細則は [ADR-015](../adr/ADR-015-no-panic-in-user-paths.md) に従う

## 表で見る違い

| カテゴリ | 起因 | 出力プレフィックス | 終了 status | バグか |
|---|---|---|---|---|
| ICE | コンパイラ内部 invariant 違反 | `[BUG] internal compiler error:` | 101 | はい (compiler bug) |
| User panic | ユーザーコードの panic / unreachable | (整形された panic message) | 非ゼロ | いいえ |
| Result | 回復可能な失敗 | (呼び出し元が判断) | (該当なし) | いいえ |
| CLI error | CLI が報告するユーザー向けエラー | (構造化メッセージ) | 非ゼロ | いいえ |

## 関連 ADR

- [ADR-015: No-Panic Quality Standard for User-Facing Paths](../adr/ADR-015-no-panic-in-user-paths.md)
  — `unwrap` / `expect` / `panic!` の禁止パターンと許容パターン

## このスライスの範囲外 (将来の #615 スライス)

このドキュメントは [#615](../../issues/) の acceptance criterion #1 のみを
満たす。次の作業は別スライスで行う:

- **CLI `.unwrap()` / `.expect()` 監査と置換** (criterion 2)
  — `crates/arukellt/src/` の最低 3 箇所を構造化エラーに置換
- **コンパイラ assertion の demotion** (criterion 3)
  — 有効なユーザー入力で発火する assertion を構造化診断に変換
- **ICE 出力フォーマットの実装** (criterion 4)
  — `[BUG]` プレフィックス + status 101 を `crates/ark-diagnostics` で実装

これらは Rust ソースの変更を伴うため、本 doc スライス
(impl-language-docs) の対象外であり、別の implementation slice として
issue #615 上で扱う。

## 関連 issue

- 親: [#592](../../issues/) — Error Handling Convergence
- 兄弟: [#613](../../issues/) — stdlib Result surface
- 兄弟: [#614](../../issues/) — Diagnostic struct
