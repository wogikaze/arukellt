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

## Audit notes (#615 implementation slice)

This section records the empirical audit performed during the #615
implementation slice and is intentionally kept terse — it is the
auditable trail that closes acceptance criteria #2, #3, #4.

### CLI structured-error sites (criterion #2 — already in compliance)

The `arukellt` CLI shim (`crates/arukellt/src/main.rs`) was audited for
bare `.unwrap()` / `.expect()` / `panic!` on user-input paths. The two
genuinely fallible host-interaction paths already use the structured
`eprintln!("...") + ExitCode::from(...)` pattern required by §4:

- `crates/arukellt/src/main.rs:135-145` — selfhost wasm not found.
  Emits a multi-line `arukellt: selfhost wasm not found.` message
  pointing the user at the build script and the `ARUKELLT_SELFHOST_WASM`
  override, then returns `ExitCode::from(127)`.
- `crates/arukellt/src/main.rs:181-187` — `wasmtime` binary not on
  `PATH`. Emits `arukellt: failed to invoke wasmtime (is it on PATH?)`
  with an install hint and returns `ExitCode::from(127)`.

The other `unwrap_or` / `unwrap_or_else` call sites in the shim are on
infallible-fallback paths (default cwd, default exit code clamping) and
are not user-input failures, so §4 does not require them to change.

Criterion #2 is therefore satisfied by **existing** code; no rewrite was
needed for this slice.

### ICE output format (criterion #4 — implemented)

`crates/arukellt/src/main.rs` now provides:

- `report_ice(reason: &str) -> ExitCode` — emits the policy-mandated
  `[BUG] internal compiler error: <reason>` line, the issue-report URL,
  and the `RUST_BACKTRACE=1` hint, then returns `ExitCode::from(101)`.
- `classify_child_exit(code: Option<i32>) -> Result<u8, String>` —
  classifies the wasmtime child's exit. Signal-killed (`None`) and
  shell-encoded fatal-signal exits (132/133/134/136/138/139) are routed
  to `report_ice`. Code `101` is passed through verbatim, trusting any
  upstream `[BUG]` line emitted by the selfhost compiler itself so the
  shim never double-prepends.
- `ARUKELLT_ICE_SMOKE` env-var hook — when set, `main()` short-circuits
  to `report_ice(...)` before launching wasmtime. This makes the policy
  format end-to-end testable without having to actually crash the
  compiler.

Smoke test (recorded for the #615 slice):

```text
$ ARUKELLT_ICE_SMOKE=1 ./target/release/arukellt --help
[BUG] internal compiler error: ARUKELLT_ICE_SMOKE hook fired (synthetic ICE for policy smoke test)
  please report this at: https://github.com/wogikaze/arukellt/issues/new
  hint: re-run with RUST_BACKTRACE=1 for a full trace
$ echo $?
101
```

Unit-test coverage lives at the bottom of `crates/arukellt/src/main.rs`
(`mod tests`) and asserts each branch of `classify_child_exit`.

### Compiler-side assertion → diagnostic conversion (criterion #3)

**Status: blocked / not applicable in the current tree.**

The work order for this slice asked for at least one compiler-side
`panic!` / `unreachable!` / `assert!` reachable from user input to be
converted into a structured diagnostic. An audit of the current
compiler-core surface found nothing eligible:

- The legacy Rust compiler driver (`ark-driver`) and MIR crate
  (`ark-mir`) were retired in #560 / #561, so the historical Rust call
  sites that did contain raw `panic!` / `unreachable!` no longer exist
  in the tree.
- The active selfhost compiler (`src/compiler/*.ark`) does not have a
  Rust-style `panic!` macro at all — by construction it cannot use one.
  The only `assert` / `assert_eq` etc. tokens in `src/compiler/` are
  *builtin name registrations* in `resolver.ark` (so user code can call
  them), not actual call sites in the compiler.
- The remaining Rust compiler-core crates still in the workspace
  (`ark-lexer`, `ark-parser`, `ark-resolve`, `ark-typecheck`,
  `ark-hir`, `ark-diagnostics`) contain only:
  - `assert!` / `assert_eq!` inside `#[cfg(test)]` modules (not user
    paths),
  - two `unreachable!()` in `crates/ark-lexer/src/scan.rs:265,282`
    inside `lex_doc_comment`. Both are guarded by the caller's
    pre-check that the next two bytes are `///` or `//!`, so they are
    structurally unreachable from any user input.
  - a `panic!` in `crates/ark-resolve/build.rs:13` that runs at *build*
    time, not at user-compile time.

There is therefore no compiler-side assertion in the current tree that
fires on valid user input and that this slice could honestly demote to
a structured diagnostic. Per the work order's `STOP_IF` clause this
criterion is recorded as **blocked-not-applicable** and is folded into
the broader compiler-core hygiene tracked separately.

If a future change reintroduces a Rust-side compiler invariant on the
user-input path (e.g. a new MIR validator written in Rust rather than
selfhost), it should be added under this section as a new conversion
candidate.

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
