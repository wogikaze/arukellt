# ADR-053: セルフホストコンパイラ中核の再構築（gc-host overlay）

ステータス: **ACCEPTED** — wasm32-gc + wasi-p2 の 10 秒 overlay は最適化ではなく中核再構築とする

決定日: 2026-09-01

実行正本: [`docs/plans/selfhost-compiler-core-rewrite.md`](../plans/selfhost-compiler-core-rewrite.md)  
追跡: [#851](../../issues/open/851-selfhost-compiler-core-rewrite.md)

---

## 文脈

公式目標は、現行セルフホストコンパイラが cacheless flattened
`src/compiler/main.ark` を `wasm32-gc` + `wasi-p2` でコンパイルし、
`sha256(s2) == sha256(s3)` のまま overlay を 10 秒以内にすることである。
`BOOTSTRAP_EMIT_*` を wasm32-gc へ翻す条件もこれである。

現行床は同一 binary で quiet **208 秒**、通常負荷 **約 239 秒**、
peak RSS **約 1.77GB** である。Null collector でも 23–26 秒で trap する。
GC を理想化しても 10 秒には届かない。必要なのは約 24 倍であり、
`#850` の局所 A/B（約 190 tick）では到達できない。

残っている構造的負債は次の三つである。

1. whole-module の fat `MirInst` グラフ
2. module 全体の type propagate / sync / 後処理 scan
3. Copying GC が走査する長寿命 object graph

ADR-002（ユーザープログラムの Wasm GC）は変更しない。
ADR-029（`s2 == s3`）は維持する。
ADR-040（TypeId / SignatureRegistry spine）は型の正本として使う。
ADR-024（関数内の明示的 CFG）は関数単位 MIR でも維持する。
whole-module MIR をセッション寿命で生かし続けることは要求しない。

---

## 決定

### 1. これは中核再構築である

`wasm32-gc` + `wasi-p2` overlay の 10 秒到達を、既存パイプラインの
micro optimization プロジェクトとして扱ってはならない。
探索順は「239 秒の hotspot を 1 個削る」ではなく、
**whole-module fat MIR を生存させる理由を 1 個ずつ消す**こととする。

各改善は期待値の半分しか効かないこと、途中で一度は大規模設計を捨てることを
前提にする。工数は 10–16 engineer-week 級と見積もる。
IR ownership と function-at-a-time の中核は 1 本の設計責任下に置く。

### 2. 受入条件を固定する

公式受入は次とする。

- 対象: cacheless flattened `src/compiler/main.ark`
- ホスト: 現行セルフホストコンパイラ → `wasm32-gc` + `wasi-p2`
- `sha256(s2) == sha256(s3)`
- 10 回計測で median ≤ 7 秒、p95 ≤ 10 秒
- peak RSS ≤ 512MB
- 出力は `wasm-tools validate` に通り、fixture parity と意味論的互換を保つ

内部目標は 7 秒である。10 秒を唯一の線にすると 9.9 秒の実装が残る。

`hello = 2312B` および現行 sha256 のバイト一致は受入条件から外す。
古い emitter と同じバイト列を保つために悪い内部構造を残してはならない。
`s2 == s3`、validation、parity、意味論的互換は必須のままとする。

`BOOTSTRAP_EMIT_*` を wasm32-gc へ翻すのは、上記受入を満たしたあとだけとする。

### 3. generated 表はプログラムではなくデータにする

`core_op_binding_*_at`、`core_op_registry_*_at`、
`native_c_core_capability_*` のような dense `if index == N` 関数を
最終形として残してはならない。compact table + index、必要なら perfect hash にする。
32-arm chunk は移行手段であり最終解ではない。

### 4. fat MIR を廃止する

hot path の正準 MIR は次の id だけを参照する。

`InstId` / `BlockId` / `LocalId` / `FuncId` / `TypeId`（いずれも i32）

命令は compact columns / slab に置く。
`MirBlock_inst_at()` が新しい fat `MirInst` record を返す API は禁止する。
hot MIR の型情報に `type_name: String` を使ってはならない。
表示名は debug layer、判定は `TypeId` とする（ADR-040）。

互換層として「保存は SoA、API は fat record」を残してはならない。
過去に RSS は約 1.77GB → 約 1.10GB まで落ちたが、再構築のせいで
320–480 秒でも overlay が完走しなかった。

### 5. whole-module 本体 MIR を廃止する

コンパイラは二段階とする。

1. 宣言・型・依存関係の discovery。関数本体の永続 MIR は作らない。
2. 1 関数 lower → その関数だけの型 / dataflow → Wasm body → relocatable body 保存 → MIR arena reset。

未確定の関数 index / GC type index は `FuncId` / `TypeId` reloc とし、
最終izer が数値 index へ変換する。
「index が未定だから全 MIR を生かす」は禁止する。

### 6. module 全体の type propagation scan を設計から消す

lower 時点で型が決まる命令はそこで `TypeId` を確定する。
本当に伝播が必要な更新だけを worklist にする。
`full typed sync`、`module-wide propagation scan`、
`normalize-by-repeated-body-scan` を最終形として残してはならない。
reachability は FunctionKey 依存グラフの queue traversal に寄せる。

### 7. phase arena は function-at-a-time のあとで仕上げる

phase arena の所有規則（phase 境界でのみ reset、cross-phase は durable handle、
最終 Wasm と durable 表は reset しない）は
[`docs/research/selfhost-phase-arena-ownership.md`](../research/selfhost-phase-arena-ownership.md)
のまま正しい。ただし `#850` のように arena を先に攻めて
fat MIR / 全体 fixpoint を残してはならない。

GC collector を速くするより、GC に 1.7GB のグラフを渡さない。
collector 改良は total が 12 秒付近まで落ちてからでよい。

### 8. 計測と打ち切り

`--time` はログではなく機械可読 receipt とする。
同一条件 3 回以上の A/B で 10% 以上改善しない変更は原則 revert する。
局所最適化が 2 回連続で外れた hotspot は、それ以上の局所最適化を禁止し
構造変更へ移る。

micro optimization（AOT、bounds check、hash、LEB、branch layout 等）は
全体が約 12 秒になってから解禁する。

---

## 帰結

- `#850` の tick 探索は 10 秒経路の正本ではない。phase arena 製品コードは
  本 ADR の Phase 5 として扱う。
- wasm32 + wasi-p1 の約 10 秒 overlay は公式目標を満たしたことにならない。
- マイルストーン（239 → 180 → 120 → 35 → 20 → 12 → 9 秒）と phase gate は
  計画正本に置く。35 秒までは速度より、fat MIR・全体 fixpoint・長寿命 GC
  graph の削除を評価する。

---

## 代替案

1. **`#850` の局所最適化を続ける。** 却下。208/239 秒の同一 binary ノイズと
   約 190 tick でも 24 倍に届かない。
2. **公式目標を wasm32 の 10 秒のままにする。** 却下。信頼ベースは
   wasm32-gc + wasi-p2 のままである。
3. **hello バイト一致を残す。** 却下。古い emit 形を保存するために
   内部構造を固定するのは本決定と逆である。
4. **arena を先に入れる。** 却下。再構築 API と whole-module 生存を残すと
   RSS だけ落ちて wall は残る。

---

## 再検討条件

- function-at-a-time の Phase 3 gate（同時生存 body MIR ≤ 2 関数、
  RSS ≤ 384MB、wall ≤ 35 秒）を構造的に満たせないことが領収書で示されたとき。
- ADR-029 の `s2 == s3` を捨てる提案が出たとき（本 ADR だけでは捨てられない）。

---

## 関連

- [ADR-002](ADR-002-memory-model.md) — ユーザープログラムの Wasm GC（不変）
- [ADR-024](ADR-024-selfhost-mir-explicit-cfg-before-ssa.md) — 関数内 CFG
- [ADR-029](ADR-029-selfhost-native-verification-contract.md) — `s2 == s3`
- [ADR-040](ADR-040-typed-mir-signature-registry.md) — TypeId spine
- [#850](../../issues/open/850-compiler-phase-arena.md) — phase arena（Phase 5）
- [#823](../../issues/done/823-selfhost-compile-latency-quadratic-mir.md)
- [#829](../../issues/done/829-selfhost-latency-phase-reprofile-hotspot.md)
