---
Status: done
Created: 2026-04-02
Updated: 2026-04-03
ID: 445
Track: runtime
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: yes
Priority: 1
---

# std: ":host::process: proc_exit を T1/T3 両エミッターに配線し stub を解除する"
`std: ":host::process::exit(code)` は現状 `exit(0)` が no-op、非ゼロコードは panic という Ark レベルの fallback 実装になっている。`proc_exit` は WASI 仕様上 T1/T3 の双方で定義されているが emitter で wired されていない。本 issue では以下を達成する。"
### Step 1: "AST/builtin 登録 (`crates/ark-typecheck/src/checker/builtins.rs`)"
- 両 builtin に `never_returns: true` 相当のフラグが存在する場合は設定する（型チェッカーがフロー解析に使う場合）。
### Step 2: "Ark stdlib 実装置き換え (`std/host/process.ark`)"
pub fn exit(code: i32) {
### Step 3: "T1 emitter 配線 (`crates/ark-wasm/src/emit/t1/mod.rs`)"
;; __intrinsic_process_exit(code: i32)
### Step 4: "T3 emitter 配線 (`crates/ark-wasm/src/emit/t3/mod.rs`)"
- WASI Preview 2 の `proc_exit` import path は `wasi: "cli/exit@0.2.0` の `exit` 関数（`(func (param i32))`）。"
(import "wasi: "cli/exit@0.2.0" "exit" (func $proc_exit (param i32)))"
### Step 5: "MIR ビルトイン展開 (`crates/ark-mir/src/` または emitter の lowering 層)"
- MIR レベルで `Builtin: ":ProcessExit` / `Builtin::ProcessAbort` に対応する variant を追加し、emitter がそれを受け取って Step 3/4 の命令列を生成できるようにする。"
- 既存の `Builtin: ":ClockNow` 等の降下パターンに倣う。"
### Step 6: "HOST_STUB_BUILTINS 確認 (`crates/arukellt/src/commands.rs`)"
- `process: ":exit` / `process::abort` の Status を `stub` → `available` に変更。"
### Step 7: fixture テスト追加
### Step 8: docs 更新
- Known Limitations の項目 1 "std: ":host::process::exit is a stub." を削除。"
- `std: ":process::Command` のような子プロセス起動 API"
- [x] `docs/capability-surface.md` の `process: ":exit` / `abort` が `stub → available` になっている"
- T3 の `proc_exit` import path は WASI P2 コンポーネントモデルの世界名 `wasi: "cli/exit@0.2.0` を使う。これは `wasi:cli/command` world に含まれる。wit ファイルや WIT generation を経由する場合は WIT 定義も確認すること。"
# std::host::process: proc_exit を T1/T3 両エミッターに配線し stub を解除する

## Summary

`std::host::process::exit(code)` は現状 `exit(0)` が no-op、非ゼロコードは panic という Ark レベルの fallback 実装になっている。`proc_exit` は WASI 仕様上 T1/T3 の双方で定義されているが emitter で wired されていない。本 issue では以下を達成する。

1. `__intrinsic_process_exit(i32)` と `__intrinsic_process_abort()` を builtin として登録し、T1/T3 emitter で `proc_exit` WASI call に落とす。
2. `abort()` は `proc_exit(134)` に統一（SIGABRT 相当の慣習コード）する。
3. `std/host/process.ark` の Ark 実装を intrinsic 呼び出しに置き換える。
4. fixture テストと runtime test を追加する。
5. `docs/capability-surface.md` と `docs/current-state.md` の status を `stub → available` に更新する。

---

## 矛盾と前提

- **capability-surface.md** は `exit`/`abort` のステータスを `stub` と記載。`proc_exit` は "not wired in the WASI emitter" と明記。
- **std/host/process.ark** コメント行 4 に "the WASI `proc_exit` import is not wired in yet" と記載。
- `crates/arukellt/src/runtime.rs` 行 102 / 178 は `I32Exit` を受け取るコードが既に存在し、`proc_exit` からの exit code 伝播はランタイム側で既に処理されている。
- **採用方針**: emitter が `proc_exit` を呼ぶよう修正すれば runtime 側は変更不要。T1/T3 共通で `proc_exit` が正しく機能する。`abort()` は `proc_exit(134)` として固定（panic への依存を廃止）。

---

## 詳細実装内容

### Step 1: AST/builtin 登録 (`crates/ark-typecheck/src/checker/builtins.rs`)

- `__intrinsic_process_exit` を `(i32) -> ()` の builtin として登録する。
  - 既存の `__intrinsic_*` パターンに倣い、`BUILTIN_FN_REGISTRY` または相当の登録箇所に追加する。
- `__intrinsic_process_abort` を `() -> ()` の builtin として登録する。
- 両 builtin に `never_returns: true` 相当のフラグが存在する場合は設定する（型チェッカーがフロー解析に使う場合）。

### Step 2: Ark stdlib 実装置き換え (`std/host/process.ark`)

現行の Ark 実装を削除し、intrinsic 呼び出しに置き換える。

```ark
/// Requests process termination with the given exit code.
pub fn exit(code: i32) {
    __intrinsic_process_exit(code)
}

/// Aborts execution immediately with exit code 134 (SIGABRT convention).
pub fn abort() {
    __intrinsic_process_abort()
}
```

ファイル先頭の警告コメント（"proc_exit is not wired in yet"）を削除する。

### Step 3: T1 emitter 配線 (`crates/ark-wasm/src/emit/t1/mod.rs`)

- `__intrinsic_process_exit` が MIR に現れた時に WASI `proc_exit` を呼ぶ wasm コードを emit する。
- T1 は WASI Preview 1 ベース。`proc_exit` は `wasi_unstable` か `wasi_snapshot_preview1` として import される。既存の WASI import パターン（`fd_write` 等）を参照して import section に追加する。
- emit する wasm 命令列（概略）:

  ```
  ;; __intrinsic_process_exit(code: i32)
  (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (param i32)))
  ;; 呼び出し箇所:
  local.get $code
  call $proc_exit
  unreachable  ;; proc_exit は noreturn
  ```

- `__intrinsic_process_abort` は `i32.const 134; call $proc_exit; unreachable` に展開する。

### Step 4: T3 emitter 配線 (`crates/ark-wasm/src/emit/t3/mod.rs`)

- WASI Preview 2 の `proc_exit` import path は `wasi:cli/exit@0.2.0` の `exit` 関数（`(func (param i32))`）。
- 既存 T3 WASI import セクション（stdio の `fd_write` 等）を参照して同様に追加する。
- emit する wasm 命令列（概略）:

  ```
  (import "wasi:cli/exit@0.2.0" "exit" (func $proc_exit (param i32)))
  ;; __intrinsic_process_exit:
  local.get $code
  call $proc_exit
  unreachable
  ```

- `__intrinsic_process_abort` は `i32.const 134; call $proc_exit; unreachable` に展開する。

### Step 5: MIR ビルトイン展開 (`crates/ark-mir/src/` または emitter の lowering 層)

- MIR レベルで `Builtin::ProcessExit` / `Builtin::ProcessAbort` に対応する variant を追加し、emitter がそれを受け取って Step 3/4 の命令列を生成できるようにする。
- 既存の `Builtin::ClockNow` 等の降下パターンに倣う。

### Step 6: HOST_STUB_BUILTINS 確認 (`crates/arukellt/src/commands.rs`)

- `process::exit` / `process::abort` は現状 `HOST_STUB_BUILTINS` に含まれていない（sockets のみ）。
- 実装完了後も誤って stub 扱いされないことを確認する（追加しない・既存エントリに混入させない）。

### Step 7: fixture テスト追加

以下の fixture を `tests/fixtures/` 以下に追加する（manifest-driven fixture 形式、既存の `*.ark` + `*.expected` パターンに倣う）。

| fixture ファイル | 内容 | 期待値 |
|---|---|---|
| `host/process/exit_zero.ark` | `exit(0)` を呼ぶ | exit code 0 |
| `host/process/exit_nonzero.ark` | `exit(1)` を呼ぶ | exit code 1 |
| `host/process/exit_code_127.ark` | `exit(127)` | exit code 127 |
| `host/process/abort.ark` | `abort()` を呼ぶ | exit code 134 |
| `host/process/exit_from_branch.ark` | 条件分岐内で `exit(2)` | exit code 2 |

各 fixture を manifest (`tests/fixtures/manifest.toml` 等) に追加し、`run = true`、期待 exit code を設定する。

### Step 8: docs 更新

- `docs/capability-surface.md`:
  - `process::exit` / `process::abort` の Status を `stub` → `available` に変更。
  - WASI import 列に `proc_exit` を記載。
  - Known Limitations の項目 1 "std::host::process::exit is a stub." を削除。
- `docs/current-state.md`:
  - `std::host::process` が利用可能になった旨を Recent Milestones に追加。
- `std/manifest.toml` の process モジュールエントリのステータスフィールドがあれば `stub → stable` に変更する。

---

## 依存関係

- 他 issue への依存なし。T1/T3 emitter は独立して変更可能。
- Issue 448（capability target-gating）は本 issue と並行可能。本 issue が完了すると `capability-surface.md` の更新が両 issue で重複しないよう注意する。

---

## 影響範囲

- `std/host/process.ark`
- `crates/ark-typecheck/src/checker/builtins.rs`
- `crates/ark-wasm/src/emit/t1/mod.rs`
- `crates/ark-wasm/src/emit/t3/mod.rs`
- `crates/ark-mir/src/` （Builtin variant 追加）
- `tests/fixtures/host/process/` （新規）
- `tests/fixtures/manifest.toml` または相当の fixture マニフェスト
- `docs/capability-surface.md`
- `docs/current-state.md`
- `std/manifest.toml`

---

## 後方互換性・移行影響

- 既存コードで `exit(0)` を呼んでいるプログラムは現状 no-op で自然終了している。実装後は `proc_exit(0)` 経由になるが、ランタイムの `I32Exit` 処理は既に存在するため行動変化なし。
- 既存コードで `exit(非ゼロ)` を呼んでいるプログラムは panic していたが、実装後は正しい exit code を返すようになる（**破壊的変化だが正しい挙動への修正**）。
- `abort()` は panic から `proc_exit(134)` に変わる。panic の際のメッセージが失われるが、`abort()` の意味論としては正しい。

---

## 今回の範囲外（明確な非対象）

- `--deny-process` フラグの追加（Issue 448 スコープ）
- exit code を Result 型で返す高レベル API の追加
- `std::process::Command` のような子プロセス起動 API
- T2 (freestanding) / T4 (native) バックエンドへの配線

---

## 完了条件

- [x] `__intrinsic_process_exit` / `__intrinsic_process_abort` が T1/T3 emitter で `proc_exit` に落ちる（wasm-tools validate 通過）
- [x] fixture `exit_zero.ark` / `exit_nonzero.ark` / `abort.ark` が T1/T3 双方で期待 exit code を返す
- [x] `bash scripts/run/verify-harness.sh` が 13/13 pass のまま（退行なし）
- [x] `docs/capability-surface.md` の `process::exit` / `abort` が `stub → available` になっている
- [x] `docs/current-state.md` に反映済み
- [x] `std/host/process.ark` の "proc_exit is not wired in yet" コメントが削除されている

---

## 必要なテスト

1. **fixture テスト**（Step 7 の全 fixture）: T1/T3 両方で exit code を検証。
2. **cargo test --workspace**: builtin 登録の unit test（既存 builtin テストパターンに倣い、`process_exit` が型チェッカーに見える）。
3. **wasm-tools validate**: 生成された wasm に `proc_exit` import が正しく入り、validate 通過。
4. **regression テスト**: 既存の `hello.ark` 等が引き続き exit code 0 で通ること。

---

## 実装時の注意点

- T3 の `proc_exit` import path は WASI P2 コンポーネントモデルの世界名 `wasi:cli/exit@0.2.0` を使う。これは `wasi:cli/command` world に含まれる。wit ファイルや WIT generation を経由する場合は WIT 定義も確認すること。
- `proc_exit` は `noreturn` であるため、Wasm バリデーターは call の後に到達不可能な命令（`unreachable`）を要求する。emit 後に `unreachable` を必ず付加すること。
- T1 の import module 名が `wasi_snapshot_preview1` か `wasi_unstable` かは既存の T1 WASI import 定義を参照して統一すること（混在させない）。