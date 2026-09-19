# ADR-075: 言語 v2 の linear-memory tracing runtime

ステータス: **ACCEPTED** — `wasm32` fallback は、Wasm GC 命令を使わない非移動 tracing runtime と明示 root frame で、`wasm32-gc` と同じ値意味論を実装する

決定日: 2026-09-19

関連: [ADR-002](ADR-002-memory-model.md)、[ADR-013](ADR-013-primary-target.md)、[ADR-035](ADR-035-wasm-gc-implementation.md)、[ADR-060](ADR-060-language-v2-owned-adts-box-slice.md)、[ADR-074](ADR-074-language-v2-target-capability-configuration.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#873](../../issues/open/873-linear-memory-tracing-gc-runtime.md)

## 文脈

ADR-002 は Wasm GC を primary としつつ、linear memory への lowering を別言語にしないことを定めている。
ADR-060 は `Option`、`Result`、`Box<T>`、`Slice<T>` の所有権と値意味論を backend 間で共有することを定めている。
したがって `wasm32` は、arena や reference counting の都合で cycle、共有、root の寿命を source semantics から除外してはならない。

従来の [wasm32-linear-memory-lowering.md](../plans/wasm32-linear-memory-lowering.md) は arena + RC hybrid と cycle reject を想定していたが、これは RFC-011 の tracing fallback と一致しない。

## 決定

### 1. allocator は非移動 linear-memory allocator とする

- fallback は linear memory 内に allocation record と payload を持つ。
- payload pointer は collector が object を回収しても移動しない。
- 未使用 record は free list へ戻し、同じ容量以上の要求で再利用できる。
- payload と metadata の衝突は trap とし、黙った truncate や成功扱いにしない。
- 初期 metadata 領域は heap の直前に予約し、allocation record は下から上へ、root frame slot は上から下へ伸ばす。

### 2. root は明示 frame と update API で管理する

- 関数の入口は `root_frame_enter`、出口は `root_frame_leave` を使う。
- managed value を保持する wasm local は `root_push` で slot を登録し、local の値が変わるたびに `root_update` で更新する。
- collector は現在の frame slots だけを root として扱う。frame の終了時に slot 領域を無効化する。
- root slot の値は linear pointer であり、GC reference や host handle を混在させない。
- compiler がまだ root update を挿入できない lowering は `collect` を呼ばない。部分的な統合を理由に live value を回収してはならない。

### 3. collector は tracing mark-sweep とする

- mark phase は root から allocation record を辿り、payload 内の aligned pointer candidate を allocation range と照合する。
- mark が増えなくなるまで payload を再走査するため、相互参照を含む cycle を到達可能性として扱える。
- sweep phase は未 mark record を free list へ移す。
- pointer candidate の conservative scan は bootstrap 表現に限る。false positive による一時的な保持は許すが、false negative による live object の回収は許さない。
- moving compaction、finalizer、弱参照はこの ADR の対象外であり、source-level API として導入しない。

### 4. backend parity と Wasm feature 境界

- `wasm32-gc` は Wasm GC の `struct`、`array`、`ref` を使用できる。
- `wasm32` fallback の runtime と managed-value lowering は `struct.new`、`array.new`、`ref.cast` などの Wasm GC instruction を出力してはならない。
- collector、header、tag、descriptor、root slot の byte layout は compiler-private とする。
- allocation、root、collection が target を理由に move / borrow / `Copy` / bounds / error の observable semantics を変えてはならない。
- linear runtime の public source API は追加しない。compiler-private intrinsic と backend ABI のみがこの runtime を呼ぶ。

## 却下した代替案

- **arena + RC hybrid**: cycle を reject または leak とし、GC target と異なる到達性を作るため採用しない。
- **純粋な reference counting**: cycle collection ができず、共有 graph の semantics を保証できないため採用しない。
- **moving compacting collector**: 現行の linear lowering が stable pointer を前提としているため、pointer update の全面導入なしには採用しない。
- **host import の tracing runtime**: `wasm32` artifact の self-contained contract と fallback の再現性を失うため採用しない。

## 検証契約

- positive fixture は同じ source を `wasm32-gc` と `wasm32` へ compile し、linear artifact が Wasm GC instruction を含まないことを binary inspection で確認する。
- runtime fixture は root update、shared graph、cycle graph、unreachable cycle の observable result を確認する。
- `wasm-tools validate` と Wasmtime runtime gate は両 target で実行する。
- root lowering が未接続の間は、runtime の collect gate を有効化せず、issue receipt に未確認範囲を残す。
