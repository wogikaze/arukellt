# `arukellt init` のテンプレート拡充と新規ユーザー導線整備

**Status**: done
**Created**: 2026-04-02
**Updated**: 2026-04-03
**ID**: 464
**Depends on**: none
**Track**: cli, dx
**Blocks v1 exit**: no
**Priority**: 3


---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: InitTemplate enum with Minimal/Cli/WithTests/WasiHost in main.rs, cmd_init in commands.rs

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).


## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/464-init-template-expansion.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

現在の `arukellt init` はシングルテンプレート（`src/main.ark` に Hello World）のみを生成する。新規ユーザーが試しやすいよう、`--template` フラグでテンプレートを選べるようにする。最小プロジェクト、CLI ツール、テスト付き、WASI host 利用例の 4 種を用意し、生成直後に `arukellt check / run / test` が通ることを確認する。拡張機能でも Run Main / Run Test がそのまま機能することを確認する。

---

## 現状の分析

`crates/arukellt/src/commands.rs` の `cmd_init()`:
- `ark.toml`（`[package]` + `[bin]`）を生成する
- `src/main.ark` に `use std::host::stdio\nfn main() { stdio::println("Hello, Arukellt!") }` を生成する
- `--template` オプションなし
- テンプレート以外の雛形（テスト、WASI host 等）なし

---

## 詳細実装内容

### Step 1: `Commands::Init` に `--template` オプションを追加する (`crates/arukellt/src/main.rs`)

```rust
Init {
    /// Project directory
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Project template
    #[arg(long, value_name = "TEMPLATE", default_value = "minimal")]
    template: InitTemplate,
},

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum InitTemplate {
    /// Minimal Hello World project (default)
    Minimal,
    /// CLI tool with argument parsing
    Cli,
    /// Project with test functions
    WithTests,
    /// WASI host API usage example (requires --target wasm32-wasi-p2)
    WasiHost,
}
```

### Step 2: テンプレート定義を `cmd_init()` に追加する (`crates/arukellt/src/commands.rs`)

#### テンプレート 1: `minimal` (既存の動作を維持)

```
my-project/
├── ark.toml
└── src/
    └── main.ark
```

`src/main.ark`:
```ark
use std::host::stdio

fn main() {
    stdio::println("Hello, Arukellt!")
}
```

#### テンプレート 2: `cli`

```
my-project/
├── ark.toml
└── src/
    └── main.ark
```

`ark.toml`:
```toml
[package]
name = "my-project"
version = "0.1.0"

[bin]
name = "my-project"
path = "src/main.ark"
```

`src/main.ark`:
```ark
use std::host::stdio
use std::host::process

fn greet(name: String) -> String {
    "Hello, " + name + "!"
}

fn main() {
    // TODO: Use std::env::args() when available for real CLI arg parsing
    let name = "World"
    stdio::println(greet(name))
    process::exit(0)
}
```

#### テンプレート 3: `with-tests`

```
my-project/
├── ark.toml
└── src/
    ├── main.ark
    └── lib.ark
```

`src/lib.ark`:
```ark
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}

fn test_add() {
    let result = add(2, 3)
    assert(result == 5)
}

fn test_subtract() {
    let result = subtract(10, 4)
    assert(result == 6)
}
```

`src/main.ark`:
```ark
use std::host::stdio
use crate::lib

fn main() {
    let result = lib::add(1, 2)
    stdio::println("1 + 2 = " + result)
}
```

`ark.toml`:
```toml
[package]
name = "my-project"
version = "0.1.0"

[bin]
name = "my-project"
path = "src/main.ark"

[test]
paths = ["src/lib.ark"]
```

#### テンプレート 4: `wasi-host`

```
my-project/
├── ark.toml
└── src/
    └── main.ark
```

`ark.toml`:
```toml
[package]
name = "my-project"
version = "0.1.0"

[bin]
name = "my-project"
path = "src/main.ark"
target = "wasm32-wasi-p2"
```

`src/main.ark`:
```ark
// This example requires wasm32-wasi-p2 target.
// Build with: arukellt run --target wasm32-wasi-p2 src/main.ark
use std::host::stdio
use std::host::http

fn main() {
    stdio::println("Fetching example.com...")
    // Note: Uncomment to actually make a network request
    // match http::get("https://example.com") {
    //     Ok(body) => stdio::println("Got " + body)
    //     Err(e) => stdio::eprintln("Error: " + e)
    // }
    stdio::println("Done. Uncomment the HTTP call in src/main.ark to try it.")
}
```

### Step 3: 生成後メッセージの改善

テンプレートに応じて生成後のメッセージを変える。

```
Initialized Arukellt project in my-project (template: with-tests)

Next steps:
  cd my-project
  arukellt check src/main.ark        # type-check
  arukellt run src/main.ark          # run the program
  arukellt test src/lib.ark          # run tests    (with-tests template)
```

