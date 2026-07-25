# P2 Bridged WASI Path — 今後の調査・修正項目

ステータス: **guest-native stdio 完了（2026-07-26）** — #714 / #668 close gate 緑。  
filesystem は #076。HTTP/sockets は #727。

調査日: 2026-07-11（更新 2026-07-25）

---

## 目的

Issue #714 の第1段階（stub-only path）は完了した。`wasm-tools validate` は通るが、
stub が全 WASI import をゼロ返却に置換するため runtime 出力が空になる。
このドキュメントは **bridged path**（実際の WASI 0.2 host imports に wiring する路径）
の実装に必要な調査・修正項目を整理し、#714 / #668 / #727 / #728 の透過的な
issue 更新の作業場として使う。

---

## 現状のアーキテクチャ（stub-only path）

```
core wasm (wasi:cli/stdout@0.2.0::write 疑似import)
  ↓ component_p2_strip.ark: strip wasi:* / wasi_snapshot_preview1 → stub functions
  ↓ component_p2_run_export.ark: run wrapper 追加 (call _start; i32.const 0; end)
  ↓ component_p2_emit.ark: component 組立
  →   core module (1個, self-contained)
  →   core instance (import args 0個)
  →   run sections (wasi:cli/run@0.2.0 export)
  → 結果: validate OK, wasmtime run exit 0, stdout 空
```

### 達成済み

- [x] `p2_component_wrap.py` / `p2_guest_fs_patch.py` / `p2_guest_stdio_patch.py` /
      `p2_strip_imports.py` 削除
- [x] `component_p2_strip.ark` — import strip + stub function 生成
- [x] `component_p2_run_export.ark` — `run` wrapper 関数追加
- [x] `component_p2_emit.ark` — component 組立
- [x] `component_p2_run_sections.ark` — wasi:cli/run@0.2.0 export sections
- [x] `component_p2_run_tail.ark` — 内側 component binary (hex embedded)
- [x] gate #074, #510 — validate + wasmtime `hello p2`
- [x] gate #076 — validate-only（runtime fs は #076）
- [x] bridged path: component imports `wasi:cli/stdout` + `wasi:io/streams`
- [x] `wasmtime run` → `hello p2`
- [x] exit-code fixture on emitter-native path
- [x] `arukellt-selfhost.sh run --emit component` wrapper-free

### 未達成（#076 / #727 へ移管）

- [x] guest-native `get-stdout` + stream method call sites（#668）
- [x] stderr / args / env fixtures（#668）
- [ ] filesystem runtime bridge（#076）
- [ ] HTTP/sockets standard WASI imports（#727）

---

## 調査・修正項目

### A. Stdout bridge の in-tree 実装

**関連 issue**: #714 (acceptance: stdout 出力)
**優先度**: P0（gate #074 stdout check 有効化の前提）
**現状**: `component_p2_bridged.ark` に hex-encoded binary として存在するが、
`component_p2_emit.ark` からは呼び出されていない

#### A-1. stdout bridge core module の動的生成

**調査項目**:
- `component_p2_bridged.ark` の `p2_stdout_bridge_bytes()` (129 bytes, hex) を
  動的生成に置換できるか
- bridge module の仕様:
  - import: `env.get-stdout` (func → i32), `env.blocking-write-and-flush` (func), `host.memory` (memory)
  - export: `write(ptr, len, ret, _) → i32` (canonical ABI shape)
  - body: `call $flush(call $get_stdout, local.get $ptr, local.get $len, local.get $ret); i32.const 0`

**修正項目**:
- `component_p2_stubs.ark` に `p2_stdout_bridge_module()` 関数を追加
  - type section, import section, export section, code section を動的生成
  - hex string 依存を除去
- `component_p2_emit.ark` の `emit_p2_command_component` で
  stdout bridge module を core module として埋め込む

**難易度**: 中 — wasm binary 生成ロジックは既存 stub 生成で実績あり

#### A-2. Host import prefix の動的生成

**調査項目**:
- `component_p2_bridged.ark` の `p2_host_import_prefix_bytes()` (643 bytes, hex) の内容:
  - component type section (wasi:cli/exit, wasi:io/error, wasi:io/streams)
  - component import section (5 imports)
  - component instance section (wasi:io/streams の instance exports)
- これを `writer_core` + `writer_string` で動的生成できるか
- GC 型システム (0x6A resource type, 0x4E/0x4F/0x50 sub types) の
  component-level encoding との整合性

**修正項目**:
- `component_p2_emit.ark` に host import prefix 生成関数を追加
  - type section: `wasi:cli/exit@0.2.0`, `wasi:io/error@0.2.0`,
    `wasi:io/streams@0.2.0`, `wasi:cli/stdin@0.2.0`, `wasi:cli/stdout@0.2.0`
  - import section: 上記5 interface の component import
  - instance section: `wasi:io/streams` instance exports の alias

