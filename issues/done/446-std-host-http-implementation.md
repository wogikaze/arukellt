---
Status: done
Created: 2026-04-02
Updated: 2026-04-18
ID: 446
Track: runtime
Depends on: none
Orchestration class: implementation-ready
---
# std::host::http: T1 動作確認・T3 WASI P2 最小実装・stub 解除
**Blocks v1 exit**: yes
**Priority**: 2

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: runtime.rs has register_http_host_fns, http_get_impl; HOST_STUB_BUILTINS is empty

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Audit normalization — 2026-04-18

The `Reopened by audit` section below is a historical note from an earlier stale-state
pass. Current repo evidence still supports `done`:

- `crates/arukellt/src/runtime.rs` registers HTTP host functions and implements
  `http_get_impl` / `http_request_impl`
- `crates/arukellt/src/commands.rs` no longer treats HTTP builtins as host stubs
- `tests/fixtures/host/http/` contains host HTTP fixtures
- `docs/current-state.md` and `docs/capability-surface.md` describe `std::host::http`
  as available rather than compile-time blocked

The issue remains in `issues/done/`; the stale reopen note is kept only as historical audit context.

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/446-std-host-http-implementation.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`std::host::http::get(url)` / `http::request(method, url, body)` は manifest 上は T3 限定で可視だが実装は stub。`docs/capability-surface.md` は両関数を compile-time blocked と記述しているが、実際の `HOST_STUB_BUILTINS` リスト（`commands.rs` 行 1162）には http が含まれておらず、T1 (Wasmtime linker) では `http_get_impl` / `http_request_impl` が `runtime.rs` に既に実装されている。

本 issue の目的は：

1. **矛盾を解消する**：T1 経路の実際の動作（Wasmtime linker で HTTP 実行可能）を確認し、docs と実装を整合させる。
2. **T3 WASI P2 経路の最小 happy path を通す**：`wasi:http/outgoing-handler` 経由で `get(url)` が成功する e2e fixture を追加する。
3. **失敗時のエラーマッピング方針を確定する**：ネットワーク不到達・4xx/5xx をどの `Err(String)` 値で返すかを仕様として固定する。
4. **host_stub 扱いを解除する**（T3 実装完了後）。
5. `docs/capability-surface.md` の Status 記述と compile-time enforcement の事実を一致させる。

---

## 矛盾と前提

### 矛盾 1: 「compile-time blocked」の記述と実際の挙動

- **capability-surface.md** "Host Stub Enforcement" 節: `http_request`, `http_get` がリストされ「hard error になる」と記述。
- **commands.rs 行 1162** の `HOST_STUB_BUILTINS`: `["sockets_connect", "__intrinsic_sockets_connect"]` のみ。http は含まれない。
- **runtime.rs 行 189–259**: Wasmtime linker に `http_get` / `http_request` を登録するコードが存在し、`http_get_impl` / `http_request_impl` として実際の HTTP クライアント実装（`ureq` 等）が存在する可能性が高い。
- **採用方針**: 実コードを正とする。**T1 経路では HTTP が既に動作する可能性が高い**。capability-surface.md の "compile-time blocked" 記述は T3（stub のみ）に対して有効だったが現状を正確に反映していない。本 issue で docs を実態に合わせて修正する。

### 矛盾 2: T1/T3 の availability 表記

- 現状の Target Compatibility Matrix: `http::request` / `http::get` は T1 列が `—`（利用不可）、T3 列が `stub`。
- しかし runtime.rs には T1 向け Wasmtime linker 登録が存在する。
- **採用方針**: T1 の linker 実装を確認・動作確認後、T1 列を `available`（またはその実態）に修正する。

---

## 詳細実装内容

### Step 1: T1 経路の動作確認と fixes

1. `crates/arukellt/src/runtime.rs` の `http_get_impl` / `http_request_impl` の実装内容を確認する。
   - HTTP クライアントライブラリ（`ureq`, `reqwest` 等）が `Cargo.toml` に依存として存在するか確認する。
   - エラー時に返す `Err(String)` の内容を確認する（"connection refused", "timeout", "HTTP 404" 等のフォーマットが曖昧な場合は統一する）。
