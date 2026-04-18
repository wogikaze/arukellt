# std::host::sockets::connect: WASI P2 最小実装・T3 限定 compile-time 明確化

**Status**: done
**Created**: 2026-04-02
**Updated**: 2026-04-18
**ID**: 447
**Depends on**: none
**Track**: runtime
**Blocks v1 exit**: yes
**Priority**: 3

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: tests/fixtures/host/sockets and target_gating/t1_import_sockets fixtures exist, HOST_STUB_BUILTINS empty

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Audit normalization — 2026-04-18

The `Reopened by audit` section below is historical. Current repo evidence still
supports `done`:

- `tests/fixtures/host/sockets/` contains connect refusal / DNS failure proof
- `tests/fixtures/target_gating/t1_import_sockets.ark` covers the T1 compile-time gate
- `crates/arukellt/src/commands.rs` no longer keeps sockets in `HOST_STUB_BUILTINS`
- `docs/stdlib/modules/sockets.md` describes the current T3-only availability contract

This issue remains in `issues/done/`; the earlier reopen note is retained only as audit history.

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/447-std-host-sockets-implementation.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`std::host::sockets::connect(host: String, port: i32) -> Result<i32, String>` は現在 `HOST_STUB_BUILTINS` により compile-time hard error になっている。本 issue では TCP connect の最小実装を T3 (wasm32-wasi-p2) 向けに通し、以下を達成する。

1. `__intrinsic_sockets_connect` を Wasmtime linker 経由で実装する（T3 の Wasmtime runtime 上での最小 TCP connect）。
2. T1 (wasm32-wasi-p1) で `sockets::connect` を import した場合に **compile-time error** が出ることを明確化する（T1 では WASI P2 sockets が存在しないため）。
3. T3 実装完了後に `HOST_STUB_BUILTINS` から sockets エントリを削除する。
4. 接続成功・失敗の fixture を追加する。
5. `docs/capability-surface.md` の sockets セクションを更新する。

---

## 矛盾と前提

- **capability-surface.md**: `connect` のステータスは `stub`、T3 限定。「Usage is rejected at compile time.」と記述。
- **commands.rs 行 1162**: `HOST_STUB_BUILTINS = ["sockets_connect", "__intrinsic_sockets_connect"]` — compile-time hard error が実装済み。
- **矛盾なし**: docs と実装が一致している。実装後に docs の stub 表記と HOST_STUB_BUILTINS エントリを両方削除する。
- **T3 限定の根拠**: WASI Preview 1 (T1) には `wasi:sockets` が存在しない。T1 で import しようとした場合の診断は Issue 448（target-gating）で対処するが、本 issue では HOST_STUB_BUILTINS による T1/T3 共通の compile-time block を T3 実装完了後に解除し、代わりに T1 向けの target-gate エラーを Issue 448 と調整する。

---

## 詳細実装内容

### Step 1: Wasmtime linker への sockets_connect 登録 (`crates/arukellt/src/runtime.rs`)

T3 の `run_wasm_gc` 経路（または共通 linker 設定）に `sockets_connect` を登録する。

```rust
// sockets_connect(host_ptr: i32, host_len: i32, port: i32, result_ptr: i32) -> i32
linker.func_wrap(
    "arukellt_host",
    "sockets_connect",
    |mut caller: Caller<'_, WasiCtx>, host_ptr: i32, host_len: i32, port: i32, result_ptr: i32| -> i32 {
        let host = read_string_from_memory(&mut caller, host_ptr as usize, host_len as usize);
        match tcp_connect_impl(&host, port as u16) {
            Ok(fd) => {
                // write fd to result_ptr, return 0 (success)
                write_i32_to_memory(&mut caller, result_ptr as usize, fd);
                0
            }
            Err(msg) => {
                // write error string, return -1
                write_string_to_result(&mut caller, result_ptr as usize, &msg);
                -1
            }
        }
    },
)?;
```

実際のメモリ読み書きは既存の `http_get` / `http_request` 登録のパターンに倣う。

### Step 2: TCP connect 実装関数 (`crates/arukellt/src/runtime.rs`)

```rust
fn tcp_connect_impl(host: &str, port: u16) -> Result<i32, String> {
    use std::net::TcpStream;
    let addr = format!("{}:{}", host, port);
    match TcpStream::connect(&addr) {
        Ok(_stream) => {
            // 接続成功。fd として 3 を返す（最小実装: 実際の fd 管理は将来拡張）
            Ok(3)
        }
        Err(e) => Err(format!("connect: {}: {}", addr, e)),
    }
}
```

**注意**: 最小実装として `TcpStream::connect` が成功したことを確認するのみ。返す fd の値は 3 固定（実際のソケット管理は将来拡張とする）。この制約を docs に明記する。

### Step 3: エラーマッピング仕様の確定

`connect(host, port) -> Result<i32, String>` のエラー文字列を固定する。

| 状況 | `Err(String)` の値 |
|---|---|
| DNS 解決失敗 | `"connect: <host>:<port>: dns not found"` |
| 接続拒否 | `"connect: <host>:<port>: connection refused"` |
| タイムアウト | `"connect: <host>:<port>: timed out"` |
| その他 | `"connect: <host>:<port>: <os_error>"` |

この仕様を `docs/capability-surface.md` の sockets セクションに追記する。

### Step 4: HOST_STUB_BUILTINS から削除 (`crates/arukellt/src/commands.rs`)

T3 実装完了後、`HOST_STUB_BUILTINS` から `"sockets_connect"` と `"__intrinsic_sockets_connect"` を削除する。

