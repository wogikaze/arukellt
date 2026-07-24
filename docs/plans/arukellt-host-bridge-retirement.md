# `#727` — `arukellt_host` bridge retirement 実装計画

ステータス: **確定（2026-07-25）** — Phase 0–1 完了; Phase 2（WIT lowering）着手可  
親 issue: [`#727`](../../issues/open/727-arukellt-host-bridge-retirement.md)  
関連 ADR: [ADR-007](../adr/ADR-007-targets.md), [ADR-011](../adr/ADR-011-wasi-host-layering.md), [ADR-008](../adr/ADR-008-component-wrapping.md), [ADR-014](../adr/ADR-014-stability.md)  
Child: [`#830`](../../issues/open/830-wasm-heap-grow-patcher-retirement.md)（patcher 退役・本計画のスコープ外）

## 1. 現行契約と分類

| 項目 | 内容 |
|------|------|
| 現行契約 | ADR-007 はカスタム host module を廃止し、host 機能は標準 WASI P2/P3 import 経由に統一する。ADR-011 は `std::host::*` facade を維持し、HTTP/sockets は P2-only。 |
| 現行実装ギャップ | `tools/host-linker` が `arukellt_host` で HTTP/sockets を提供。emitter / stdlib intrinsic も同 module に依存。portable でない。 |
| 変更の分類 | **実装修正**（採択済み ADR-007/011 へ実装を合わせる）。公開 API 形状は変えない（互換性変更ではない）。 |
| 未決定だった点 | 本計画で確定（下記 §3）。新規 ADR は不要。 |

## 2. 完了の定義（`#727` close）

1. `std::host::http::{get,request}` → `wasi:http/outgoing-handler@0.2.x` component import
2. `std::host::http::serve` → `wasi:http/incoming-handler@0.2.x`（または documented equivalent; serve は guest export 側になり得る — Phase 1 で形状を固定）
3. `std::host::sockets::{connect,read,write,listen,accept}` → `wasi:sockets/tcp@0.2.x`（＋必要な `wasi:io/streams` / `wasi:sockets/network` 補助 interface）
4. 生成 component / core wasm に `arukellt_host` import module 名が残らない
5. `tools/host-linker` が `wasmtime-wasi` + `wasi-http` で HTTP/sockets を提供し、`host_http.rs` / `host_sockets.rs` を削除
6. `gate-655` / `656` / `657` / `658` が WIT import 検査 + `wasm-tools validate` + `wasmtime run` 証拠へ更新
7. `std/manifest.toml` / `docs/capability-surface.md` / `docs/current-state.md` から `arukellt_host` 参照を削除し、`#675` と整合する reachability 記述へ更新
8. `python3 scripts/manager.py verify quick` が pass

**含めない:** `wasm-heap-grow-patcher` 退役（`#830`）。UDP（`#675` 側）。HTTPS/TLS。公開 API 形状変更。

## 3. 確定した設計判断

### 3.1 sockets WIT package

**決定: `wasi:sockets/tcp@0.2.x` を正とする。**

- WASI 0.2 の標準 TCP interface。`std/host/udp.ark` も既に `wasi:sockets/udp` 表記。
- issue 本文の `wasi:io/sockets@0.2.x` は誤記扱いとし、acceptance / close gate 文言を本決定に合わせて更新する。
- `wasi:io/streams@0.2.x` は read/write の stream method として併用してよい（stdio `#714` と同じパターン）。
- 「両方サポート」はしない（YAGNI）。

### 3.2 wasm-heap-grow-patcher 退役

**決定: `#727` から分離し、child `#830` で追跡する。**

- `#727` acceptance criteria に含まれない。
- 退役は pinned wasm refresh / Vec_new overflow / MIR prune / `#730` bootstrap と連動する独立作業。
- `#727` は HTTP/sockets bridge 撤去のみで close 可能とする。

### 3.3 CoreOp → WIT 経路

**決定: MIR_CALL → MIR_WIT_CALL 変換は行わない。** CoreOp layer から直接 import index を計算する。`std::host::{http,sockets}` facade は維持。

## 4. Blocker と協調注意

### Phase 0 blocker: `#714` — **resolved 2026-07-25**

`#714` は bridged emitter-native WASI P2 として close 済み（`wave/714-p2-emitter-native` → master）。
`tests/fixtures/wasi_p2_native/hello.ark` が wrapper-free で `wasm-tools validate` + `wasmtime run` 緑。

HTTP/sockets は `#714` の component emit / canon lower パターンを再利用する。guest-native
`get-stdout` 直呼びは `#668` で完了済みであり、`#727` の blocker ではない。

### `#714` worktree との衝突 — **resolved**

旧 `feature/714-wrapper-retirement` の host-linker 削除はマージしていない。
`host_http.rs` / `host_sockets.rs` 削除と gate-655–658 実行証拠更新は引き続き `#727` 所有。

## 5. フェーズ

