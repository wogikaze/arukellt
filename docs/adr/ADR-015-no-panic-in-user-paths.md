# ADR-015: ユーザー到達パスの No-Panic 品質基準

ステータス: **ACCEPTED** — ユーザー到達パスでのpanic禁止  
作成日: 2026-04-09  
範囲: CLI, LSP, extension, manifest parsing

## 文脈

ユーザーが通常操作（`arukellt compile` の実行、LSP の hover、VS Code 拡張の有効化など）を
行うとき、Rust の `panic!` や JavaScript レベルのクラッシュに遭遇してはならない。
panic は行動可能な案内のない混乱した出力を生み、ユーザー到達パスでは許容できない。

## 決定

### 1. 「ユーザー到達パス」の定義

次のいずれかで起動できるコードパスをユーザー到達とする。

- 有効・無効いずれの引数でも起動できる任意の `arukellt` CLI サブコマンド
- 任意の LSP リクエスト（hover、completion、diagnostics など）
- VS Code 拡張の有効化、コマンド、タスク実行、デバッグアダプタ要求
- マニフェスト解析（`ark.toml`、`std/manifest.toml`、fixture マニフェスト）

### 2. ユーザー到達パスで禁止するパターン

- `panic!("...")` — 代わりに `anyhow::bail!` または `Err(...)` を返す
- ユーザー到達で `None`/`Err` になりうる `Option`/`Result` への `.unwrap()`
- ユーザー向けメッセージではなく内部メモである `.expect("...")`
- `todo!()`、`unimplemented!()` — 適切なスタブまたは feature gate に置き換える
- `unreachable!()` — 型不変条件により論理的に到達不能な場合のみ可。
  ユーザー入力で到達しうるならエラーへ変換する

### 3. 許容するパターン

- `Mutex` の `.lock().unwrap()` — 他スレッドが既に panic した場合のみ panic する。
  許容（mutex poison = 既にバグ）
- 型システムまたはコンパイラ検査済みアサーションで検証できる不変条件の
  `.expect("invariant: ...")`
- `#[cfg(test)]` 内の `panic!`
- `--internal-*` 開発者フラグ経由でのみ到達するコード内の `panic!`

### 4. エラー出力の基準

ユーザー向けエラー発生時:

- 明確で人間が読めるメッセージを出す（デフォルトでスタックトレースなし）
- 関連コンテキスト（どのファイル、どのコマンド、どのフィールド）を含める
- 非ゼロ終了コードで終了する
- 可能なら修正案やドキュメントへの案内を付ける

### 5. 新規コードの規則

新規 PR はユーザー到達パスに `unwrap()`、`expect()`、`panic!()`、`todo!()`、
`unimplemented!()` を導入してはならない。導入する PR はレビューで却下する。

## 現状（2026-04-09）

ユーザー向けクレートの監査:

| Crate | 危険な panic |
|-------|----------------|
| `crates/arukellt/src/` | なし |
| `crates/ark-lsp/src/` | なし（`lock().unwrap()` は mutex のみ） |
| `src/compiler/resolver.arksrc/` | なし（マニフェスト解析は #580 で `ark-manifest` から移行） |
| ~~`crates/ark-manifest/src/`~~ | ~~なし~~（#580 で削除） |
| `crates/ark-driver/src/` | なし |
| `extensions/arukellt-all-in-one/src/` | なし |

本 ADR 時点でユーザー向けクレートはクリーンである。

## 強制

- `scripts/manager.py` に panic 監査チェック `--no-panic-audit` がある
- CI は `integration` 層でこのチェックを実行する
- 新規違反は `P1` バグ（即時修正必須）として扱う

## 参照

- `issues/done/243-no-panic-in-user-paths-quality-standard.md`
- `docs/contributing.md`