**難易度**: 高 — component model binary encoding の GC 型システム理解が必要

#### A-3. Instance section の wiring

**調査項目**:
- bridge instance: `env.get-stdout` → wasi:cli/stdout instance, `env.blocking-write-and-flush` → wasi:io/streams instance, `host.memory` → guest instance export "memory"
- guest instance: 5 P2 host import args を各 instance に wiring
- stub instances (env, fs, read, exit) の wiring

**修正項目**:
- `component_p2_emit.ark` の instance section 生成を
  bridge + stubs + guest の multi-instance 構成に更新
- `p2_bridged_emit_instance_section` のロジックを統合

**難易度**: 中 — 既存 `p2_bridged_emit_instance_section` が参考実装

#### A-4. Guest core wasm の import shape 変換

**調査項目**:
- 現在 `sections_imports.ark` が生成する `wasi:cli/stdout@0.2.0::write` 疑似 import を
  どう扱うか:
  1. strip → stub 化（現状）+ bridge module が代わりに host に接続
  2. import を component-correct な形状に変換（#714 の理想）
- 短期案 (1) は bridge module が `write` export を提供し、guest がそれを
  import する形。guest の import section は strip された後、
  bridge module の `write` export に wiring される
- 長期案 (2) は #728 WIR layer で実現

**修正項目**:
- 短期案: `component_p2_strip.ark` の strip 後、
  guest の `wasi:cli/stdout@0.2.0::write` import を
  bridge instance の `write` export に置換
- instance section で guest の import args に bridge instance を指定

**難易度**: 中 — instance arg の wiring ロジックは既存

---

### B. Stderr / exit-code / args / env_var fixture 追加

**関連 issue**: #668 (acceptance: stderr, fixture coverage)
**優先度**: P1（#714 完了後に着手）
**前提**: A. Stdout bridge 実装完了

#### B-1. stderr bridge

**調査項目**:
- `wasi:cli/stderr@0.2.0` の host import 追加
- stderr bridge core module (stdout bridge と同構造、`get-stderr` 使用)
- `eprintln` のルーティング確認

**修正項目**:
- host import prefix に `wasi:cli/stderr@0.2.0` 追加
- stderr bridge module 生成
- instance section に stderr wiring 追加

#### B-2. exit-code fixture

**調査項目**:
- `wasi:cli/exit@0.2.0::exit` import の扱い
- stub ではなく実際の exit 機能に wiring する方法
- wasmtime での exit code 検証

#### B-3. args / env_var fixture

**調査項目**:
- `wasi:cli/environment@0.2.0` の `args-sizes` / `arguments` / `get-env` の wiring
- stub module から実際の host instance への切り替え
- canonical ABI での `list<string>` lowering

---

### C. arukellt_host bridge retirement (HTTP/sockets)

**関連 issue**: #727
**優先度**: P1（#714, #675 完了後）
**前提**: A. Stdout bridge 完了（同じ canonical ABI glue 基盤を使用）

#### C-1. HTTP import 移行

**調査項目**:
- `std::host::http` の `__intrinsic_http_get` / `__intrinsic_http_request` を
  `wasi:http/outgoing-handler@0.2.x` component import に移行
- `tools/host-linker/src/host_http.rs` を `wasmtime-wasi` (wasi-http feature) に置換
- canonical ABI glue: `wasi:http/types outgoing-request` resource handle,
  `outgoing-handler.handle` 関数の lowering

#### C-2. Sockets import 移行

**調査項目**:
- `std::host::sockets` の `__intrinsic_sockets_*` を
  `wasi:io/sockets@0.2.x` (or `wasi:sockets/tcp@0.2.x`) に移行
- `tools/host-linker/src/host_sockets.rs` を `wasmtime-wasi` に置換
- TCP connect/listen/accept の resource handle 管理

#### C-3. wasm-heap-grow-patcher retirement

