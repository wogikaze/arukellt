# ADR-074: Language v2 target capability configuration

ステータス: **ACCEPTED** — GC availability、memory width、host profile、artifact kind を独立した能力軸として扱う
決定日: 2026-09-19
トラック: language-v2
Issue: [#870](../../issues/done/870-target-capability-configuration.md)
関連: [ADR-007](ADR-007-targets.md)、[ADR-013](ADR-013-primary-target.md)、[ADR-035](ADR-035-wasm-gc-implementation.md)、[ADR-058](ADR-058-language-v2-semantics-redesign.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)

## 文脈

`wasm32-gc`、Memory64、WASI P2、Component 出力は現行の製品経路で強く結び付いている。
しかし、これらは同じ性質の識別子ではない。
GC は値の lowering capability、Memory64 は linear-memory address width、WASI は host
profile、Component は artifact boundary である。
一つの target 名や emitter 内の暗黙条件へ詰め込むと、`wasm32-gc` の core Wasm と
Component の canonical memory を区別できず、`wasm32` fallback の capability error も
source legality と混同する。

## 決定

### 1. 4 軸を分離する

CLI と manifest/config の正規表現は次の軸を持つ。

| 軸 | 正規表現 | 所有者 |
|---|---|---|
| Target identity / GC availability | `wasm32` = `linear-memory`、`wasm32-gc` = `wasm-gc` | `target_profiles` と generated target contract |
| Memory width | `--memory-width auto\|memory32\|memory64` | target capability matrix と `DriverConfig.memory_width` |
| Host profile | `--wasi-version wasi-p1\|wasi-p2\|wasi-p3` | `host_profiles` の target matrix |
| Artifact kind | `--emit wasm\|wat\|component\|wit\|all` | `allowed_emit_kinds` |

Target aliases は CLI 境界で一度だけ canonical target と host profile へ正規化する。
alias 名に memory width や artifact kind を埋め込まない。

### 2. Memory width の解決規則

- `auto` は core Wasm では target profile の `default_memory_width` を使う。
- `wasm32-gc` の core Wasm 既定は `memory64` である。
- `wasm32-gc` は `memory32` も許可する。これは GC availability を linear-memory へ
  変更せず、GC lowering と Memory32 address width を組み合わせる設定である。
- `wasm32` は `memory32` だけを許可する。`memory64` は target-specific capability error
  とする。
- Component artifact は official WASI P2 canonical memory boundary を使うため、解決済み
  width は `memory32` とする。`--emit component` と `--emit all` で明示された
  `--memory-width memory64` は、Component を含む組合せとして CLI で拒否する。
- WAT は現行 scaffold の固定 Memory32 artifact である。`--emit wat` の `auto` は
  `memory32` に解決し、`memory64` は scaffold の capability error とする。
- `DriverConfig` は各 artifact leg に対して解決済み width を保持し、Wasm writer の
  `memory64` flag をその値から一度だけ作る。host profile の文字列から width を推測しない。

### 3. Host profile と artifact capability

`wasi-p1`、`wasi-p2`、`wasi-p3` は target ではない。
`host_profiles.supported_targets` が到達可能性を所有し、target と host の不正な組合せは
source を解析する前に capability error とする。
P3 の実装状態は host profile の living state であり、target 名や GC availability の
意味を変えない。

同様に、`--emit` の許可集合は target profile の artifact matrix が所有する。
Component の target、host、memory boundary を target 名の別名で表現しない。

### 4. Source legality と GC availability

通常の言語構文、所有権、型/Trait、数値、文字列の source legality は GC availability に
依存しない。
`wasm32-gc` は Wasm GC representation を、`wasm32` は linear-memory representation を
選ぶが、共有する parser、resolver、typechecker、semantic result は同じである。

target-specific error は host API、ABI、artifact boundary、Memory64 のように本当に
backend/host 固有の capability に限定する。
これにより、GC の有無を理由に通常コードを source-level feature gate にしない。

### 5. 検証と移行

- manifest の capability matrix、generated compiler contract、CLI、extension view、target
  summary は `docs/data/project-state.toml` から同時生成する。
- positive fixture は同じ通常 source を GC target の `memory32` で受け入れることを示す。
- diagnostic fixture は `wasm32 + memory64` を target-specific capability error として固定する。
- 旧 target alias は ADR-007 の `W0002` warning policy を維持し、未指定の memory width は
  `auto` として既存の default behavior を保つ。

## 帰結

- `wasm32-gc` と Memory64 は同義語ではなく、GC + Memory32 という組合せを表現できる。
- P2 Component の memory32 boundary と core Wasm の Memory64 default が同じ compiler
  pipeline 内で明示的に共存する。
- 新しい capability を追加するときは target 名を増やさず、profile matrix の軸と生成契約を
  更新する。
- Wasm GC と linear-memory fallback の representation は異なるが、source semantics の
  parity を fixture と backend validation で検証できる。

## 再検討条件

- Component canonical ABI が guest Memory64 を正式に共有する場合は、Component boundary
  の memory32 rule と width conversion owner を再決定する。
- `--host` や非 WASI host profile を公開する場合は、host profile の CLI grammar と
  manifest matrix を別 ADR で拡張する。
- source semantics が backend capability に依存する新しい言語機能を導入する場合は、
  source legality と capability error の境界を再審査する。

## 関連

- [#870](../../issues/done/870-target-capability-configuration.md)
- [#852](../../issues/open/852-language-v2-semantics-full-implementation.md)
- [ADR-007: コンパイルターゲット整理](ADR-007-targets.md)
- [ADR-013: `wasm32-gc` をプライマリターゲットとする](ADR-013-primary-target.md)
- [ADR-035: Wasm GC implementation](ADR-035-wasm-gc-implementation.md)
- [RFC-011: 言語 v2 の意味論再設計案](../rfcs/011-language-v2-semantics-redesign.md)
