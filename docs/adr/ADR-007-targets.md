# ADR-007: コンパイルターゲット整理

ステータス: **ACCEPTED** — canonical は `wasm32` / `wasm32-gc` / `native-*`（scaffold）

決定日: 2026-03-26  
改訂日: 2026-07-11 — T1–T5 廃止、現行挙動を platform 文書へ分離

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

<a id="capability-surface"></a>

ターゲットが対応しない言語機能・host API を使う場合は **コンパイルエラー**
（または明示の target capability error）。実装未了は「仕様上無い」と混同しない
（例: `env::var` と WASI P1 — platform / current-state）。

現行の host 到達可能性・関数別対応表は
[`docs/platform/target-runtime-and-surfaces.md`](../platform/target-runtime-and-surfaces.md#capability-surface)。

### 8. Support tier 語彙

| Tier | 意味 |
|------|------|
| primary | 出荷品質。CI 全ゲート（ADR-013） |
| supported | 日常利用可。失敗は merge を止めない場合あり |
| scaffold | 試験のみ。広域保証なし |

現行のどの surface が guaranteed/smoke かは platform / current-state。

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
- [ADR-045](ADR-045-llvm-scope-withdrawn.md) — native 未決定
- [`docs/platform/target-runtime-and-surfaces.md`](../platform/target-runtime-and-surfaces.md) — 現行実行面
- `docs/current-state.md`
