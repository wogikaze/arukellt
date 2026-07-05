# ADR-007: コンパイルターゲット整理

ステータス: **DECIDED** — ターゲットを3系統に確定（wasm32 / wasm32-gc / native）

決定日: 2026-03-26（2026-07 改定: T1-T5 命名を廃止、ターゲット名ベースに再構成）

---

## 文脈

arukellt は複数のランタイム・用途向けにコードを生成する必要がある。
ターゲットをランタイム軸で整理すると、言語機能セット（GC有無、WASI版）が不明確になる。
ADR-002（Wasm GC 採用）・ADR-005（LLVM 従属）・ADR-006（ABI 3層）との整合を明示するため、
**「言語機能セット × 実行環境」軸** でターゲットを確定する。

旧版では T1-T5 の5段階ティアを定義していたが、実態に合わせて
ターゲット名ベースの3系統に再構成する。

---

## 決定

ターゲットを以下の 3 系統に確定する。

### `wasm32` — AtCoder・競技プログラミング用

| 項目 | 内容 |
|------|------|
| メモリモデル | Linear memory（No GC） |
| 実行環境 | wabt 1.0.34 + iwasm 2.4.1 |
| WASI | Preview 1 |
| Component Model | なし |
| 出力 | `.wasm`, `.wat` |
| 主な用途 | AtCoder、競技プログラミング |

**ADR-002 例外**: ADR-002 は Wasm GC 採用を決定しているが、AtCoder が
wabt 1.0.34 + iwasm 2.4.1 環境に固定されている事実を優先し、このターゲットのみ
GC なし linear memory を維持する。

**廃止条件**: AtCoder 側が Wasm GC に対応した場合、即座に廃止し `wasm32-gc` に統合する。

### `wasm32-gc` — 最新機能・ブラウザ・JavaScript用（メインターゲット）

| 項目 | 内容 |
|------|------|
| メモリモデル | Linear memory + Wasm GC |
| 実行環境 | Chrome, Node.js, wasmtime, jco |
| WASI | **P2（デフォルト）または P3（`--wasi p3`）**。`none` / `p1` は指定不可 |
| Component Model | あり（canonical ABI） |
| 出力 | `.wasm`, `.wat`, `.wit`, `.component.wasm`, `.core.wasm`, `world.wit` |
| 主な用途 | ブラウザ実行、Node.js 実行、サーバーサイド、CLI ツール |

ADR-002（GC 採用）・ADR-006（Layer 2B ABI）の**正規ターゲット**。
言語意味論の基準はこのターゲットで定義する。

**WASI バージョンの制約**: `wasm32-gc` は Component Model + WASI P2/P3 のみを前提とする。
`--wasi none` や `--wasi p1` を指定した場合は **コンパイルエラー** とする。
ブラウザが WASI を用意していないことは jco transpile が解決するため、
WASI 非依存のモード（旧 `wasm32-freestanding` 相当）は提供しない。

#### `--wasi` フラグ

| 値 | 意味 | 備考 |
|----|------|------|
| `p2`（デフォルト） | WASI Preview 2 | wasmtime、jco transpile 後のブラウザ/Node.js |
| `p3` | WASI Preview 3 | async-first、将来対応 |
| `none` | **指定不可** | コンパイルエラー |
| `p1` | **指定不可** | コンパイルエラー。P1 は `wasm32` ターゲットを使用 |

#### 実行環境の違い

| 環境 | WASI | 備考 |
|------|------|------|
| Chrome | なし（jco が変換） | ブラウザは WASI を用意しない。jco transpile で WASI P2 imports を JS glue に変換 |
| Node.js | なし（jco が変換） | `node:wasi` (P1) は使わない。jco transpile 後の ESM を実行 |
| wasmtime | P2/P3 | フル機能。サーバーサイド実行の主戦場 |
| jco | — | Component → ESM 変換ツール。実行エンジンではない |

> **注意**: 旧 `wasm32-freestanding`（T2）の `arukellt_io` ホストモジュール経由の
> stdio は廃止する。`wasm32-gc` では stdio も WASI P2 経由（`wasi:cli/stdout` 等）
> とし、ブラウザ向けは jco transpile で JS glue に変換する。

