# セルフホストコンパイラ中核再構築

Status: active — Phase 0 NEXT  
Owner: overlay / #851 / ADR-053  
Created: 2026-09-01  
Last updated: 2026-09-01

決定: [`docs/adr/ADR-053-selfhost-compiler-core-rewrite.md`](../adr/ADR-053-selfhost-compiler-core-rewrite.md)  
追跡: [`issues/open/851-selfhost-compiler-core-rewrite.md`](../../issues/open/851-selfhost-compiler-core-rewrite.md)

この文書が **wasm32-gc + wasi-p2 overlay ≤10s** の実行正本である。
`#850` の tick 探索と、局所 hotspot 削りはこの正本に従わない。

## 現行床（2026-09-01）

| 条件 | wall | RSS |
|---|---:|---:|
| quiet best（tick 80 binary） | **208s** | ~1.76GB |
| 通常負荷（tick 90 / 同一 binary） | **239s** | ~1.77GB |
| Null collector | 23–26s のあと trap | — |
| wasm32 + wasi-p1（参考。公式目標ではない） | ~10s | — |

必要なのは約 24 倍。各改善は半分しか効かない前提。
マイルストーン: **239 → 180 → 120 → 35 → 20 → 12 → 9 秒**。
35 秒までは速度より、fat MIR / 全体 fixpoint / 長寿命 GC graph を消せたかを評価する。

## 受入

- cacheless flattened `src/compiler/main.ark`
- 現行セルフホスト → `wasm32-gc` + `wasi-p2`
- `sha256(s2) == sha256(s3)`
- 10 回計測で median ≤ 7s、p95 ≤ 10s
- peak RSS ≤ 512MB
- validate + fixture parity + 意味論的互換
- `hello` の 2312B / 現行 sha256 は必須ではない
- `BOOTSTRAP_EMIT_*` は受入達成後にだけ翻す

## 探索規則

- `--time` は機械可読 receipt。ログ読みで A/B しない。
- 同一条件 3 回以上で 10% 以上改善しない変更は原則 revert。
- 局所最適化が 2 回連続で外れた hotspot は、それ以上の局所最適化を禁止し構造変更へ移る。
- IR ownership と function-at-a-time の中核は 1 本の設計責任。並列化してよいのは
  receipt / generated tables / reloc linker / fixture・parity / consumer 移行。
- `#850` tick 191 以降の micro hop を開始しない。

## 最終 budget（p95 10s を守る内部 9s）

| 領域 | hard budget |
|---|---:|
| parse + resolve + typecheck | 1.5s |
| declaration / mono / closure discovery | 1.0s |
| body lower | 2.0s |
| local dataflow | 1.0s |
| Wasm body emit | 1.5s |
| link / relocation / sections | 0.7s |
| GC + allocator | 0.7s |
| その他 | 0.6s |
| **合計** | **9.0s** |

## Phase 0 — 239 秒を分解する

- [ ] `--time` を機械可読 receipt にする（ログ専用にしない）
- [ ] 毎回記録: parse / resolve / typecheck / mono-discovery / lower / propagate /
      reachability / MIR optimize / wasm types / wasm bodies / finalize / GC
- [ ] 総 allocation、phase 境界 live bytes、MirInst / MirLocal / MirFunction 個数、
      最大 32 関数の wall
- [ ] 同一条件 3 回の baseline receipt を `docs/research/receipts/` に残す
- [ ] 208s と 239s の差を、同一 binary ノイズか負荷差か切り分ける

schema: [`docs/data/selfhost-overlay-receipt.schema.json`](../data/selfhost-overlay-receipt.schema.json)  
writer: `python3 scripts/selfhost/write_overlay_receipt.py`

Phase 0 完了前に Phase 2 の製品 MIR 切替を始めない。
Phase 1（generated tables）は独立なので Phase 0 と並行してよい。

## Phase 1 — generated code をデータにする