**調査項目** (#727 関連):
- pinned wasm の memory section 更新 (128 → 8192 pages)
- Vec_new u32 wraparound check の compiler 組み込み
- export deduplication の完全性確認
- MIR prune flag の pinned wasm 更新

---

### D. WIR / backend target IR 設計

**関連 issue**: #728
**優先度**: P3（設計調査）
**前提**: A-C の実装で得た知見を反映

#### D-1. WIR layer 設計

**調査項目**:
- MIR と wasm byte emitter の間に WIR layer を挿入する設計
- `is_gc_target` / `is_p2_wasi` / `is_freestanding_target` ブランチの集約
- `HostCall` WIR operation による host function unification
- canonical ABI glue の WIR からの生成
- T4 native との関係 (共通 BackendIR vs 別 NativeIR)

#### D-2. Prototype

**調査項目**:
- `stdio::println` を MIR → WIR → wasm bytes で証明
- T1 P1 core wasm, T3 P2 core wasm, T3 P2 component wasm の3経路

---

### E. Bootstrap / selfhost 制約

**関連 issue**: #714, #668 (FD-07 risk)
**優先度**: 継続的

#### E-1. 早期 return 制約

**調査項目**:
- selfhost compiler で早期 `return` が壊れる問題（`range_equals` で遭遇）
- どのパターンの早期 return が安全か / 危険か
- overlay patch で対応すべきか、compiler 修正すべきか

**修正項目**:
- 危険な早期 return を flag-based 制御フローに書き換え
- 新規コードで早期 return を避ける guideline 確立

#### E-2. flat-overlay-cache の hash 方針

**調査項目**:
- `_compiler_source_content_hash` が git blob OID を使用するため、
  unstaged changes が cache に反映されない問題
- 開発中の cache 無効化手法の整理

#### E-3. BOOTSTRAP_COMPONENT_STUB の risk

**調査項目** (#668 FD-07):
- `scripts/selfhost/checks.py` の `BOOTSTRAP_COMPONENT_STUB` が
  bootstrap path で残存している risk
- stub-only path と bridged path の切り替えが
  bootstrap で正しく行われるか

---

## 実装優先順位

```
Phase 1: A-1, A-3, A-4 — stdout bridge 復活 (gate #074 stdout check 有効化)
  ↓
Phase 2: A-2 — host import prefix 動的生成 (hex 依存除去)
  ↓
Phase 3: B-1, B-2, B-3 — stderr/exit/args/env fixture (gate #668)
  ↓
Phase 4: C-1, C-2 — HTTP/sockets 標準 WASI import 移行 (#727)
  ↓
Phase 5: D-1, D-2 — WIR layer 設計 (#728)
```

---

## 技術的制約メモ

### selfhost compiler の制約

- 早期 `return` が壊れる場合がある（`range_equals`, `find_type_index` で遭遇）
  → flag-based 制御フローで回避
- `for` loop より `while` loop が安全
- `get_unchecked` は境界チェックを skip するため高速だが危険
- struct field 追加は安全（runtime layout 計算）、削除は危険

### component model binary encoding

- GC 型システム: 0x6A (resource type), 0x4E/0x4F/0x50 (sub types),
  0x5E (array), 0x5F (struct), 0x60 (func)
- sub type (0x50) は `vec<typeidx>` + comptype を含む（順序重要）
- struct field: valtype → mutability の順（逆は不可）
- abstract heap type (0x63/0x64) は常に heap type index が後続

### canonical ABI

- `list<u8>` lowering: guest 側は `ptr` + `len` (linear memory),
  component 側は `list<u8>` (lift/lower 変換が必要)
- `own<T>` resource handle: integer handle として表現,
  handle table 管理が必要
- `result<T, E>`: tag + payload の lowering,
  error path の制御フローが必要

---

## 参照

- [Issue #714](../../issues/open/714-wasi-p2-emitter-native-component-output.md) —
  Emitter-native WASI P2 component output without wrapper
- [Issue #668](../../issues/open/668-p2-native-component-polish.md) —
  P2 native component polish (post-#074)
- [Issue #727](../../issues/open/727-arukellt-host-bridge-retirement.md) —
  Retire arukellt_host custom host bridge; migrate HTTP/sockets to standard WASI P2/P3 imports
- [Issue #728](../../issues/open/728-wir-target-backend-ir.md) —
  WIR / backend target IR for ADR-007 multi-target separation
- [ADR-008](../adr/ADR-008-component-wrapping.md) —
  Component wrapping strategy (in-tree)
- [ADR-007](../adr/ADR-007-targets.md) —
  Target tiers and host-function unification
- `src/compiler/wasm/component_p2_emit.ark` — 現在の stub-only emitter
- `src/compiler/wasm/component_p2_bridged.ark` — bridged path 参考実装 (hex encoded)
- `src/compiler/wasm/component_p2_stubs.ark` — stub core module 生成
- `src/compiler/wasm/component_p2_strip.ark` — import strip + stub 化
- `src/compiler/wasm/component_p2_run_export.ark` — run wrapper 追加
- `src/compiler/wasm/component_p2_run_sections.ark` — wasi:cli/run export sections
- `scripts/selfhost/checks.py` — bootstrap overlay patch ロジック
- `scripts/check/check-false-done-close-gates.py` — gate #074, #076, #510
