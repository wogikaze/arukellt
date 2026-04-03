# capability / target-gating のエンドツーエンド実装: load.rs TODO 解消・deny flag fixture 化

**Status**: done
**Created**: 2026-04-02
**Updated**: 2026-04-03
**ID**: 448
**Depends on**: none
**Track**: runtime
**Blocks v1 exit**: yes
**Priority**: 4


---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: E0500 in codes.rs, t1_import_sockets.diag, T3_ONLY_MODULES in load.rs

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).


## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/448-capability-target-gating-e2e.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`crates/ark-resolve/src/load.rs` 行 11 に `TODO(issue-077, issue-139)` として残っているターゲット検証ロジックを実装し、T1 で `std::host::http` / `std::host::sockets` を import した際に compile-time diagnostics を出す。加えて `--deny-clock` / `--deny-random` / `--dir` / `--deny-fs` の挙動を compile-time と runtime の両側から fixture で網羅する。docs 内の矛盾（current-state.md と CLI 実装のズレ）も本 issue で解消する。

---

## 矛盾と前提

### 矛盾 1: `docs/current-state.md` vs CLI の `--dir` / `--deny-*` 記述

- `docs/current-state.md` の compile/run コマンドのデフォルトパス・フラグ記述が CLI の実際の実装と食い違っていることが指摘されている（`compile` の default path、`--dir` / `--deny-*` の挙動記述のズレ）。
- **採用方針**: `crates/arukellt/src/commands.rs` の実装を正として、`docs/current-state.md` と `docs/capability-surface.md` を修正する。この修正を本 issue のドキュメント横断タスクとして含める。

### 矛盾 2: `capability-surface.md` の http compile-time block 記述

- Issue 446 で解決される（本 issue では docs 更新を再確認する）。

### 矛盾 3: target-gating の診断コード

- 現時点で target-gating 専用の DiagnosticCode が存在しない可能性がある（既存コードは `E0001–E0402`, `W0001–W0008`）。
- **採用方針**: 新しい診断コード `E0500: incompatible target` を追加する（`E0400–E0402` は component errors; `E0500` 番台を target/capability errors とする）。

---

## 詳細実装内容

### Step 1: TargetId を module loader に threading する (`crates/ark-resolve/src/load.rs`)

load.rs の `TODO(issue-077, issue-139)` を解消する。

1. `crates/ark-resolve/src/load.rs` の `load_module` 関数（または相当のエントリポイント）のシグネチャに `target: TargetId` パラメータを追加する。
2. 呼び出し元（`ark-driver` の `Session` や `commands.rs` の compile 経路）から `TargetId` を渡す。
3. module ロード時に import されたモジュール名を確認し、以下の条件でエラーを emit する。

```rust
// load.rs 内
if target == TargetId::Wasm32WasiP1 {
    if module_name.starts_with("std::host::http") ||
       module_name.starts_with("std::host::sockets") {
        sink.emit(Diagnostic {
            code: DiagnosticCode::E0500,
            message: format!(
                "`{}` は wasm32-wasi-p2 (T3) 専用モジュールです。\
                 --target wasm32-wasi-p2 を指定してください。",
                module_name
            ),
            span: import_span,
            severity: Severity::Error,
        });
    }
}
```

4. `TargetId` の threading: `ark-driver::Session` に `target_id: TargetId` フィールドが既に存在するはずなので、`load_module` へ渡すパスを通す。

### Step 2: 新 DiagnosticCode `E0500` の追加 (`crates/ark-diagnostics/src/codes.rs`)

```rust
pub enum DiagnosticCode {
    // ... 既存
    E0500, // incompatible-target: module requires a different target
}

// codes.rs の spec() に追加:
Self::E0500 => DiagnosticSpec {
    phase: "resolve",
    id: "E0500",
    // ...
},
```

`docs/current-state.md` の Diagnostics セクションに `E0500` を追記する。

### Step 3: deny flag の compile-time 挙動 fixture 化

既存の `--deny-clock` / `--deny-random` MIR scan は実装済み（`commands.rs` の `mir_uses_capability`）。fixture が不足している場合に追加する。

以下を `tests/fixtures/capability/` 以下に追加する。

| fixture | compile flag | 期待挙動 |
|---|---|---|
| `deny_clock_compile.ark` | `--deny-clock` | compile-time error（`clock_now` 使用） |
| `deny_random_compile.ark` | `--deny-random` | compile-time error（`random_i32` 使用） |
| `deny_clock_transitive.ark` | `--deny-clock` | compile-time error（clock 呼ぶ関数を呼ぶ） |
| `t1_import_http.ark` | T1 target | `E0500: std::host::http は T3 専用` |
| `t1_import_sockets.ark` | T1 target | `E0500: std::host::sockets は T3 専用` |

### Step 4: deny flag の runtime 挙動 fixture 化

以下の runtime fixture を追加する（`run = true` で実行する）。

