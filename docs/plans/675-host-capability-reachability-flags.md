# \#675 — Host capability user-reachability and runtime permission flags 実装計画

ステータス: **確定** — option 1 採用（ADR-007 準拠、#727 先実装）
親 issue: [`#675`](../../issues/open/675-host-capability-reachability-flags.md)
関連: [`#727` `arukellt_host` bridge 退役計画](arukellt-host-bridge-retirement.md)

## 1. 決定事項

- `arukellt_host` stopgap は **reject**。ADR-007 は custom host module を禁止している。
- 実装順序: **#727 先** → その後 `#675` で permission flags + docs/manifest 更新。
- `#675` の runtime reachability 系 acceptance (HTTP/sockets/UDP success fixture) は `#727` で所有。`#675` は `#727` 完了後にそれらを gate / docs 整合で検証する。

## 2. 縮小された #675 完了の定義

`#727` 完了後に実施する。

- CLI フラグ `--allow-http`, `--deny-http`, `--allow-net`, `--deny-net` を追加し、host-linker の WASI P2 path で runtime 権限制御を行う。
- HTTPS URL は stable diagnostic で拒否（`#727` の host-linker 実装 or 追加ロジック）。
- `std/manifest.toml` + `docs/data/capabilities.toml` + `docs/capability-surface.md` + 生成 stdlib docs を `user_reachable` に更新。
- `scripts/gen/generate-docs.py` に `capabilities.toml` と `manifest.toml` / `availability` の drift check を追加。
- `call_host.ark` / `call_host_network.ark` の runtime capability audit gate を追加/更新。
- Close gate: `gate-675-host-capability-reachability.py` または `#138` gate 拡張。
- `python3 scripts/manager.py verify quick` exits 0.

## 3. 前提: #727 で実装される内容

`#727` plan (`docs/plans/arukellt-host-bridge-retirement.md`) に従い、以下が先に完了する。

- `std::host::http::{get,request}` → `wasi:http/outgoing-handler@0.2.x`
- `std::host::http::serve` → `wasi:http/incoming-handler@0.2.x`（または documented equivalent）
- `std::host::sockets::{connect,read,write,listen,accept}` → `wasi:sockets/tcp@0.2.x`（+ `wasi:io/streams`）
- `std::host::udp::send` → `wasi:sockets/udp@0.2.x`（`#675` 側でも要確認；`#727` plan では out-of-scope だが、ADR-007 準拠で動かすなら統一）
- `tools/host-linker` から `host_http.rs` / `host_sockets.rs` の `arukellt_host` 実装を削除し、`wasmtime-wasi` (+ `wasi-http` 機能) 経由で標準 WASI import を link。
- `arukellt_host` module 名を生成物から削除。
- `gate-655` / `656` / `657` / `658` を WIT import 検査 + `wasmtime run` 証拠へ更新。

## 4. #675 フェーズ

### Phase 0 — ドキュメント整備

- `docs/plans/675-host-capability-reachability-flags.md` 更新（本ファイル）。
- `issues/open/675-host-capability-reachability-flags.md` frontmatter:
  - `Depends on: "727"`
  - `Plan: docs/plans/675-host-capability-reachability-flags.md`
  - Decision note 追加（`arukellt_host` reject、#727 first）。
- `issues/done/727-arukellt-host-bridge-retirement.md` frontmatter:
  - `Depends on` から `675` を削除、`Related` に追加。
  - `#675` 後続注記追加。

### Phase 1 — #727 実装

- `docs/plans/arukellt-host-bridge-retirement.md` Phase 1–7 を実施。
- `#675` 作業者は #727 の進捗を追い、必要に応じて協力する。
- `#727` close gate が pass するまで `#675` の runtime 部分は着手しない。

### Phase 2 — Permission flags / CLI

- `src/compiler/main/args_record.ark`, `args_parse_flags.ark`, `usage.ark`:
  - `--allow-http`, `--deny-http`, `--allow-net`, `--deny-net` 追加。
- `tools/host-linker/src/lib.rs`:
  - `RuntimeCaps` に `allow_http`, `deny_http`, `allow_net`, `deny_net` 追加。
  - `from_cli` 拡張。
- `tools/host-linker/src/main.rs`:
  - フラグ解析。
- `scripts/run/arukellt-selfhost.sh` / `arukellt-run-hosted.sh`:
  - `run` 時にフラグを `arukellt-host-run` へ転送。
- 権限がない場合は host function 実行時に `Result::Err` を返す（実装は `#727` の host-linker 側に組み込む or 追加）。

### Phase 3 — Manifest / docs / generator

- `std/manifest.toml`:
  - `http` / `sockets` / `udp` の `request`, `get`, `serve`, `connect`, `read`, `write`, `listen`, `accept`, `send` から `implementation_status = "unreachable"` を削除。
  - `availability.note` を `--allow-http` / `--allow-net` 表記に更新。
- `docs/data/capabilities.toml`:
  - `http`, `sockets`, `udp` を `user_reachable = true`、`grant_required` に `--allow-*` 表記。
- `docs/current-state.md` 既知の制限事項を更新。
- `scripts/gen/generate-docs.py`:
  - `capabilities.toml` の `user_reachable` と `std/manifest.toml` の `availability.t3` / `implementation_status` の矛盾を検出。
- `python3 scripts/manager.py docs regenerate` 実行。

### Phase 4 — Fixtures / gate

- `tests/fixtures/host/http/` および `host/udp/` から #727 gate fixture を再利用/追加。
- `tests/fixtures/manifest.txt` に `run:` / `t3-run:` エントリ追加。
- `scripts/check/gate-675-host-capability-reachability.py`（新規 or 更新）:
  - static: manifest / `capabilities.toml` / `call_host_network.ark` / host-linker caps フラグ。
  - runtime: `arukellt-selfhost.sh run --allow-http --allow-net <fixture>` 実行と期待出力検証。
- `scripts/manager.py` の `verify quick` `bg_checks` リストに追加。

### Phase 5 — 検証

- `python3 scripts/check/gate-675-host-capability-reachability.py`
- `python3 scripts/manager.py verify lane`
- `python3 scripts/manager.py verify quick`
- `cargo test --manifest-path tools/host-linker/Cargo.toml --lib`

## 5. 変更しない領域

- `arukellt_host` 経路（削除対象）。
- `std::host::*` 公開 API 形状。
- `wasm32` + WASI P1 ターゲット（HTTP/sockets/udp は P2-only）。
- 公開 API へのユーザー可達 free function 追加。

## 6. リスク

- `#727` が未完の間 `#675` の close はブロックされる。
- `#727` の UDP 対応が out-of-scope のままだと、`std::host::udp::send` の reachability は #675 段階では未解決。必要に応じて `#727` 側に UDP 移行タスクを追加する。
- `wasmtime-wasi` 46 に `wasi-http` 機能/クレートが含まれていない可能性があり、`tools/host-linker/Cargo.toml` へ `wasmtime-wasi-http` 追加が必要。
- `--deny-*` と `--allow-*` の同時指定時の優先順位をドキュメントで明確にする必要がある。