2. fixture `tests/fixtures/host/http/get_success.ark` を T1 ターゲットで通す。
   - ネットワーク依存 fixture は CI で動かせない場合があるため、「ローカルのみ・CI skip」タグを manifest に設定する方針を採用する（既存の skip 機構を流用）。
   - 代替として: httpbin.org や localhost の mock サーバーを立てる方法があるが、fixture harness の制約を確認の上決定する。
3. T1 用の error mapping を `docs/capability-surface.md` の仕様として明文化する（以下参照）。

### Step 2: エラーマッピング仕様の確定

`get(url) -> Result<String, String>` および `request(method, url, body) -> Result<String, String>` のエラー文字列を以下に固定する。

| 状況 | `Err(String)` の値 |
|---|---|
| DNS 解決失敗 | `"dns: <hostname>: not found"` |
| 接続タイムアウト | `"timeout: <url>"` |
| 接続拒否 | `"connection refused: <url>"` |
| HTTP 4xx | `"http <status_code>: <url>"` |
| HTTP 5xx | `"http <status_code>: <url>"` |
| その他 | `"error: <message>"` |

この仕様を `docs/capability-surface.md` の `std::host::http` セクションに追記する。

### Step 3: T3 WASI P2 component 経路の実装

T3 (wasm32-wasi-p2) の component モードで `wasi:http/outgoing-handler` を呼ぶ実装を追加する。

- `std/host/http.ark` に `__intrinsic_http_get_t3` / `__intrinsic_http_request_t3` を定義し、T3 では `wasi:http` 経由に dispatch する。
  - ただし、WIT bindings 生成が完了するまでの過渡期対応として、T3 ビルドでも Wasmtime linker 経由（T1 と同一実装）を使うか、WIT 直接呼び出しを使うかを決定する必要がある。
  - **採用方針**: T3 の current state は "bridge mode" であり、完全な component path が未完成。本 issue では **T3 でも Wasmtime linker 経由で動作すること** を最小完了条件とし、native WASI P2 component パスは将来拡張とする（明確に非対象とする）。
- `crates/arukellt/src/runtime.rs` の `run_wasm_gc` 経路（T3）で http linker 登録が呼ばれていることを確認する。呼ばれていなければ `register_http_host_fns` 等を追加する。

### Step 4: HOST_STUB_BUILTINS の修正と compile-time enforcement

現状: http は `HOST_STUB_BUILTINS` に含まれていない → compile-time では弾かれない。

T3 での実装が完了した後:
- `commands.rs` の `HOST_STUB_BUILTINS` から http エントリが存在しないことを確認（追加しない）。
- T1 で http を使おうとした時の target-gating は Issue 448 で対処する（本 issue のスコープ外）。

T3 実装前（過渡期）:
- T3 + http を使ったプログラムが実行時に `Err("not yet implemented")` を返すのは許容範囲内（ただし docs に明記する）。

### Step 5: capability-surface.md の全面修正

以下の箇所を更新する。

1. "Host Stub Enforcement" の http エントリを削除または訂正する。
   - T1 で動作するなら: "T1 (Wasmtime linker 経由) では利用可能。T3 では Wasmtime linker 経由で利用可能（wasi:http native path は将来拡張）。"
   - "compile-time blocked" 記述は sockets のみに限定する。
2. Function Reference テーブルの Status を `stub → available (T1/T3 linker)` に変更。
3. Target Compatibility Matrix の T1 列 `—` を `✓` に変更（確認後）。
4. Known Limitations の項目 3 の http 部分を更新。

### Step 6: fixture テスト追加

| fixture ファイル | ターゲット | CI | 内容 | 期待値 |
|---|---|---|---|---|
| `host/http/get_err_dns.ark` | T1/T3 | CI 可 | 存在しないドメインへ GET | `Err("dns: ...")` 形式 |
| `host/http/request_err_refused.ark` | T1/T3 | CI 可 | localhost:1 へ POST | `Err("connection refused: ...")` 形式 |
| `host/http/get_success_mock.ark` | T1/T3 | ローカルのみ | 実 HTTP サーバーへ GET | `Ok(body)` |