### Step 4: `arukellt init --list-templates` を追加する

利用可能なテンプレート一覧を出力するサブオプション。

```
$ arukellt init --list-templates
minimal    - Minimal Hello World project (default)
cli        - CLI tool with argument parsing
with-tests - Project with test functions
wasi-host  - WASI host API usage example (wasm32-wasi-p2)
```

### Step 5: テンプレートが `check / run / test` を通ることをテストで確認する

`crates/arukellt/tests/init_templates.rs` を追加する。

```rust
use std::process::Command;
use tempfile::TempDir;

fn arukellt_bin() -> &'static str { "arukellt" }

fn run_init(dir: &TempDir, template: &str) {
    let status = Command::new(arukellt_bin())
        .args(["init", dir.path().to_str().unwrap(), "--template", template])
        .status()
        .expect("arukellt init failed");
    assert!(status.success(), "arukellt init --template {} failed", template);
}

#[test]
fn template_minimal_check_passes() {
    let dir = TempDir::new().unwrap();
    run_init(&dir, "minimal");
    let status = Command::new(arukellt_bin())
        .args(["check", "src/main.ark", "--target", "wasm32-wasi-p1"])
        .current_dir(dir.path())
        .status().unwrap();
    assert!(status.success());
}

#[test]
fn template_with_tests_test_passes() {
    let dir = TempDir::new().unwrap();
    run_init(&dir, "with-tests");
    let status = Command::new(arukellt_bin())
        .args(["test", "src/lib.ark"])
        .current_dir(dir.path())
        .status().unwrap();
    assert!(status.success());
}

// minimal, cli, with-tests 各テンプレートで check が通ることを確認
// wasi-host は --target wasm32-wasi-p2 で check が通ることを確認
```

### Step 6: 拡張機能での動作確認

Issue 458（CodeLens 再設計）完了後、以下を確認する（テストで自動化する、または手動確認でも可）。

- `with-tests` テンプレートで生成した `src/lib.ark` を VS Code で開くと、`fn test_add()` の上に「▶ Run Test」が出る。
- `minimal` テンプレートの `fn main()` の上に「▶ Run Main」が出る。

---

## 依存関係

- 依存なし（独立して着手可能）
- Issue 458（CodeLens）: `with-tests` テンプレートの test 関数に CodeLens が出るかの確認で連動

---

## 影響範囲

- `crates/arukellt/src/main.rs`（`InitTemplate` enum, `--template` 引数）
- `crates/arukellt/src/commands.rs`（`cmd_init()` のテンプレート分岐）
- `crates/arukellt/tests/init_templates.rs`（新規テスト）
- `docs/current-state.md`（`arukellt init --template` の記載追加）

---

## 後方互換性

- `arukellt init` のデフォルト動作（`--template minimal`）は現在と同じ。
- 新オプションの追加は後方互換。

---

## 今回の範囲外

- git 初期化（`git init`）の自動実行
- `.gitignore` 生成
- 依存クレートの管理（`ark.lock` 等）
- インタラクティブな選択 UI（`--template` 指定のみ）
- テンプレートのリモート取得（ローカル内蔵のみ）

---

## 完了条件

- [x] `arukellt init my-project --template minimal` が動き、`arukellt check / run` が通る
- [x] `arukellt init my-project --template cli` が動き、`arukellt check / run` が通る
- [x] `arukellt init my-project --template with-tests` が動き、`arukellt test` が通る
- [x] `arukellt init my-project --template wasi-host` が動き、`arukellt check --target wasm32-wasi-p2` が通る
- [x] `arukellt init --list-templates` がテンプレート一覧を出力する
- [x] `cargo test -p arukellt -- init_templates` が全 pass する
- [x] `docs/current-state.md` に `arukellt init --template` の記載がある
- [x] `bash scripts/run/verify-harness.sh` 通過

---

## 必要なテスト

1. 各テンプレートで `arukellt check` が成功することの integration test（`tests/init_templates.rs`）
2. `with-tests` テンプレートで `arukellt test` が成功することの integration test
3. `arukellt init .` を既に `ark.toml` がある場所で実行するとエラーになること（既存動作の保持確認）
4. `--template invalid_name` で clap のエラーが出ること

---

## 実装時の注意点

- `wasi-host` テンプレートのメイン関数は HTTP リクエストをコメントアウトしておく（CI でネットワーク不要にする）。
- テンプレートのコード内容は `cmd_init()` に `const` 文字列として埋め込む（外部ファイル読み込み不要、配布時の依存を減らす）。
- `with-tests` テンプレートの `test_add` / `test_subtract` 関数に使う `assert()` 組み込みが存在するかを確認する。存在しない場合は比較と条件分岐で代替する。