#### Component Model 変換フロー

```
[あなたのコード]
  ↓
[Core Wasm 実装]
  - i32/i64/f32/f64
  - linear memory, GC
  - cabi_realloc
  - WIT export に対応する core 関数
  - WIT import を呼ぶための lowered import
  ↓
[Component 化]
  - world.wit を読む
  - canon lift で core export を component export へ
  - canon lower で component import を core import へ
  ↓
[app.component.wasm]
  ↓ jco transpile
[Node.js で読める ESM + core wasm + JS glue]
  ↓
[node dist/app.js]
```

#### Node.js の stdin 非対応問題

Node.js の `node:wasi` (WASI P1) は使用しない。jco transpile 後の ESM を実行するが、
jco が生成する JS glue の stdin 扱いに制限がある場合がある。
Node.js 実行はあくまでおまけであり、stdin を必要とするプログラムの
Node.js 実行は制約を受ける場合がある。

### `native-cpp` / `native-llvm` — ネイティブバックエンド

| 項目 | 内容 |
|------|------|
| バックエンド | C++ (`native-cpp`) または LLVM IR (`native-llvm`) |
| プラットフォーム | Linux / Windows / macOS |
| ABI | C ABI（System V AMD64 / Windows x64） |
| 従属 | ADR-005: Wasm 意味論に従属 |
| 主な用途 | ローカルデバッグ、性能比較 |

ADR-005 に従い、ネイティブバックエンドは **Wasm 意味論の再現**に留める。
native 専用の言語機能・最適化は追加しない。

---

## ターゲット優先順位

```
実装優先度: wasm32-gc（メイン） → wasm32（AtCoder維持） → native
言語意味論の基準: wasm32-gc
ADR-002 GC採用: wasm32-gc, native-llvm（wasm32 のみ例外）
ADR-005 LLVM従属: native-llvm
ADR-006 ABI 3層: wasm32-gc = Layer 2、native = Layer 3
```

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
| `wasm32-gc` | Chrome, Node.js, wasmtime, jco | ブラウザが主、Node.jsはおまけ、wasmtimeはフル機能 |
| `native-cpp` | gcc/clang (C++ コンパイル) | ローカルデバッグ |
| `native-llvm` | LLVM | ローカルデバッグ、性能比較 |

---

## 禁止事項

- `wasm32` の linear memory 実装を `wasm32-gc` に持ち込まない
- `native` で `wasm32-gc` にない言語機能を追加しない（ADR-005）
- `wasm32` は AtCoder が GC 対応したら即座に廃止する。将来に向けた拡張は `wasm32-gc` に行う

---

## 未対応機能を使おうとした際の挙動

ターゲットが対応していない機能をユーザーが使おうとした場合の挙動:

### `wasm32` — WASI P2以降・未対応Wasm機能

- **WASI P2以降の機能**（`environ_get`、HTTP、sockets 等）を使用しようとした場合:
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

出力ファイル名は入力ファイル名を元に `<input>.*` の形式で生成する（仮置き）。

| ターゲット | 出力 | 備考 |
|-----------|------|------|
| `wasm32` | `<input>.wasm`, `<input>.wat` | |
| `wasm32-gc` | `<input>.wasm`, `<input>.wat`, `<input>.wit`, `<input>.component.wasm`, `<input>.core.wasm`, `<input>.world.wit` | Component化・jco transpile は別途実行 |
| `native-cpp` / `native-llvm` | 別途ADRで決定 | |

> **注意**: `<input>.*` の命名規則は仮置きである。最終的な命名規則は別途 issue で決定する。

---

## Emit surface

ターゲットごとの出力形式:

| Emit kind | `wasm32` | `wasm32-gc` | Notes |
|-----------|----------|-------------|-------|
| `core-wasm` | Yes | Yes | default production path |
| `wat` | Yes | Yes | WAT text format |
| `component` | No | Yes | requires external `wasm-tools` + adapter |
| `wit` | No | Yes | WIT export surface generation |
| `all` | No | Yes | emits both core Wasm and component artifacts |