CI 可能な fixture（DNS 失敗・接続拒否）を優先的に追加し、成功ケースはローカル専用タグで管理する。

### Step 7: docs 更新

- `docs/capability-surface.md`: 上記 Step 5 の全修正。
- `docs/current-state.md`: Recent Milestones に `std::host::http` が T1/T3 で利用可能になった旨を追記。
- `std/manifest.toml`: http モジュールの stability を `experimental → available` に変更（T3 native WASI P2 path 完成時に `stable` に引き上げる）。

---

## 依存関係

- Issue 445（process）とは独立して進行可能。
- Issue 448（target-gating）が完了すると T1 で http を import した際の warning が自動的に出るようになる（本 issue は 448 の前提条件ではない）。
- Issue 037（jco Wasm GC 型サポート blocked）は T3 native component path に影響するが、本 issue の最小完了条件（Wasmtime linker 経由）には影響しない。

---

## 影響範囲

- `std/host/http.ark`（あれば）/ `std/manifest.toml`
- `crates/arukellt/src/runtime.rs`
- `crates/arukellt/src/commands.rs`（HOST_STUB_BUILTINS 変更なし、確認のみ）
- `tests/fixtures/host/http/`（新規）
- `docs/capability-surface.md`
- `docs/current-state.md`

---

## 後方互換性・移行影響

- 既存コードで `http::get` / `http::request` を呼んでいるプログラムは現状コンパイルエラーになっていないが実行時 `Err` を返していた可能性がある（HOST_STUB_BUILTINS に含まれないため）。実装完了後は `Ok` が返る場合があり、**既存コードのエラーハンドリングが `Err` 前提だった場合に影響する**。ただし、http stub が想定通り動いていた（= Err を想定した）コードは Err ケースも引き続きテストされるべきであり、実装前後で動作が壊れることはない。

---

## 今回の範囲外（明確な非対象）

- WASI P2 native component `wasi:http/outgoing-handler` を直接 wasm レベルで import する（Bridge mode の現行制約を超える）
- HTTPS クライアント証明書認証
- HTTP/2、HTTP/3 対応
- `--deny-http` 等の capability deny flag
- T1 で http を import した時の compile-time target warning（Issue 448 スコープ）

---

## 完了条件

- [x] T1 で `http::get(url)` が Wasmtime linker 経由で実行可能（エラーケース fixture が CI pass）
- [x] T3 で `http::get(url)` が Wasmtime linker 経由で実行可能（エラーケース fixture が CI pass）
- [x] エラーマッピング仕様が `docs/capability-surface.md` に明文化されている
- [x] `docs/capability-surface.md` の "compile-time blocked" 記述が sockets 専用に修正されている
- [x] Target Compatibility Matrix の http 行が実態と一致している
- [x] `bash scripts/run/verify-harness.sh` が 13/13 pass

---

## 必要なテスト

1. `host/http/get_err_dns.ark`（T1/T3）: 存在しないドメインへの GET が `Err("dns: ...")` を返す
2. `host/http/request_err_refused.ark`（T1/T3）: 接続拒否が `Err("connection refused: ...")` を返す
3. `cargo test --workspace`: runtime の http 関連 unit test（既存があれば拡充、なければ追加）

---

## 実装時の注意点

- `runtime.rs` の `http_request_impl` が依存している HTTP ライブラリのバージョンを確認し、`Cargo.lock` の変更が CI に影響しないか確認する。
- T3 の `run_wasm_gc` ルートで http linker が登録されていない場合、`Linker::func_wrap` または `Linker::define` で登録する。T1 と T3 で同じ linker 登録関数を共用できる場合は共用する。
- CI 環境でのネットワーク到達可能性を確認してから success fixture を CI に追加する。到達不可の場合は skip タグを使う。