| Phase | 内容 | 正本（主な編集先） | 完了条件 |
|-------|------|-------------------|----------|
| 0 | `#714` 完了（済） | bridged P2 on master | hello validate+run; wrap deleted |
| 1 | WIT mapping + CoreOp schema（済） | `data/core-ops.toml`, generator, `CoreOpEntry` | 8 CoreOp = `kind="wit"`; generator emits package/interface/function/version |
| 2 | CoreOp → WIT lowering（着手） | `call_runtime_wit.ark`, `call_host_network.ark`, `intrinsic_http.ark`, `intrinsic_sockets.ark` | HTTP/sockets call が WIT identity / import index 経由 |
| 3 | import table / component wrapper | `sections_imports.ark`, `import_indices.ark`, `function_indices.ark`, `emit_target.ark`, `component_p2_*.ark` | `needs_arukellt_host` / `arukellt_host` エントリ削除。P1 では compile-time error 維持 |
| 4 | host-linker → wasmtime-wasi | `tools/host-linker/**`, `Cargo.toml` | カスタム HTTP/sockets 実装削除。標準 WASI link |
| 5 | runner / gate / fixture | `arukellt-selfhost.sh`, `arukellt-run-hosted.sh`, `gate-655`–`658`, fixtures | WIT 検査 + wasmtime run 証拠 |
| 6 | docs / manifest | `std/manifest.toml`, `docs/capability-surface.md`, `docs/current-state.md` | `arukellt_host` 記述削除、`#675` 整合 |
| 7 | close gate | 新規/拡張 `scripts/check/check-arukellt-host-absence.py`（例） | absence + WIT presence + run。`verify quick` |

### Phase 1 CoreOp 対応表

| CoreOp | 現 symbol | 目標 WIT |
|--------|-----------|----------|
| `runtime.get` | `http_get` | `wasi:http/outgoing-handler@0.2.x::handle` |
| `runtime.request` | `http_request` | 同上 |
| `runtime.serve` | `http_serve` | **guest export** `wasi:http/incoming-handler@0.2.0::handle`（host import ではない） |
| `runtime.connect` | `sockets_connect` | `wasi:sockets/tcp@0.2.x` create/connect 系 |
| `runtime.read` | `sockets_read` | tcp + `wasi:io/streams` read |
| `runtime.write` | `sockets_write` | tcp + streams write |
| `runtime.listen` | `sockets_listen` | tcp bind+listen |
| `runtime.accept` | `sockets_accept` | tcp accept |

注: Phase 1 で generator は WIT フィールドを emit 済み。

### Phase 2–3 実装方針（bridged、`#714` と同型）

真の WASI HTTP/sockets ABI（OutgoingRequest リソース等）を guest 直 emit するのは
別規模の作業になる。`#727` は `#714` stdio と同じ **bridged path** で閉じる:

1. Guest core は当面既存の簡略 ABI（url/buffer scratch）を維持する。
2. Import module 名を `arukellt_host` から WIT 形
   （`wasi:http/outgoing-handler@0.2.0` / `wasi:sockets/tcp@0.2.0` /
   `wasi:io/streams@0.2.0`）へ差し替え、component strip/bridge で canon lower する。
3. Component 境界には標準 WASI import だけが残る（成果物に `arukellt_host` 無し）。
4. `runtime.serve` は guest **export**（`incoming-handler::handle`）。host import ではない。
5. Phase 4 で `host_http.rs` / `host_sockets.rs` を削除し、wasmtime-wasi /
   wasi-http が component import を満たす。

`call_runtime_wit.ark` が CoreOp → WIT module/function 文字列の正本ヘルパ。

### Phase 2/3 途中成果（2026-07-25）

- Guest import module 名を WIT 形へ変更（`sections_imports.ark`）。
  `arukellt_host` 文字列は import emit から除去済み。
- `tools/host-linker` の bind 先を同じ WIT module 名へ追従（まだ簡略 ABI
  `http_get` 等。Phase 4 で削除）。
- `call_runtime_wit.ark` + `call_host_network.ark` が WIT package 判定で dispatch。

### 次のブロッカー（GC stub）

`intrinsic_http.ark` は `wasm32-gc` で HTTP を **null Result stub** している
（`emit_http_result_gc_null_after_drop_strings`）。理由は linear-memory Result
pack（`intrinsic_http_result.ark`）が GC Result enum と非互換なため（#730 関連）。

そのため現状の `wasm32-gc` + `wasi-p2` fixture は:

1. `needs_arukellt_host` が CoreOp spine 経由だと 0 のままになりやすい
2. 生成物に `http_get` / `wasi:http/...` import が出ない
3. DNS Err の期待出力を runtime で満たせない

`#727` close には次のいずれかが必要:

- **A (推奨):** GC 向けに host 戻り値 → `Result<String, String>` GC enum を組み立てる
  finalize を実装し、stub を外して WIT-shaped import を実際に call する
- **B:** component bridge で guest から host call を隠し、stdio `#714` と同型の
  canon lower だけを残す（guest ABI 刷新）

## 6. 変更しない領域

- `wasm32` + WASI P1（HTTP/sockets は P2 only のまま）
- `std::host::http` / `std::host::sockets` 公開 API 形状
- 退役済み Rust-era `crates/`
- `std::*` 直下の pure stdlib（ADR-011 分離維持）
- patcher / walrus（`#830`）

## 7. 検証コマンド

```bash
python3 scripts/check/gate-655-http-outgoing.py
python3 scripts/check/gate-656-http-incoming.py
python3 scripts/check/gate-657-sockets-connect-read-write.py
python3 scripts/check/gate-658-sockets-listen-accept.py
# Phase 7 で追加する absence gate
python3 scripts/manager.py verify lane   # 編集中
python3 scripts/manager.py verify quick  # close / merge 後
```

## 8. 進捗の更新規則

- 生きた進捗・チェックボックスは `#727` issue 本文を更新する。
- 本 plan はフェーズ構成・確定判断の正本。完了後にステータスを「完了」へ更新し、詳細数字は `docs/current-state.md` へ寄せる。
- Phase 完了ごとに worktree `wave/727-host-bridge-retirement` へ commit。`#714` merge 後に Phase 1 を開始する。
