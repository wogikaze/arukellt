# `#727` — `arukellt_host` bridge retirement 実装計画

ステータス: **完了（bridged close、2026-07-25）** — `#714` 同型。真の WASI method ABI / bare wasmtime / `host_*` 削除は [`#841`](../../issues/open/841-wit-network-real-wasi-abi.md)。  
親 issue: [`#727`](../../issues/done/727-arukellt-host-bridge-retirement.md)  
関連 ADR: [ADR-007](../adr/ADR-007-targets.md), [ADR-011](../adr/ADR-011-wasi-host-layering.md), [ADR-008](../adr/ADR-008-component-wrapping.md), [ADR-014](../adr/ADR-014-stability.md)  
Child: [`#830`](../../issues/open/830-wasm-heap-grow-patcher-retirement.md)（patcher 退役・本計画のスコープ外）  
Follow-up: [`#841`](../../issues/open/841-wit-network-real-wasi-abi.md)（real WASI ABI）

## 1. 現行契約と分類

| 項目 | 内容 |
|------|------|
| 現行契約 | ADR-007 はカスタム host module を廃止し、host 機能は標準 WASI P2/P3 import 経由に統一する。ADR-011 は `std::host::*` facade を維持し、HTTP/sockets は P2-only。 |
| 達成した実装 | Guest import module 名は WIT 形（`wasi:http/...` / `wasi:sockets/tcp@...`）。成果物に `arukellt_host` 無し。host-linker は同 WIT module 名へ簡略 guest ABI を bind。 |
| 残ギャップ（`#841`） | 真の WASI method / resource ABI、bare `wasmtime run`、`host_http.rs` / `host_sockets.rs` 削除、`needs_arukellt_host` リネーム。 |
| 変更の分類 | **実装修正**（採択済み ADR-007/011 へ実装を合わせる）。公開 API 形状は変えない。 |

## 2. 完了の定義（`#727` bridged close）

1. `std::host::http::{get,request}` → WIT module `wasi:http/outgoing-handler@0.2.x`（bridged `http_*` guest names 可）
2. `std::host::http::serve` → WIT module `wasi:http/incoming-handler@0.2.x`（bridged `http_serve`）
3. `std::host::sockets::*` → `wasi:sockets/tcp@0.2.x`（＋ `wasi:io/streams`；bridged `sockets_*`）
4. 生成 wasm に `arukellt_host` import module 名が残らない
5. `tools/host-linker` が WIT module 名で bind（カスタム `arukellt_host` 登録なし）。`host_http` / `host_sockets` 実装本体の削除は `#841`
6. `gate-655`–`658` が WIT module 文字列検査を含み、host-linker 証拠を維持
7. `std/manifest.toml` / capability docs から `arukellt_host` 参照を削除
8. `gate-727-arukellt-host-absence.py` + `verify quick` が pass

**含めない:** patcher（`#830`）。UDP（`#675`）。HTTPS/TLS。公開 API 形状変更。真の WASI ABI（`#841`）。

## 3. 確定した設計判断

### 3.1 sockets WIT package

**決定: `wasi:sockets/tcp@0.2.x` を正とする。**（issue 本文の `wasi:io/sockets` は誤記扱い）

### 3.2 wasm-heap-grow-patcher 退役

**決定: `#830` で追跡。** `#727` acceptance 外。

### 3.3 CoreOp → WIT 経路

**決定: MIR_CALL → MIR_WIT_CALL 変換は行わない。** CoreOp layer から直接 import index。facade 維持。

### 3.4 Bridged close（`#714` 同型）

真の WASI HTTP/sockets ABI は別規模。`#727` は:

1. Guest は簡略 ABI（url/buffer scratch）を維持
2. Import module 名だけ WIT 形へ差し替え
3. 成果物に `arukellt_host` 無し
4. host-linker が WIT module 名で簡略 ABI を提供
5. real ABI / bare wasmtime / shim 削除 → `#841`

## 4. フェーズ結果

| Phase | 結果 |
|-------|------|
| 0 `#714` | done |
| 1 WIT CoreOp schema | done — 8 ops `kind="wit"` |
| 2 WIT lowering + GC finalize | done — HTTP/sockets Result finalize; import emit WIT-shaped |
| 3 import table | done — module 文字列から `arukellt_host` 除去。flag 名 `needs_arukellt_host` は overlay 都合で `#841` へ延期 |
| 4 host-linker | done（bridged）— WIT module bind。`host_*` 削除は `#841` |
| 5 gates | done — 655–658 WIT 検査 + gate-727 absence/run |
| 6 docs / manifest | done — `arukellt_host` 記述削除 |
| 7 close | done — `gate-727-arukellt-host-absence.py` |

## 5. 検証コマンド

```bash
python3 scripts/check/gate-727-arukellt-host-absence.py
python3 scripts/check/gate-655-http-outgoing.py
python3 scripts/check/gate-656-http-incoming.py
python3 scripts/check/gate-657-sockets-connect-read-write.py
python3 scripts/check/gate-658-sockets-listen-accept.py
python3 scripts/manager.py verify quick
```

## 6. 変更しない領域

- `wasm32` + WASI P1（HTTP/sockets は P2 only）
- `std::host::http` / `std::host::sockets` 公開 API 形状
- patcher / walrus（`#830`）
- 真の WASI method lowering（`#841`）