Component output は `wasm-tools component embed` → `wasm-tools component new` で生成する。
複数コンポーネントのリンクは `wac plug` を使用する。
ブラウザ向けはさらに `jco transpile` で ESM + JS glue に変換する。

Component output tier:
- Core Wasm output (`--emit core-wasm`): **guaranteed** for `wasm32` and `wasm32-gc`
- Component output (`--emit component`): **smoke** tier for `wasm32-gc`
- `wasm-tools` 未インストール時は component-compile fixtures を skip（fail にしない）

---

## Alias policy

旧ターゲット alias は受理されるが `W0002` を出す。canonical 名を使うこと。

| 旧名 | 新名 |
|------|------|
| `wasm32-wasi-p1` | `wasm32` |
| `wasm32-wasi` | `wasm32` |
| `wasm32-freestanding` | `wasm32-gc` |
| `wasm32` (旧 freestanding) | `wasm32-gc` |
| `wasm-gc` | `wasm32-gc` |
| `wasm32-wasi-p2` | `wasm32-gc` |
| `wasm-gc-wasi-p2` | `wasm32-gc` |
| `wasm32-wasi-p3` | `wasm32-gc` (+ `--wasi p3` flag) |
| `native` | `native-cpp` または `native-llvm` |

---

## Runtime model terminology

| RuntimeModel | Meaning | Current state |
|-------------|---------|---------------|
| `Wasm32Linear` | Linear memory + WASI P1 (wabt/iwasm) | Active |
| `Wasm32Gc` | Linear memory + Wasm GC + Component Model | Active |
| `NativeCpp` | C++ native backend | Scaffold |
| `NativeLlvm` | LLVM IR native backend | Scaffold |

---

<!-- BEGIN GENERATED:CURRENT_STATE_TARGET_SUMMARY_SOURCE -->
| Target | Tier | ADR-013 Tier | Status | Run | Notes |
|--------|------|--------------|--------|-----|-------|
| `wasm32-wasi-p1` | T1 | supported | stable | Yes | Supported: full fixture coverage, AtCoder/competition target |
| `wasm32-freestanding` | T2 | scaffold | scaffold | No | Scaffold: minimal core Wasm emitter proof and validator pass; no runtime execution support yet |
| `wasm32-wasi-p2` | T3 | primary | stable | Yes | Primary (ADR-013): WASI P2 linear-memory path (default target), all CI gates apply |
| `native` | T4 | scaffold | scaffold | No | Scaffold: selfhost-native asm stub via `native::emit_native_scaffold`; compile-only proof at `tests/fixtures/t4/native_scaffold.ark` (#641). |
| `wasm32-wasi-p3` | T5 | not-started | not-started | No | Not started: target id exists but no backend, runtime, or scaffold code |
<!-- END GENERATED:CURRENT_STATE_TARGET_SUMMARY_SOURCE -->

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
| emit component | smoke | `component-compile` fixtures; skip if `wasm-tools` absent |
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

`env::var` は `wasm32` で利用不可（WASI Preview 1 が `environ_get` を import しない）。

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

1. `env::var` unavailable on `wasm32` (WASI P1 lacks `environ_get`)
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

`wasm32` は iwasm 2.4.1 が SIMD に対応しているため、ネイティブ SIMD 命令を使用する。
スカラー展開（`call_simd_scalar*.ark`）はフォールバックコードとして存在するが、
現在の `is_simd_target()` は全ターゲットで `true` を返す。

---

## 関連

- ADR-002: Wasm GC 採用（`wasm32` の例外根拠）
- ADR-005: LLVM バックエンドの役割制限（`native-llvm`）
- ADR-006: 公開 ABI 3層構造（`wasm32-gc` / `native`）
- ADR-013: Primary Target（`wasm32-gc` primary 根拠）
- ADR-020: T2 I/O surface
- ADR-035: Wasm GC Implementation Plan
- ADR-037: std::simd — Explicit SIMD Library API
- `docs/platform/abi.md`: ABI 詳細
