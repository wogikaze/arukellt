# #727 — Retire arukellt_host custom host bridge クローズ計画

ステータス: **完了（bridged close、verified 2026-07-26）** — `#714` 同型。真の WASI method ABI / bare wasmtime / `host_*` 削除は [`#841`](../../issues/open/841-wit-network-real-wasi-abi.md)。  
親 issue: [#727](../../issues/done/727-arukellt-host-bridge-retirement.md)  
関連計画: [arukellt-host-bridge-retirement.md](arukellt-host-bridge-retirement.md)  
前提: #714 done  
担当 subagent lane: `wave/727-host-bridge`  
作業 worktree: `.worktrees/wave-727-host-bridge`  
作成日: 2026-07-25  
検証日: 2026-07-26（L4 close-gate re-verify）

## 1. 現状とゴール

- Phase 0–7 done（bridged close）。issue は `issues/done/` 済み。
- 達成: guest import から `arukellt_host` を除去し、HTTP/sockets を WIT 形 module 名へ統一。
- 残ギャップ（`#841`）: 真の WASI method / resource ABI、bare `wasmtime run`、`host_http` / `host_sockets` shim 削除、`needs_network_runtime` リネーム。
- これにより `#819` runtime ABI CoreOp lowering の前提が整う。

## 2. 前提・依存

- #714（P2 emitter-native component output）done。
- `scripts/selfhost/wit/deps/wasi-cli-0.2.0` に WASI 0.2.0 WIT ファイルあり。

## 3. フェーズと完了条件

| Phase | Status |
|-------|--------|
| 0 `#714` | done |
| 1 WIT CoreOp schema | done |
| 2–3 WIT import emit + GC finalize | done |
| 4 host-linker WIT bind | done（shims remain → #841） |
| 5 gates 655–658 + gate-727 | done |
| 6 docs / manifest | done |
| 7 close gate | done — `gate-727-arukellt-host-absence.py` |

### Phase 4 — host-linker WIT bind（bridged）
- host-linker は WIT module 名へ簡略 guest ABI を bind。
- `host_http.rs` / `host_sockets.rs` のカスタム実装削除と真の `wasmtime-wasi` リンクは `#841`。
- `needs_network_runtime` フラグ改名は overlay 都合で `#841` へ延期。

### Phase 5 — runner / gate / fixture 更新
- `gate-655`–`658` に WIT import 検査を追加済み。
- hosted run 証拠は host-linker 経由（bare `wasmtime run` → #841）。

### Phase 6 — docs / manifest 更新
- `std/manifest.toml` / `docs/capability-surface.md` / `docs/current-state.md` から `arukellt_host` bridge 主張を削除済み。

### Phase 7 — close gate
- `scripts/check/gate-727-arukellt-host-absence.py`:
  - HTTP/sockets fixture が `arukellt_host` を import しない
  - 標準 WIT import が存在する
  - host-linker で HTTP DNS Err を実行できる

## 4. 作業レーン・並列可否

- 実装レーン完了。残作業は `#841` / `#830`（本計画スコープ外）。

## 5. 検証コマンド（L4 re-verify 2026-07-26）

```bash
export ARUKELLT_BUILD_DIR="$PWD/.build"
python3 scripts/check/gate-727-arukellt-host-absence.py   # PASS
python3 scripts/check/gate-655-http-outgoing.py            # PASS
python3 scripts/check/gate-656-http-incoming.py            # PASS
python3 scripts/check/gate-657-sockets-connect-read-write.py  # PASS
python3 scripts/check/gate-658-sockets-listen-accept.py    # PASS
```

注: worktree に s2 が無い場合は `python3 scripts/manager.py selfhost build-compiler`（または現行 s2-runtime の配置）が必要。bootstrap のみでは HTTP/sockets WIT import が出ない。

## 6. リスク（残）

- 真の WASI ABI 差分は `#841` で解消する。
- `needs_network_runtime` 名の残存は overlay 都合の既知延期。

## 7. 進捗更新規則

- Phase 実装は完了済み。本ファイルは close 後の verified 記録を正とする。
- 追加の実装進捗は `#841` 側の計画へ書く。
