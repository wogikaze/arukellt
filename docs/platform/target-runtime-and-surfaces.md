# ターゲット実行面・検証・capability（現行）

ステータス: 現行挙動メモ（決定記録ではない）  
関連 ADR: [ADR-007](../adr/ADR-007-targets.md)  
正本の決定は ADR-007。本ファイルはランタイム対応・検証面・host 到達可能性・CLI 詳細の置き場。

---

## 機能差分表

各実行環境（ツールチェーン・ランタイム）の Wasm 機能対応状況:

| 機能 | wabt 1.0.34 | iwasm 2.4.1 / WAMR | Chrome | Node.js | Wasmtime | jco |
|------|------------:|-------------------:|-------:|--------:|---------:|----:|
| Core Wasm MVP | 可 | 可 | 可 | 可 | 可 | 可 |
| multi-value | 可 | 可扱いでよい | 可 | 可 | 可 | 可 |
| bulk memory | 可 | 可 | 可 | 可 | 可 | 可 |
| reference types | 可 | 可扱いでよい | 可 | 可 | 可 | 可 |
| fixed SIMD `v128` | wabtは対応 | WAMRは構成依存 | 可 | 可 | 可 | JSエンジン依存 |
| relaxed SIMD | wabtはflag付き対応 | 怪しい/避ける | Chrome可、Safari差あり | V8依存で可寄り | 可 | JSエンジン依存 |
| Wasm GC | wabt 1.0.34では実用対象外 | 2.4系に実装はあるが制限あり | 可 | Node 22+で可 | 可 | 実行エンジン依存 |
| tail-call | wabtはflag付き対応 | 構成依存 | 主要ブラウザ可 | V8依存 | 可 | 実行エンジン依存 |
| exception handling | wabtはflag付き対応 | WAMRはlegacy/構成依存に注意 | 可 | V8依存 | 可 | 実行エンジン依存 |
| memory64 | wabtはflag付き対応 | 2.4系に実装あり、制限注意 | ブラウザ差/制限あり | V8依存 | 可 | 実行エンジン依存 |
| multi-memory | wabtはflag付き対応 | 構成依存 | ブラウザ差あり | V8依存 | 可 | 実行エンジン依存 |
| Component Model | 不可 | 基本対象外 | ネイティブ不可 | ネイティブ不可 | 可 | 可 |
| WASI P1 | wabt自体は実行hostではない | iwasmで可 | 標準不可 | `node:wasi`で可 | 可 | 主対象ではない |
| WASI P2/P3 | 不可 | 主対象外 | ネイティブ不可 | ネイティブ不可 | 可 | 可 |

### ターゲット ↔ 実行環境の対応

| ターゲット | 主な実行環境 | 備考 |
|-----------|-------------|------|
| `wasm32` | wabt 1.0.34, iwasm 2.4.1 | AtCoder 環境 |
| `wasm32-gc` | Chrome, Node.js, wasmtime | ブラウザが主、Node.jsはおまけ、wasmtimeはフル機能。`jco` は packaging tool（実行エンジンではない） |
| `native-cpp` | gcc/clang (C++ コンパイル) | ローカルデバッグ |
| `native-llvm` | LLVM | ローカルデバッグ、性能比較 |

---

## 禁止事項

- `wasm32` の linear memory 実装を `wasm32-gc` に持ち込まない
- `native` 向けの言語機能・最適化方針・ABI は ADR-045 の再評価と後継採択まで固定しない
- `wasm32` は AtCoder が GC 対応したら即座に廃止する。将来に向けた拡張は `wasm32-gc` に行う

---

## 未対応機能を使おうとした際の挙動

ターゲットが対応していない機能をユーザーが使おうとした場合の挙動:

### `wasm32` — WASI P2以降・未対応Wasm機能

- **WASI P2以降の機能**（HTTP、sockets 等）を使用しようとした場合:
  **コンパイルエラー**。
- **Wasm GC、Component Model、relaxed SIMD 等の未対応Wasm機能**を使用しようとした場合:
  **コンパイルエラー**。
  ただし、ほとんどの場合 **線形メモリへのフォールバック** を用意する。
  フォールバックが存在する機能についてはエラーではなくフォールバックコードを生成する。
  フォールバックが存在しない機能のみコンパイルエラーとする。

### `wasm32-gc` — `--wasi none` / `--wasi p1` / 未対応Wasm機能

- **`--wasi none` または `--wasi p1` を指定した場合**:
  **コンパイルエラー**。
  `wasm32-gc` は WASI P2/P3 のみを前提とする。WASI 非依存や P1 が必要な場合は
  `wasm32` ターゲットを使用すること。
