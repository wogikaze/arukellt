# ADR-007: コンパイルターゲット整理

ステータス: **DECIDED** — ターゲットを5つに確定（T1/T2/T3/T4/T5）

決定日: 2026-03-26

---

## 文脈

arukellt は複数のランタイム・用途向けにコードを生成する必要がある。
ターゲットをランタイム軸で整理すると、言語機能セット（GC有無、WASI版）が不明確になる。
ADR-002（Wasm GC 採用）・ADR-005（LLVM 従属）・ADR-006（ABI 3層）との整合を明示するため、
**「言語機能セット × WASI版」軸** でターゲットを確定する。

---

## 決定

ターゲットを以下の 5 つに確定する。

### T1: wasm32-wasi-p1（AtCoder 特例）

| 項目 | 内容 |
|------|------|
| メモリモデル | Linear memory（No GC） |
| WASI | Preview 1 |
| Component Model | なし |
| ランタイム | iwasm 2.4.1 |
| 主な用途 | AtCoder、競技プログラミング |
| 現状 | **現行 v0 実装** |

**ADR-002 例外**: ADR-002 は Wasm GC 採用を決定しているが、AtCoder が WASI p1 + linear memory 環境に固定されている事実を優先し、このターゲットのみ GC なし linear memory を維持する。

### T2: wasm32-freestanding（ブラウザ JS / 組み込み）

| 項目 | 内容 |
|------|------|
| メモリモデル | Wasm GC（Reference Types） |
| WASI | なし |
| Component Model | なし |
| ランタイム | ブラウザ（主）、wasmtime（デバッグ用のみ） |
| 主な用途 | ブラウザ JS 呼び出し、軽量組み込み |

wasmtime での T2 実行はデバッグ目的に限る。
本来の用途はブラウザ環境（V8/SpiderMonkey の Wasm GC 対応済み）。

### T3: wasm32-wasi-p2（メインターゲット）

| 項目 | 内容 |
|------|------|
| メモリモデル | Wasm GC |
| WASI | Preview 2 |
| Component Model | あり（canonical ABI） |
| ランタイム | wasmtime |
| テスト | <https://wa.dev/mizchi:tmgrammar> |
| 主な用途 | サーバーサイド、クラウド、CLI ツール |

ADR-002（GC 採用）・ADR-006（Layer 2B ABI）の**正規ターゲット**。
言語意味論の基準はこのターゲットで定義する。

### T4: native（LLVM 従属バックエンド）

| 項目 | 内容 |
|------|------|
| バックエンド | LLVM IR |
| プラットフォーム | Linux / Windows / macOS |
| ABI | C ABI（System V AMD64 / Windows x64） |
| 従属 | ADR-005: Wasm 意味論に従属 |
| 主な用途 | ローカルデバッグ、性能比較 |

ADR-005 に従い、LLVM バックエンドは **Wasm 意味論の再現**に留める。
native 専用の言語機能・最適化は追加しない。

### T5: wasm32-wasi-p3（将来ターゲット）

| 項目 | 内容 |
|------|------|
| メモリモデル | Wasm GC |
| WASI | Preview 3 |
| Component Model | あり |
| ランタイム | wasmtime（対応後） |
| テスト | <https://wa.dev/mizchi:tmgrammar> |
| 状態 | 仕様策定中。着手は WASI p3 安定化後 |

T3 の後継。WASI p3 は async-first 設計で、長期的なメインターゲット候補。

---

## ターゲット優先順位

```
実装優先度: T1（現行） → T3（メイン） → T2 → T4 → T5
言語意味論の基準: T3
ADR-002 GC採用: T2, T3, T5（T1 のみ例外）
ADR-005 LLVM従属: T4
ADR-006 ABI 3層: T2/T3 = Layer 2、T4 = Layer 3
```

---

## 禁止事項

- T1 の linear memory 実装を T2/T3 に持ち込まない
- T4（LLVM）で T3 にない言語機能を追加しない（ADR-005）
- T5 への着手は WASI p3 仕様確定前には行わない

---

## Emit surface

ターゲットごとの出力形式:

| Emit kind | T1 | T3 | Notes |
|-----------|----|----|-------|
| `core-wasm` | Yes | Yes | default production path |
| `component` | No | Yes | requires external `wasm-tools` + adapter |
| `wit` | No | Yes | WIT export surface generation |
| `all` | No | Yes | emits both core Wasm and component artifacts |

Component output は `wasm-tools component embed` → `wasm-tools component new` で生成する。
WASI P1 adapter (`wasi_snapshot_preview1.reactor.wasm`) が必要。
複数コンポーネントのリンクは `wac plug` を使用する。

Component output tier:
- Core Wasm output (`--emit core-wasm`): **guaranteed** for T1 and T3
- Component output (`--emit component`): **smoke** tier for T3
- `wasm-tools` 未インストール時は component-compile fixtures を skip（fail にしない）

---

## Alias policy

旧ターゲット alias は受理されるが `W0002` を出す。canonical 名を使うこと。

- `wasm32-wasi` → `wasm32-wasi-p1`
- `wasm-gc` → `wasm32-wasi-p2`
- `wasm-gc-wasi-p2` → `wasm32-wasi-p2`
- `wasm32` → `wasm32-freestanding`

---

## Runtime model terminology

| RuntimeModel | Meaning | Current state |
|-------------|---------|---------------|
| `T1LinearP1` | Linear memory + WASI P1 | Active |
| `T3WasmGcP2` | Wasm GC-native runtime on `wasm32-wasi-p2` | Active |
| `T4LlvmScaffold` | LLVM native scaffold | Optional / not implemented |