悲観 gate: **239 → ≤180s**（48s がそのまま消えるとは見ない）。

- [ ] `core_op_binding_*_at` / `core_op_registry_*_at` を compact table + index にする
- [ ] `native_c_core_capability_*` も同様
- [ ] 32-arm chunk を最終解として残さない
- [ ] 手編集の `*_generated.ark` 禁止。generator を正本にする
- [ ] `_at` ごとの Vec 再構築は禁止

## Phase 2 — fat MIR を廃止する

悲観 gate: **RSS ≤ 900MB、wall ≤ 120s、hot path の MirInst reconstruction = 0**。
60 秒到達は期待しない。

- [ ] `InstId` / `BlockId` / `LocalId` / `FuncId` / `TypeId` だけを hot 参照にする
- [ ] `op` / `dest` / `arg0` / `arg1` / `type_id` / `extra_id` の scalar storage
- [ ] `MirBlock_inst_at()` の fat record 再構築を削除する（互換層なし）
- [ ] hot MIR から `type_name: String` を外し、判定は TypeId
- [ ] この段階では whole-module MIR を残してよい

禁止: SoA 保存 + fat API、tick 173/178 級の再構築、`MirFunction` への場当たり field 追加。

## Phase 3 — whole-module 本体 MIR を廃止する

悲観 gate: **同時生存 body MIR ≤ 2 関数、RSS ≤ 384MB、wall ≤ 35s**。
35 秒超過は 10 秒計画の赤信号。GC をいじる前にアルゴリズムを再調査する。

- [ ] discovery: FunctionKey / TypeKey / signature / closure / mono worklist。本体 MIR なし
- [ ] body: 1 関数 lower → 局所解析 → Wasm body → reloc 保存 → arena reset
- [ ] CALL / GC type は FuncId / TypeId reloc。最終izer が数値 index にする
- [ ] 「index 未定だから全 MIR 生存」を消す

## Phase 4 — post-pass type propagation を消す

gate: **lower + 全 dataflow + reachability ≤ 3s、whole-module instruction scan = 0**。
全体 15–20 秒を狙う。

- [ ] typed-by-construction。lower で決まる TypeId はそこで確定
- [ ] LocalId 更新 → `users(LocalId)` worklist だけ再評価
- [ ] enum / open variant は型 lattice の join。String 名推測をやめる
- [ ] reachability を FunctionKey queue にする
- [ ] `full typed sync` / module-wide propagate / 反復 body scan を削除

## Phase 5 — phase arena / GC を仕上げる（旧 #850）

gate: **peak RSS ≤ 256–384MB、GC wall ≤ 0.7s、total ≤ 12s**。
12 秒未達ならここで初めて collector 改良を検討する。

- [ ] function lower / analysis / emit scratch を関数ごとに reset
- [ ] durable: 最終 Wasm、TypeId 表、FuncId 表、reloc 表だけ
- [ ] `#850` の製品 arena をこの phase で実装する
- [ ] `#850` の tick 履歴は閉じた扉の証拠として残す

## Phase 6 — 最後の 2–5 秒だけ通常の最適化

12 秒を切ってから解禁。候補: AOT、bounds check、hot hash、string intern、
byte buffer、LEB、generated lookup、branch layout。

- [ ] 12s → 8s を profile-guided で取る
- [ ] 10 回計測で median ≤ 7s、p95 ≤ 10s、RSS ≤ 512MB
- [ ] `s2 == s3` + validate + fixture parity
- [ ] 受入後にだけ `BOOTSTRAP_EMIT_*` を wasm32-gc へ翻す
- [ ] `#851` を close する

## 真の停止条件

- 必須 tool / artifact が無く、リポジトリ内で再生成できない
- Phase 3 gate が構造的に不可能であることが receipt で示された
- ADR-053 を SUPERSEDE する新しい ACCEPTED ADR が出た

commit、tick 完了、1 回の overlay 成功、hello バイト一致、`#850` close は停止条件ではない。