- **未対応Wasm機能**（`wasm32-gc` の実行環境がサポートしない機能）を使用しようとした場合:
  **コンパイルエラー**。

> **旧 T2（`wasm32-freestanding`）からの変更**:
> 旧 T2 では stdio は `arukellt_io` ホスト経由で動作し、clock/random/env/fs/http/sockets は
> `unreachable` 命令で trap する半動的ゲートだった。新設計では `arukellt_io` を廃止し、
> 全てのホスト関数を WASI P2/P3 imports 経由に統一する。ブラウザ向けは jco transpile が
> WASI imports を JS glue に変換するため、WASI 非依存モードは不要となる。
> これにより `unreachable` 分岐（`intrinsic_clock.ark`, `intrinsic_random.ark` 等）と
> `process::exit` のターゲット分岐不足バグも解消する。

### `native-cpp` / `native-llvm`

- 別途 ADR で決定する。

---

## 出力ファイル

出力ファイル名は入力ファイル名の stem（拡張子 `.ark` を除去した部分）を元に生成する。

### `<input>` の定義

- `<input>` = 入力ファイルパスから `.ark` 拡張子を除去した文字列
  - 例: `src/hello.ark` → `<input>` = `src/hello`
  - 例: `tests/fixtures/wasi_fs_p2.ark` → `<input>` = `tests/fixtures/wasi_fs_p2`
- 拡張子が `.ark` でない場合はファイルパス全体を `<input>` とする
- `--output <path>` で上書き可能:
  - `--emit core-wasm` 時: `<path>` がそのまま出力ファイル名
  - `--emit all` 時: `<path>` をベースに `.wasm` / `.component.wasm` を派生
  - `<path>` が `.wasm` で終わる場合、component 側は `.wasm` を `.component.wasm` に置換

### ターゲット別出力ファイル

| ターゲット | `--emit` | 出力ファイル | 備考 |
|-----------|----------|-------------|------|
| `wasm32` | `core-wasm` (default) | `<input>.wasm` | |
| `wasm32` | `wat` | `<input>.wat` | |
| `wasm32-gc` | `core-wasm` (default) | `<input>.wasm` | |
| `wasm32-gc` | `wat` | `<input>.wat` | |
| `wasm32-gc` | `component` | `<input>.component.wasm` | public contract: in-tree (ADR-008); living path may still use helpers |
| `wasm32-gc` | `wit` | `<input>.wit` | WIT export surface |
| `wasm32-gc` | `all` | `<input>.wasm` + `<input>.component.wasm` | core + component |

`native-*`:

- emit kind、拡張子、object 生成、link 手順は **未決定**（ADR-007 / ADR-045）
- 試験実装の現状のみ `docs/current-state.md` に記録する（本ファイルでも契約化しない）

### `--emit all` 時のファイル衝突

`--emit all` は `wasm32-gc` ターゲットでのみ有効。以下のファイルを生成する:

1. `<input>.wasm` — core Wasm
2. `<input>.component.wasm` — component Wasm

これらは拡張子が異なるため衝突しない。`<input>.wit` と `<input>.world.wit` は `--emit wit` で別途出力するため、`--emit all` では生成されない。

### native ターゲットの出力形式

`native-cpp` / `native-llvm`（scaffold）:

- 出力形式・リンク手順・ABI は **未決定**（ADR-045）
- C99 / C ABI / clang / `llc` 等を契約として固定しない
- 試験実装の現状のみ `docs/current-state.md` に記録する

### ブラウザ向けパッケージング（jco）

`--emit component` で得た `.component.wasm` をブラウザで動かす場合、ユーザーまたは
ツールチェーンが `jco transpile` で ESM + JS glue を生成する。これはコンパイラ外の
手順であり、ADR-008 の in-tree component 生成とは別段である。

```
arukellt compile --target wasm32-gc --emit component input.ark -o app.component.wasm
jco transpile app.component.wasm -o app.dist/
```

---

## Emit surface

ターゲットごとの出力形式（契約）:

| Emit kind | `wasm32` | `wasm32-gc` | `native-*` | Notes |
|-----------|----------|-------------|------------|-------|
| `core-wasm` | Yes | Yes | — | 既定の core Wasm |
| `wat` | Yes | Yes | — | WAT テキスト |
| `component` | No | Yes | — | **Public contract (ADR-008):** in-tree. **Implementation:** living path may still use `wasm-tools` / Python helpers — see `current-state.md` |
| `wit` | No | Yes | — | WIT export surface |
| `all` | No | Yes | — | core + component（契約はコンパイラ内; 実装ギャップは current-state） |
| native emits | — | — | scaffold | 形式・ABI は未決定（ADR-045）。現状は current-state |