---

## 検証サーフェス

各ターゲットの検証状態:

| Label | Meaning |
|-------|---------|
| **guaranteed** | CI で毎 push/PR 実行。失敗は merge block。 |
| **smoke** | CI で実行するが失敗は non-blocking、または opt-in flag。 |
| **scaffold** | コード存在するが広域保証対象外。 |
| **none** | 実装なし。 |

### T1 — `wasm32-wasi-p1`

| Surface | Status | Detail |
|---------|--------|--------|
| parse | guaranteed | `run` + `module-run` + `diag` + `module-diag` fixtures |
| typecheck | guaranteed | same fixture set |
| compile (core Wasm) | guaranteed | all `run`/`module-run` fixtures compile |
| run (wasmtime) | guaranteed | stdout compared against `.expected` |
| emit component | n/a | T3-only |
| emit WIT | n/a | T3-only |
| host capabilities | guaranteed | `--deny-clock`, `--deny-random` hard-error placeholders |
| determinism | smoke | baselines spot-checked |
| validator pass | guaranteed | `wasmparser` validation post-emit |

### T3 — `wasm32-wasi-p2`

| Surface | Status | Detail |
|---------|--------|--------|
| parse | guaranteed | shared frontend |
| typecheck | guaranteed | shared frontend |
| compile (core Wasm) | guaranteed | `t3-run` + `t3-compile` fixtures |
| run (wasmtime) | guaranteed | `t3-run` fixtures with stdout comparison |
| emit component | smoke | `component-compile` fixtures; skip if `wasm-tools` absent |
| emit WIT | smoke | `--emit wit` tested in component-compile fixtures |
| host capabilities | guaranteed | WASI P2 imports conditionally emitted per reachability |
| determinism | smoke | baselines spot-checked |
| validator pass | guaranteed | `wasmparser` validation post-emit |
| compile-error | guaranteed | `compile-error` fixtures verify expected failures |

### T2 — `wasm32-freestanding`

Status: **scaffold** — compile-only core Wasm proof, no runtime execution.

| Surface | Status | Detail |
|---------|--------|--------|
| compile (core Wasm) | scaffold | `t2_scaffold.ark` compile + validate |
| run | none | no runtime/browser execution |
| validator pass | scaffold | `wasmparser::Validator::validate_all` |

### T4 — native

Status: **scaffold** — asm stub only.

| Surface | Status | Detail |
|---------|--------|--------|
| parse / typecheck | guaranteed | shared frontend |
| compile | scaffold | `native::emit_native_scaffold` asm stub |
| run | none | `run_supported=false` |

### T5 — `wasm32-wasi-p3`

Status: **not-started** — target ID exists, no backend.

### CI job mapping

| CI job | Target | What runs |
|--------|--------|-----------|
| `verification` | all | `python3 scripts/manager.py verify` |
| `selfhost` | T1/T3 | fixpoint, fixture parity, CLI parity, diagnostic parity |
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
| `std::host::env` | 5 | available | all (partial T1) |
| `std::host::fs` | 3 | available | all |
| `std::host::process` | 2 | available | all |
| `std::host::http` | 2 | not user-reachable | — |
| `std::host::sockets` | 1 | not user-reachable | — |
| `std::host::udp` | 1 | not user-reachable | — |

### Target compatibility matrix

| Function | T1 (wasm32-wasi-p1) | T3 (wasm32-wasi-p2) |
|----------|---------------------|---------------------|
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

`env::var` は T1 で利用不可（WASI Preview 1 が `environ_get` を import しない）。

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

1. `env::var` unavailable on T1 (WASI P1 lacks `environ_get`)
2. HTTP/sockets/UDP not user-reachable (#633)
3. No `--deny-stdio` flag
4. No per-function capability deny (module-level only)
5. Filesystem deny-by-default but not compile-time scan (runtime failure without `--dir`)

---

## SIMD

SIMD 機能のターゲット別状況（ADR-037 参照）:

| Feature | T1 | T2 | T3 | T4 | T5 |
|---------|----|----|----|----|-----|
| v128 first-class type | ✅ native SIMD | ✅ native SIMD | ✅ native SIMD | #699 | not started |
| `std::simd` lane types | ✅ 11 types | ✅ 11 types | ✅ 11 types | #699 | not started |
| `std::wasm` raw v128 intrinsics | ✅ | ✅ | ✅ | #699 | not started |
| GC struct/array v128 field | n/a (linear) | n/a | fixtures ready | #699 | not started |
| shuffle / swizzle | deferred | deferred | deferred | deferred | deferred |

T1 は iwasm 2.4.1 が SIMD に対応しているため、ネイティブ SIMD 命令を使用する。
スカラー展開（`call_simd_scalar*.ark`）はフォールバックコードとして存在するが、
現在の `is_simd_target()` は全ターゲットで `true` を返す。

---

## 関連

- ADR-002: Wasm GC 採用（T1 の例外根拠）
- ADR-005: LLVM バックエンドの役割制限（T4）
- ADR-006: 公開 ABI 3層構造（T2/T3/T4）
- ADR-013: Primary Target（T3 primary 根拠）
- ADR-020: T2 I/O surface
- ADR-035: Wasm GC Implementation Plan
- ADR-037: std::simd — Explicit SIMD Library API
- `docs/platform/abi.md`: ABI 詳細
