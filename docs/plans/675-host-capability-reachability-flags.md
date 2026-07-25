# \#675 — Host capability user-reachability and runtime permission flags 実装計画

ステータス: **草案** — 実装着手前に `$implementation-strategy` で ADR-007 / `#727` との整合を確定する。
親 issue: [`#675`](../../issues/open/675-host-capability-reachability-flags.md)
関連: [`#727` `arukellt_host` bridge 退役計画](arukellt-host-bridge-retirement.md), [`#633`](../../issues/done/633-host-capability-surface-honesty-vs-selfhost-runtime.md)

## 1. 完了の定義

- `std::host::http::{get,request}` が `wasm32-gc` + `wasi-p2`（T3 selfhost）のユーザー Ark から呼べる。
- `std::host::sockets::{connect,read,write,listen,accept}` が同じ T3 パスで呼べる。
- `std::host::udp::send` の compile-time `host_stub` 拒否を削除し、runtime dispatch を実装する。
- CLI フラグ `--allow-http`, `--deny-http`, `--allow-net`, `--deny-net` を追加し、host-linker で runtime 権限制御を行う。
- HTTPS URL は runtime で `Result::Err` を返す。可能であれば `https://` リテラルを compile-time 診断で弾く。
- Fixture / gate を追加・更新し、`std/manifest.toml`・`docs/data/capabilities.toml`・生成ドキュメントを `user_reachable` に合わせる。
- `scripts/gen/generate-docs.py` が `capabilities.toml` と `manifest.toml` の `user_reachable` 主張の乖離を検出する。
- `call_host.ark` / `call_host_network.ark` の capability dispatch を監査する gate を追加する。
- `python3 scripts/manager.py verify quick` が pass する。

## 2. 現行契約と ADR

- ADR-007 / `#727`: 最終的には host HTTP/sockets/UDP を `wasi:http/outgoing-handler` / `wasi:sockets/tcp` / `wasi:sockets/udp` など標準 WASI P2/P3 import に統一し、`arukellt_host` ブリッジを廃止する。
- `#675` の目的は「import 経路の移行」ではなく「ユーザー可達性と runtime 権限フラグ」である。`arukellt_host` を使った一時的な stopgap を、`#727` の removal condition を明示して実装する。
- `std::host::*` の公開 API 形状は変えない（ADR-011 facade 分離を維持）。
- `docs/current-state.md` 既知の制限事項 (line 313) では http/sockets/udp が「not user-reachable」と記載されている。
- `docs/data/capabilities.toml` と `std/manifest.toml` は capability surface の正本。

## 3. 変更の分類

- **一時的な実装ギャップ**: `wasm32-gc` 向けに `arukellt_host` 経由で host bridge を実装し、`#727` 完了時に WASI P2 import へ置き換える。
- **公開 API**: 変えない。
- **互換性**: stable/provisional ラベルは ADR-014 に従い、現行の `provisional` を維持。
- **診断**: `host_stub` からの E0500 を capability / runtime エラーに置き換える。

## 4. 大きな設計判断

1. **Stopgap bridge**: `wasm32-gc` の `http` / `sockets` / `udp` call を GC String/Vec → linear memory に変換し、`arukellt_host` import を呼び出す。戻り値は GC `Result` / `Vec` として再構築する。
2. **Runtime linker**: `tools/host-linker` の P2 path (`run_wasm_p2`) で `arukellt_host` import に実装を登録する。P1 path (`run_wasm`) も維持する。
3. **権限モデル**: runtime 権限はデフォルト deny。`--allow-http` / `--allow-net` で許可。`--deny-http` / `--deny-net` は明示的 deny（`--allow-*` より優先）。権限がなければ host 関数は `Result::Err` を返す。
4. **HTTPS**: TLS 未実装のため `http://` のみ許可。`https://` は runtime エラー。compile-time リテラル検出は可能なら追加。
5. **Semantic debt**: `arukellt_host` GC bridge を `docs/data/semantic-debt-allowlist.toml` に `#727` removal condition 付きで登録し、サイトに `TODO(#727 owner=... removal=...)` コメントを置く。

## 5. フェーズ構成

### Phase 0 — 設計判断の確定