複数コンポーネントのリンクは ADR-034（`wac plug`）。ブラウザ向けは上記 jco。

検証の現行 tier（guaranteed / smoke 等）と fixture 状況は `docs/current-state.md` を正本とする。

---

## Alias policy

旧ターゲット名の扱い:

| 旧名 | 扱い |
|------|------|
| `wasm32-wasi-p1` | → `wasm32`（警告 `W0002`） |
| `wasm32-wasi` | → `wasm32`（警告 `W0002`） |
| `wasm32-wasi-p2` | → `wasm32-gc`（警告 `W0002`）。既定 host は WASI P2 |
| `wasm-gc` / `wasm-gc-wasi-p2` | → `wasm32-gc`（警告 `W0002`） |
| `wasm32-wasi-p3` | → `wasm32-gc` + `--wasi p3`（警告 `W0002`）。別ターゲットではない |
| `native` | → `native-cpp` または `native-llvm` を明示（曖昧ならエラー） |
| `wasm32-freestanding` | **受理しない**（ハードエラー）。`wasm32-gc` への自動変換はしない。host / メモリモデル / Component 前提が変わるため alias ではない |

`wasm32` は AtCoder / P1 向け canonical 名のみを指す。旧 freestanding を `wasm32` と書いて
`wasm32-gc` へ誘導する規則は置かない（入力文字列が衝突し実装不能なため）。

---

## Runtime model terminology

| RuntimeModel | Meaning |
|-------------|---------|
| `Wasm32Linear` | Linear memory + WASI P1 |
| `Wasm32Gc` | Linear memory + Wasm GC + Component Model |
| `NativeCpp` | C++ / C99 native backend（scaffold） |
| `NativeLlvm` | LLVM IR native backend（scaffold; 意味論は ADR-045） |

現行の稼働状態・件数は `docs/current-state.md` を参照。

---

## 検証サーフェス

各ターゲットの検証状態:

| Label | Meaning |
|-------|---------|
| **guaranteed** | CI で毎 push/PR 実行。失敗は merge block。 |
| **smoke** | CI で実行するが失敗は non-blocking、または opt-in flag。 |
| **scaffold** | コード存在するが広域保証対象外。 |
| **none** | 実装なし。 |

### `wasm32` — AtCoder・競技プログラミング用

| Surface | Status | Detail |
|---------|--------|--------|
| parse | guaranteed | `run` + `module-run` + `diag` + `module-diag` fixtures |
| typecheck | guaranteed | same fixture set |
| compile (core Wasm) | guaranteed | all `run`/`module-run` fixtures compile |
| run (wasmtime) | guaranteed | stdout compared against `.expected` |
| emit component | n/a | `wasm32-gc`-only |
| emit WIT | n/a | `wasm32-gc`-only |
| host capabilities | guaranteed | `--deny-clock`, `--deny-random` hard-error placeholders |
| determinism | smoke | baselines spot-checked |
| validator pass | guaranteed | `wasmparser` validation post-emit |

### `wasm32-gc` — メインターゲット

| Surface | Status | Detail |
|---------|--------|--------|
| parse | guaranteed | shared frontend |
| typecheck | guaranteed | shared frontend |
| compile (core Wasm) | guaranteed | `t3-run` + `t3-compile` fixtures |
| run (wasmtime) | guaranteed | `t3-run` fixtures with stdout comparison |
| emit component | smoke | `component-compile` fixtures（in-tree; 現行は current-state） |
| emit WIT | smoke | `--emit wit` tested in component-compile fixtures |
| host capabilities | guaranteed | WASI imports conditionally emitted per reachability |
| determinism | smoke | baselines spot-checked |
| validator pass | guaranteed | `wasmparser` validation post-emit |
| compile-error | guaranteed | `compile-error` fixtures verify expected failures |

### `native-cpp` / `native-llvm` — ネイティブバックエンド

Status: **scaffold** — asm stub only.

| Surface | Status | Detail |
|---------|--------|--------|
| parse / typecheck | guaranteed | shared frontend |
| compile | scaffold | `native::emit_native_scaffold` asm stub |
| run | none | `run_supported=false` |

### CI job mapping

| CI job | Target | What runs |
|--------|--------|-----------|
| `verification` | all | `python3 scripts/manager.py verify` |
| `selfhost` | `wasm32`/`wasm32-gc` | fixpoint, fixture parity, CLI parity, diagnostic parity |
| `docs` | docs | `python3 scripts/check/check-docs-consistency.py` |

