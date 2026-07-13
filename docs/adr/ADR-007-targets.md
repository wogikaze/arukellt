# ADR-007: コンパイルターゲット整理

ステータス: **ACCEPTED** — canonical は `wasm32` / `wasm32-gc` / `native-*`（scaffold）

決定日: 2026-03-26  
改訂日: 2026-07-13 — default Wasm feature emit の許可集合を契約化（iwasm / wasmtime∩Node∩Browser∩jco）

---

## 文脈

複数ランタイム向けにコードを生成するため、ターゲットを
**表現モデル × 製品 profile** で固定する。旧 T1–T5 表記は廃止する。
native の意味論・ABI は [ADR-045](ADR-045-llvm-scope-withdrawn.md) まで未決定。

現行のランタイム版・fixture・CI・host 到達可能性は
[`docs/platform/target-runtime-and-surfaces.md`](../platform/target-runtime-and-surfaces.md)
および `docs/current-state.md` を正本とする。

---

## 決定

### 1. Canonical ターゲット系統

| 系統 | 役割 | 表現 | Support tier（定義） |
|------|------|------|----------------------|
| `wasm32` | AtCoder / 非 GC 互換 | linear memory（同一言語意味の lowering） | **supported** |
| `wasm32-gc` | 主製品 profile | Wasm GC | **primary**（ADR-013） |
| `native-cpp` / `native-llvm` | 試験 | 未決定 | **scaffold** |

### 2. `wasm32-gc` は製品 profile として束ねる

CLI の `--target wasm32-gc` は、当面次を**既定で束ねた製品 profile**である:

| 軸 | 既定 |
|----|------|
| TargetArchitecture | Wasm32Gc（値表現 = Wasm GC） |
| HostProfile | WASI P2（`--wasi p3` で P3） |
| ArtifactKind | core Wasm 既定。`--emit component` / `wit` / `all` 可（ADR-008） |

内部モデルとしては三軸が直交しうる（将来 `--host none` 等）。現状 CLI は
`--wasi none` / `p1` を `wasm32-gc` 上で拒否する（コンパイルエラー）。
WASI 非依存の旧 freestanding は提供しない。

`jco` は実行エンジンではなく **packaging tool**（component → ESM）。
ブラウザ経路の検証記録は `docs/research/target-runtime-verification.md`。

### 3. `wasm32`

- 言語意味論は `wasm32-gc` と同一。表現だけ linear lowering（ADR-002）
- WASI Preview 1 を前提とする互換ターゲット
- AtCoder が Wasm GC に対応したら廃止し `wasm32-gc` に統合する

### 4. `native-*`

- scaffold のみ（`native_scaffold`）。emit kind・拡張子・object 生成・link・ABI・FFI は **未決定**
- 試験実装の現状だけ current-state / platform 文書に置く。本 ADR では契約化しない

### 5. Emit 契約（Wasm のみ）

<a id="emit-surface"></a>

| Emit kind | `wasm32` | `wasm32-gc` | `native-*` |
|-----------|----------|-------------|------------|
| `core-wasm` | Yes | Yes | —（未決定） |
| `wat` | Yes | Yes | — |
| `component` | No | Yes（in-tree, ADR-008） | — |
| `wit` | No | Yes | — |
| `all` | No | Yes（core + component） | — |