- `$implementation-strategy` を起動し、ADR-007 / `#727` との整合と stopgap の範囲を確定する。
- 必要に応じて `$architecture-decision` で ADR-007 への一時例外を記録する。
- `docs/data/semantic-debt-allowlist.toml` に `arukellt_host` GC bridge のエントリを追加する（removal = `#727`、recheck = 実装後 3 ヶ月）。

### Phase 1 — CLI / config / runner 配線

- `src/compiler/main/args_record.ark`: `allow_http`, `deny_http`, `allow_net`, `deny_net` フィールドを追加。
- `src/compiler/main/args_parse_flags.ark` / `usage.ark`: フラグを解析・表示。
- `src/compiler/main/project_run.ark` / `build_run_driver_config`: `run` 時もフラグを無視して通す。
- `tools/host-linker/src/lib.rs`: `RuntimeCaps` に `allow_http`, `deny_http`, `allow_net`, `deny_net` を追加。`from_cli` を拡張。
- `tools/host-linker/src/main.rs`: `--allow-http` などを解析。
- `scripts/run/arukellt-selfhost.sh` / `arukellt-run-hosted.sh`: `run` 時にフラグを `arukellt-host-run` へ転送。

### Phase 2 — Resolver / host_stub / capability ゲート

- `src/compiler/resolver/host_stub_gate.ark`:
  - `udp::send` / `std::host::udp::send` を `path_is_host_stub` から削除。
  - オプションで compile-time deny フラグに応じた rejection を追加（Phase 1 の `DriverConfig` 渡しが必要）。
- `src/compiler/mir/module_host_calls.ark`: `__intrinsic_udp_send` を `mir_call_is_arukellt_host` に追加。
- `src/compiler/wasm/call_host.ark` / `call_host_network.ark`: 新しい capability パスを監査し、dispatch 表を更新。

### Phase 3 — CoreOp schema

- `data/core-ops.toml`: `runtime.udp_send` を追加。aliases: `udp_send`, `__intrinsic_udp_send`, `udp::send`, `std::host::udp::send`。
- `scripts/gen/generate-core-ops-registry.py` と `scripts/gen/generate-core-op-bindings.py` を実行し、以下を再生成：
  - `src/compiler/corehir/core_op_registry_generated.ark`
  - `src/compiler/corehir/core_op_binding_generated.ark`
- `src/compiler/wasm/call_host_network.ark`: `runtime.udp_send` handler を `intrinsic_udp` へ dispatch する分岐を追加。

### Phase 4 — Emitter GC lowering

- `src/compiler/wasm/intrinsic_http.ark`:
  - `emit_http_get_save_args_gc`, `emit_http_request_save_args_gc`, `emit_http_serve_save_args_gc` を本番呼び出しに繋ぐ。
  - GC `Result<String, String>` / `Result<(), String>` を構築する helper を追加（linear-memory 結果を GC string / unit enum に変換）。
- `src/compiler/wasm/intrinsic_sockets.ark`:
  - `connect` / `listen` / `accept` の GC path で String を linear memory に変換して import call。
  - `read` の GC path で linear-memory バイト列から GC `Vec<i32>` を構築。
  - `write` の GC path で `Vec<i32>` を linear-memory バイト列に変換。
  - `Result<i32, String>` / `Result<Vec<i32>, String>` の GC 版 builder を追加。
- `src/compiler/wasm/intrinsic_udp.ark`（新規）: `udp_send` の GC lowering を実装。
- `src/compiler/wasm/import_indices.ark`: `udp_send_import_idx(ctx)` を追加。
- `src/compiler/wasm/sections_imports.ark`: `arukellt_host` セクションに `udp_send` import を追加。
- `src/compiler/wasm/sections_type_plan.ark` / `src/compiler/wasm/constants.ark`: `WASI_HOST_TYPE_UDP_SEND` と `(i32 i32 i32 i32 i32 i32) -> i32` 型を追加。

### Phase 5 — host-linker runtime

- `tools/host-linker/src/lib.rs`:
  - `run_wasm_p2` で `arukellt_host` import を auto-stub せず、`host_http` / `host_sockets` / `host_udp` の実装を登録。
  - `RuntimeCaps` を `register_*_host_fns` へ渡し、deny 時は `Err` を返す。
- `tools/host-linker/src/host_http.rs` / `host_sockets.rs`:
  - `Caller<'_, WasiP1Ctx>` 依存を `impl AsContext` / `impl AsContextMut` へ generify し、P1 / P2 (`()`) の両方で使えるようにする。
  - 各 host 関数先頭で `caps.allow_http` / `caps.allow_net` を検査。