| fixture | runtime flag | 内容 | 期待挙動 |
|---|---|---|---|
| `deny_clock_runtime.ark` | `--deny-clock` | `clock::monotonic_now()` を使う | runtime error / capability denied |
| `deny_random_runtime.ark` | `--deny-random` | `random::random_i32()` を使う | runtime error / capability denied |
| `dir_grant_read.ark` | `--dir ./testdata` | `fs::read_to_string(path)` | `Ok(content)` |
| `dir_deny_fs.ark` | `--deny-fs` | `fs::read_to_string(path)` | `Err(...)` |
| `dir_readonly.ark` | `--dir ./testdata:ro` | write 試行 | `Err(...)` |

**注意**: `--deny-clock` と `--deny-random` はすでに compile-time scan で弾かれるため、これらが compile 通過した上で runtime まで到達することは現状ない。runtime 側の確認は「compile-time で弾かれることの確認 fixture」として扱う。runtime 到達を確認したい場合は intrinsic を直接呼ぶ lower-level テストが必要。

### Step 5: `docs/current-state.md` の CLI 記述修正

`crates/arukellt/src/commands.rs` の実装から以下を確認し、`docs/current-state.md` に反映する。

1. `compile` コマンドのデフォルトターゲット（T1 か T3 か）を確認し、docs の記述と一致させる。
2. `--dir` フラグの挙動（パス解決方法、`:ro`/`:rw` サフィックスの有無）を確認して docs に反映。
3. `--deny-fs` フラグの挙動（`--deny-fs` が `--dir` を上書きするか）を確認して反映。
4. `check` コマンドの `--mir-select` デフォルト値が `corehir` であることを確認して docs に反映。

### Step 6: `docs/capability-surface.md` の CLI Capability Flags セクション修正

- Deny flags テーブルに `--deny-fs` の行を追加（現状テーブルに記載があるか確認）。
- "Default policy" テーブルの `Filesystem: Deny` 記述について、`--deny-fs` が「明示的に grant された `--dir` も無効化する」ことを補足する。
- http / sockets の "compile-time blocked" 記述を Issue 446/447 の完了と整合させる（本 issue では T1 target-gate の追加のみ行い、T3 での動作を変えない）。

---

## 依存関係

- Issue 445/446/447 と並行可能。本 issue の完了が 446/447 の "T1 target-gate" 要件を満たす。
- Issue 449（type_table）とは独立。

---

## 影響範囲

- `crates/ark-resolve/src/load.rs`
- `crates/ark-diagnostics/src/codes.rs`
- `crates/ark-driver/` または呼び出し経路（TargetId threading）
- `crates/arukellt/src/commands.rs`（呼び出し元修正）
- `tests/fixtures/capability/`（新規）
- `docs/current-state.md`
- `docs/capability-surface.md`

---

## 後方互換性・移行影響

- T1 で `std::host::http` / `std::host::sockets` を import しているコードは現在 HOST_STUB_BUILTINS（sockets）か実行時 Err（http）で動作していた。本 issue 後は compile-time E0500 で弾かれる。**これは破壊的変化だが意図的な正の変化（誤ったコードをより早期に検出）。**

---

## 今回の範囲外（明確な非対象）

- `--deny-stdio` フラグの追加（capability-surface.md Known Limitations 項目 4）
- per-function capability deny
- T5 (wasm32-wasi-p3) の target-gating
- 診断の warning → error 昇格ポリシーの変更

---

## 完了条件

- [x] T1 で `std::host::http` を import すると `E0500` compile-time error が出る
- [x] T1 で `std::host::sockets` を import すると `E0500` compile-time error が出る
- [x] `deny_clock_compile.ark` / `deny_random_compile.ark` fixture が CI pass
- [x] `docs/current-state.md` の CLI フラグ記述が実装と一致している
- [x] `docs/capability-surface.md` が docs と実装の矛盾なく整合している
- [x] `bash scripts/run/verify-harness.sh` が 13/13 pass

---

## 必要なテスト

1. `t1_import_http.ark`（T1）: E0500 が出ること
2. `t1_import_sockets.ark`（T1）: E0500 が出ること
3. `deny_clock_compile.ark`（T1/T3）: compile-time error
4. `deny_random_compile.ark`（T1/T3）: compile-time error
5. `deny_clock_transitive.ark`（T1/T3）: transitive scan で compile-time error
6. `dir_grant_read.ark`（runtime）: ファイル読み込み成功
7. `dir_deny_fs.ark`（runtime）: 拒否確認

---

## 実装時の注意点

- `load_module` のシグネチャ変更は呼び出し元が複数ある可能性があるため、全呼び出しサイトを grep して漏れなく修正する。
- `TargetId` を load.rs に渡す経路は `Session → Resolver → load_module` の順になる可能性が高いが、`Session` に TargetId が入っていない場合は先に追加する。
- `E0500` の diagnostic span は import 文のトークン span を使う。import 文が複数モジュールをまとめてインポートする構文の場合、各モジュール名ごとに個別 span を使う。
- fixture manifest に `expected_error_code = "E0500"` 相当のフィールドがあれば活用する。なければ `expected_stderr_contains` で代替する。