---

## Capability surface

全ホスト相互作用は `std::host::*` namespace 経由。

### Host modules

| Module | Functions | Status | Targets |
|--------|-----------|--------|---------|
| `std::host::stdio` | 3 | available | all |
| `std::host::clock` | 1 | available | all |
| `std::host::random` | 3 | available | all |
| `std::host::env` | 5 | available | all (partial `wasm32`) |
| `std::host::fs` | 3 | available | all |
| `std::host::process` | 2 | available | all |
| `std::host::http` | 2 | not user-reachable | — |
| `std::host::sockets` | 1 | not user-reachable | — |
| `std::host::udp` | 1 | not user-reachable | — |

### Target compatibility matrix

| Function | `wasm32` | `wasm32-gc` |
|----------|----------|-------------|
| `stdio::print` | ✓ | ✓ |
| `stdio::println` | ✓ | ✓ |
| `stdio::eprintln` | ✓ | ✓ |
| `clock::monotonic_now` | ✓ | ✓ |
| `random::random_i32` | ✓ | ✓ |
| `random::random_i32_range` | ✓ | ✓ |
| `random::random_bool` | ✓ | ✓ |
| `env::args` | ✓ | ✓ |
| `env::arg_count` | ✓ | ✓ |
| `env::arg_at` | ✓ | ✓ |
| `env::var` | ✗ | ✓ |
| `env::has_flag` | ✓ | ✓ |
| `fs::read_to_string` | ✓ | ✓ |
| `fs::write_string` | ✓ | ✓ |
| `fs::write_bytes` | ✓ | ✓ |
| `process::exit` | ✓ | ✓ |
| `process::abort` | ✓ | ✓ |
| `http::request` | — | — |
| `http::get` | — | — |
| `sockets::connect` | E0500 | — |

`env::var` は現在の `wasm32` **実装**では未対応である。WASI Preview 1 自体は
`environ_get` / `environ_sizes_get` を定義している。未対応の理由（emitter・harness・
stdlib adapter 等）と進捗は `docs/current-state.md` に記録する。

### CLI capability flags

| Flag | Scope | Enforcement | Effect |
|------|-------|-------------|--------|
| `--deny-fs` | Filesystem | Runtime (WASI) | Blocks all directory grants; overrides `--dir` |
| `--deny-clock` | Clock | Compile-time (MIR scan) | Hard error if clock intrinsic referenced |
| `--deny-random` | Random | Compile-time (MIR scan) | Hard error if random intrinsic referenced |
| `--dir PATH` | Filesystem | Runtime (WASI preopened) | Grants read-write access to `PATH` |
| `--dir PATH:ro` | Filesystem | Runtime (WASI preopened) | Grants read-only access |

Default policy: stdio **allow** (cannot be denied), filesystem **deny**, clock/random **allow**.

### Known limitations

1. `env::var` は現在の `wasm32` 実装で未対応（WASI P1 に `environ_get` はある。実装ギャップは current-state）
2. HTTP/sockets/UDP not user-reachable (#633)
3. No `--deny-stdio` flag
4. No per-function capability deny (module-level only)
5. Filesystem deny-by-default but not compile-time scan (runtime failure without `--dir`)
6. Node.js 実行時の stdin 非対応（`node:wasi` の制限）

---

## SIMD

SIMD 機能のターゲット別状況（ADR-037 参照）:

| Feature | `wasm32` | `wasm32-gc` | `native` |
|---------|----------|-------------|----------|
| v128 first-class type | ✅ native SIMD | ✅ native SIMD | #699 |
| `std::simd` lane types | ✅ 11 types | ✅ 11 types | #699 |
| `std::wasm` raw v128 intrinsics | ✅ | ✅ | #699 |
| GC struct/array v128 field | n/a (linear) | fixtures ready | #699 |
| shuffle / swizzle | deferred | deferred | deferred |

`wasm32` は iwasm 2.4.1 が SIMD に対応しているため、現行実装はネイティブ SIMD 命令を使用する。
スカラー展開（`call_simd_scalar*.ark`）はフォールバックコードとして存在するが、
現在の `is_simd_target()` は全ターゲットで `true` を返す。

**ADR-037 契約ギャップ:** 提案は `portable_simd_lowering` / `wasm_raw_v128` /
`wasm_relaxed_simd` の三軸。現行の単一 `is_simd_target()` は未分離。
API 面も #698 の lane モジュール + 無印 `v128` が先行しており、ADR-037 の
nominal 型移行は未着手（`current-state.md` ADR contract gaps）。

---