- `tools/host-linker/src/host_udp.rs`（新規）:
  - `arukellt_host::udp_send` を `std::net::UdpSocket` で実装。
  - 無効ホスト・無効ポート・DNS 失敗時の診断文字列を `std/host/udp.ark` の `errors` 表に合わせる。

### Phase 6 — Manifest / docs / generator 整合

- `std/manifest.toml`:
  - `http` / `sockets` / `udp` の `request`, `get`, `serve`, `connect`, `read`, `write`, `listen`, `accept`, `send` から `implementation_status = "unreachable"` を削除。
  - `availability.note` を `--allow-http` / `--allow-net` 表記に更新。
  - `request_with_headers`, `read_body`, `response_status` は `unreachable` を維持。
- `docs/data/capabilities.toml`:
  - `http`, `sockets`, `udp` の `user_reachable = true`, `grant_required = "yes (--allow-http/--allow-net)"`, `verified_on = ["wasm32-gc"]` へ更新。
- `docs/current-state.md` 既知の制限事項を更新。
- `scripts/gen/generate-docs.py`:
  - `capabilities.toml` の `user_reachable` と `std/manifest.toml` の `availability.t3` / `implementation_status` が矛盾している場合にエラー終了する check を追加。
- `python3 scripts/manager.py docs regenerate` を実行。

### Phase 7 — Fixtures / gate

- Fixture 追加:
  - `tests/fixtures/host/http/get_success.ark`（loopback 簡易サーバーまたは既存 helper を使った緑の GET）
  - `tests/fixtures/host/http/get_404.ark`
  - `tests/fixtures/host/http/get_timeout.ark`
  - `tests/fixtures/host/udp/send_success.ark`
  - `tests/fixtures/host/udp/send_invalid_host.ark`
- `tests/fixtures/manifest.txt` に `t3-run:` / `run:` エントリを追加。
- `scripts/check/gate-675-host-capability-reachability.py`（新規）:
  - static: manifest, `capabilities.toml`, `call_host_network.ark`, `sections_imports.ark`, `host-linker` caps フラグ。
  - runtime: `arukellt-selfhost.sh run --allow-http --allow-net <fixture>` のコンパイル・実行と期待出力を検証。
  - `call_host.ark` / `call_host_network.ark` runtime capability audit を実施。
- `scripts/manager.py` の `verify quick` `bg_checks` リストに gate-675 を追加。
- `scripts/check/gate-138-shared-capabilities-t1-t3.py` を `http` / `sockets` / `udp` まで拡張するか、gate-675 へ統合。

### Phase 8 — 検証

-  emitter 編集後: `python3 scripts/manager.py selfhost build-compiler`
-  `python3 scripts/check/gate-675-host-capability-reachability.py`
-  `python3 scripts/manager.py verify lane`（編集中）
-  `python3 scripts/manager.py verify quick`（merge 前）
-  `cargo test --manifest-path tools/host-linker/Cargo.toml --lib`

## 6. 変更しない領域

- `wasm32` + `wasi-p1` ターゲットで `std::host::http` / `sockets` / `udp` を利用可能にする（target gate は維持）。
- `std::host::*` の公開 API 形状。
- `crates/` など退役 Rust-era 経路。
- `#727` 自身のスコープ（WASI P2 import 移行）。

## 7. リスクと未決定事項

- **ADR-007 整合**: `arukellt_host` GC bridge は ADR-007 の理想形と衝突する。Phase 0 で `$implementation-strategy` / `$architecture-decision` で stopgap を承認し、`semantic-debt-allowlist.toml` に登録する必要がある。
- **GC Result/Vec builder**: 現行の `intrinsic_http_result.ark` / `intrinsic_sockets_vec.ark` は linear-memory 版。GC 版 builder を新設する必要がある。
- **host-linker generic 化**: `host_http.rs` / `host_sockets.rs` の `WasiP1Ctx` ロックを外し、`Linker<()>` でも動作するよう変更する。
- **network fixture の揺らぎ**: loopback と `.invalid` TLD を使い、外部ネットワーク依存を排除する。
- **HTTPS compile-time 診断**: リテラル判定を実装する場合、`intrinsic_http.ark` で `MIR_CONST_STRING` を検出して `E0500` または専用コードを発行する。
