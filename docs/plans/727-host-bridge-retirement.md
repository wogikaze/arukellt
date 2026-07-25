# #727 — Retire arukellt_host custom host bridge クローズ計画

ステータス: 計画  
親 issue: [#727](../../issues/open/727-arukellt-host-bridge-retirement.md)  
関連計画: [arukellt-host-bridge-retirement.md](arukellt-host-bridge-retirement.md)  
前提: #714 done  
担当 subagent lane: `wave/727-host-bridge`  
作業 worktree: `.worktrees/wave-727-host-bridge`  
作成日: 2026-07-25

## 1. 現状とゴール

- Phase 0/1 done、Phase 2/3 ほぼ done。
- 目標: `arukellt_host` custom host bridge を削除し、HTTP/sockets を標準 WASI P2/P3 import 経由に統一する。
- これにより `#819` runtime ABI CoreOp lowering の前提が整う。

## 2. 前提・依存

- #714（P2 emitter-native component output）done。
- `scripts/selfhost/wit/deps/wasi-cli-0.2.0` に WASI 0.2.0 WIT ファイルあり。

## 3. フェーズと完了条件

### Phase 4 — host-linker → wasmtime-wasi 移行
- `tools/host-linker/Cargo.toml` に `wasmtime-wasi` / `wasi-http` feature を追加。
- `tools/host-linker/src/host_http.rs` / `host_sockets.rs` のカスタム実装を削除。
- `lib.rs` で標準 WASI HTTP/sockets リンクを有効化。
- `needs_arukellt_host` フラグの改名を完了。

### Phase 5 — runner / gate / fixture 更新
- `gate-655`–`658` に WIT import / export 検査を追加。
- 各 gate に `wasm-tools validate` パスと `wasmtime run` 証拠を追加。
- `arukellt-selfhost.sh` / `arukellt-run-hosted.sh` が wasmtime-wasi を使用するように更新。

### Phase 6 — docs / manifest 更新
- `std/manifest.toml` から `arukellt_host` 参照を削除。
- `docs/capability-surface.md` と `docs/current-state.md` を更新。

### Phase 7 — close gate 追加
- `scripts/check/check-arukellt-host-absence.py` 新規追加:
  - HTTP/sockets fixture component が `arukellt_host` を import していないこと
  - 標準 WASI import が存在すること
  - `wasmtime run` で実行できること
- `python3 scripts/manager.py verify quick` 0 失敗。

## 4. 作業レーン・並列可否

- `#822` / `#819` と並列可能。ただし `data/core-ops.toml` や `std/manifest.toml` で競合する可能性がある。
- `tools/host-linker`（Rust）と `src/compiler/` は分離されているため、基本独立。

## 5. 検証コマンド

```bash
python3 scripts/check/gate-655-http-outgoing.py
python3 scripts/check/gate-656-http-incoming.py
python3 scripts/check/gate-657-sockets-connect-read-write.py
python3 scripts/check/gate-658-sockets-listen-accept.py
python3 scripts/manager.py verify quick
```

## 6. リスク

- WIT バージョン整合性。
- `wasmtime-wasi` の標準 HTTP/sockets 動作が `arukellt_host` 簡略 ABI と異なる場合の挙動変更。
- Phase 3 フラグ改名の漏れ。

## 7. 進捗更新規則

- Phase ごとに commit。
- `docs/capability-surface.md` 更新は Phase 6 で一度に行う。