出力ファイル名の stem 規則など CLI 詳細は
[`docs/platform/target-runtime-and-surfaces.md`](../platform/target-runtime-and-surfaces.md#emit-surface)。

### 5.1 Default Wasm feature emit（許可集合）

<a id="default-wasm-feature-emit"></a>

コンパイラが **default で core Wasm に出してよい機能**は、ターゲットごとの
実行／packaging ゲートの積集合に限定する。証拠は
[`docs/research/target-runtime-verification.md`](../research/target-runtime-verification.md)
付録 A および `docs/research/wat-probes/`（2026-07-13）。

| ターゲット | ゲート（積集合） | 契約上の default emit 上限 |
|-----------|------------------|---------------------------|
| `wasm32` | pinned **iwasm**（WAMR）が実行できる範囲 | **Wasm 2.0 Core**（fixed SIMD 込み）。Wasm 3.0 系は default 禁止 |
| `wasm32-gc` | **wasmtime ∩ Node(V8) ∩ Browser(Chrome/V8) ∩ jco≥1.25.2 transpile** | 下表。**multiple memories は default 禁止**（jco が拒否） |

`jco` は実行エンジンではなく packaging gate（component → ESM）。browser/Node 経路で
component を配る前提のため、`wasm32-gc` の default emit 積集合に含める。

#### `wasm32` — default emit OK / 禁止

| default emit OK | default 禁止（フォールバックまたはコンパイルエラー） |
|-----------------|------------------------------------------------------|
| Wasm 1.0 Core | Memory64 / Table64 |
| multi-value / reference types / bulk memory | Multiple memories |
| fixed SIMD `v128`（trunc_sat SIMD 含む） | Tail call / `return_call_ref` |
| sign-extension / trunc_sat（scalar） | Typed function references / `br_on_null` |
| | GC（struct / array / i31 / recursive types） |
| | Exception handling（`try_table` / legacy） |
| | Extended const / Relaxed SIMD |
| | Threads / Atomics（Wasm 3.0 Core 外） |

#### `wasm32-gc` — default emit OK / 禁止

| default emit OK（積集合で成功） | default 禁止 |
|--------------------------------|--------------|
| Wasm 1.0 / 2.0 Core | **Multiple memories**（jco transpile 非対応） |
| Wasm GC（struct / array / i31 / recursive types） | JS String Builtins（JS embedding。core emit 契約外） |
| typed function references / `call_ref` / `br_on_null` | Threads / Atomics（独立提案。default に入れない） |
| Memory64 / Table64 | Legacy EH（`try`/`catch`。現行 EH は `try_table`） |
| Tail call / `return_call_ref` | Branch Hinting（未契約；metadata のみ） |
| EH `try_table` | |
| Extended const | |
| Relaxed SIMD | |

詳細表・再測手順・toolchain 版は platform 文書を正本の運用面とする:

[`docs/platform/target-runtime-and-surfaces.md`](../platform/target-runtime-and-surfaces.md#default-wasm-feature-emit)

opt-in（明示フラグや将来プロファイル）で積集合外の機能を出すことは妨げないが、
**default の `--target wasm32` / `wasm32-gc` では出さない**。
実装ゲートの揃い具合は living（`docs/current-state.md`）。

### 6. Alias policy

| 旧名 | 扱い |
|------|------|
| `wasm32-wasi-p1` / `wasm32-wasi` | → `wasm32`（`W0002`） |
| `wasm32-wasi-p2` / `wasm-gc` / `wasm-gc-wasi-p2` | → `wasm32-gc`（`W0002`） |
| `wasm32-wasi-p3` | → `wasm32-gc` + `--wasi p3`（`W0002`） |
| `native` | `native-cpp` または `native-llvm` を明示（曖昧ならエラー） |
| `wasm32-freestanding` | **ハードエラー**（alias にしない） |

`wasm32` は AtCoder/P1 向け canonical のみ。旧 freestanding を `wasm32` 経由で
`wasm32-gc` へ誘導しない。

### 7. Capability error の原則

ターゲットが対応しない言語機能・host API を使う場合は **コンパイルエラー**
（または明示の target capability error）。実装未了は「仕様上無い」と混同しない
（例: `env::var` と WASI P1 — platform / current-state）。

現行の host 到達可能性・関数別対応表（Capability surface）は
[`docs/platform/target-runtime-and-surfaces.md`](../platform/target-runtime-and-surfaces.md#capability-surface)。

### 8. Support tier 語彙

| Tier | 意味 |
|------|------|
| primary | リリース品質の基準とする唯一の target（ADR-013）。リリース時に全 gate 必須。各 commit が常に release-ready とは限らない |
| supported | 日常利用可。失敗は merge を止めない場合あり |
| scaffold | 試験のみ。広域保証なし |

現行のどの surface が guaranteed/smoke かは platform / current-state。

### 9. Target profile の軸（project-state）

`docs/data/project-state.toml` の target 行は次を混同しない:

| フィールド | 意味 | 根拠 |
|------------|------|------|
| `support_tier` | primary / supported / scaffold / not-started | 本 ADR §8 / ADR-013 |
| `implementation_state` | complete / partial / scaffold / unimplemented | 実装の完成度（living） |
| `contract_stability` | CLI 名・公開 target 契約の安定度 | 本節。ADR-014 の API stability ではない |

`contract_stability = stable` は「target 識別子と CLI 契約が安定」を意味し、
`implementation_state = partial` と両立しうる（例: `wasm32-gc`）。
言語機能・stdlib API のラベルは引き続き ADR-014。

---

## 禁止事項

- `wasm32` の linear 実装を `wasm32-gc` の意味論に持ち込まない
- native の ABI・言語機能を ADR-045 後継採択前に固定しない
- primary を複数にしない

---

## 関連

- [ADR-002](ADR-002-memory-model.md) — GC 意味論、`wasm32` lowering
- [ADR-006](ADR-006-abi-policy.md) — 安定境界は WIT/canonical
- [ADR-008](ADR-008-component-wrapping.md) — component in-tree
- [ADR-013](ADR-013-primary-target.md) — primary = `wasm32-gc`
- [ADR-037](ADR-037-std-simd.md) — SIMD 軸（fixed / relaxed）
- [ADR-045](ADR-045-llvm-scope-withdrawn.md) — native 未決定
- [`docs/platform/target-runtime-and-surfaces.md`](../platform/target-runtime-and-surfaces.md) — 現行実行面・feature emit 表
- [`docs/research/target-runtime-verification.md`](../research/target-runtime-verification.md) — 機能別 WAT プローブ証拠
- `docs/current-state.md`