この削除と同時に、T1 target で `std::host::sockets` を import した場合に Issue 448 の target-gating 診断が出るように調整する。Issue 448 が未完了の場合は、T1 向けに限定した別の HOST_STUB check を残す（T1 のみ stub 扱い）か、Issue 448 の完了を待つかを判断する。

**採用方針**: Issue 448 と本 issue は並行可能だが、HOST_STUB_BUILTINS 削除のタイミングは Issue 448 の T1 target-gate が完成したタイミングに合わせる。Issue 448 が遅れる場合は、HOST_STUB_BUILTINS に `T1 only` のエントリとして残し、T3 では通過させる分岐を追加する方法もある（実装時に判断）。

### Step 5: T3 限定であることの compile-time 診断

Issue 448 の完了に依存するが、以下の動作を確保する。

- T1 で `std::host::sockets` を import → `E0XXX: std::host::sockets は wasm32-wasi-p2 (T3) 専用です。ターゲットを --target wasm32-wasi-p2 に変更してください。` というコンパイルエラーを出す。
- T3 で `std::host::sockets` を import → コンパイル通過（HOST_STUB_BUILTINS に含まれない）。

このエラーは Issue 448 が担当するが、本 issue 完了条件としても「T1 で sockets を使った時に有用な診断が出る」ことを要求する。

### Step 6: fixture テスト追加

| fixture ファイル | ターゲット | CI | 内容 | 期待値 |
|---|---|---|---|---|
| `host/sockets/connect_refused.ark` | T3 | CI 可 | `connect("127.0.0.1", 1)` — ポート 1 は通常拒否 | `Err("connect: 127.0.0.1:1: connection refused")` |
| `host/sockets/connect_dns_fail.ark` | T3 | CI 可 | `connect("invalid.example.invalid", 80)` | `Err("connect: ...")` で始まる |
| `host/sockets/connect_success.ark` | T3 | ローカルのみ | ローカルサーバーへの接続 | `Ok(3)` |
| `host/sockets/t1_target_error.ark` | T1 | CI 可 | T1 target で sockets import → compile error | compile error（Issue 448 完了後に追加） |

### Step 7: docs 更新

- `docs/capability-surface.md`:
  - `sockets::connect` の Status を `stub → available (T3 only, minimum implementation)`.
  - "Usage is rejected at compile time" を "T1 で import した場合は compile-time error になる。T3 では利用可能（最小実装: 接続確立のみ、fd 管理は将来拡張）。" に変更。
  - エラーマッピング仕様を追記。
  - Known Limitations の項目 3 の sockets 部分を更新。
- `docs/current-state.md`: Recent Milestones に `std::host::sockets::connect` が T3 で利用可能になった旨を追記。
- `std/manifest.toml`: sockets モジュールの stability を `experimental` のまま維持（fd 管理が完全でないため）。

---

## 依存関係

- Issue 448（target-gating）と協調が必要（T1 での compile-time エラー）。448 の進捗次第で HOST_STUB_BUILTINS 削除タイミングを調整する。
- Issue 446（http）とは独立して進行可能。

---

## 影響範囲

- `crates/arukellt/src/runtime.rs`
- `crates/arukellt/src/commands.rs`（HOST_STUB_BUILTINS 変更）
- `tests/fixtures/host/sockets/`（新規）
- `docs/capability-surface.md`
- `docs/current-state.md`
- `std/manifest.toml`

---

## 後方互換性・移行影響

- 現状: sockets を使うプログラムは compile-time hard error。実装後: T3 でコンパイル通過し実行可能になる。T1 での挙動は変わらない（Issue 448 完了まで HOST_STUB_BUILTINS か target-gate により継続して block）。
- **Breaking change**: なし（compile-time error からの解除は機能追加であり破壊ではない）。

---

## 今回の範囲外（明確な非対象）

- ソケット fd の実際の管理（read/write/close）— 「接続確立のみ」が最小完了条件
- UDP ソケット
- `--deny-sockets` capability flag
- WASI P2 native `wasi:sockets/tcp` component path（Bridge mode の制約内で Wasmtime linker を使う）
- T4 (native) バックエンドへの配線

---

## 完了条件

- [x] T3 で `sockets::connect("127.0.0.1", 1)` が `Err(...)` を返す fixture が CI pass
- [x] T3 で `sockets::connect` が compile 通過する（HOST_STUB_BUILTINS から削除）
- [x] T1 で `sockets::connect` を使うと compile-time error または明確な diagnostics が出る
- [x] `docs/capability-surface.md` の sockets Status が更新されている
- [x] `bash scripts/run/verify-harness.sh` が 13/13 pass

---

## 必要なテスト

1. `host/sockets/connect_refused.ark`（T3 CI）: 接続拒否で `Err` が返る
2. `host/sockets/connect_dns_fail.ark`（T3 CI）: DNS 失敗で `Err` が返る
3. HOST_STUB_BUILTINS 削除後の regression: 既存の "sockets stub" 検証 fixture（あれば）が失敗しないことを確認

---

## 実装時の注意点

- `TcpStream::connect` はデフォルトで接続タイムアウトがない。`TcpStream::connect_timeout` を使い、タイムアウトを設定すること（例: 5秒）。
- ポート番号の型変換: Ark の `i32` から Rust の `u16` への変換で負数・65536 超の場合の処理を追加する（範囲外は `Err("connect: invalid port <n>")` を返す）。
- 最小実装で fd を 3 固定にする場合は、コメントで「将来の fd 管理実装のプレースホルダー」と明記し、TODO を残す。
