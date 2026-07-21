# Selfhost compile latency — root cause (2026-07-17, revised 2026-07-20)

ステータス: 調査メモ（決定記録ではない）  
関連:

- [#730](../../issues/open/730-bootstrap-wasm-4gb-memory-limit.md) — Memory64 / fixpoint green（計測入口を含む）
- [#823](../../issues/open/823-selfhost-compile-latency-quadratic-mir.md) — quadratic MIR P0 + reachability BFS（コード landed）
- [#829](../../issues/open/829-selfhost-latency-phase-reprofile-hotspot.md) — **次テーマ**: phase re-profile と dominant hotspot 除去
- 候補: [#824](../../issues/open/824-early-body-lowering-worklist.md)、[#825](../../issues/open/825-ast-cache-format-repair.md)、[#826](../../issues/open/826-symbol-path-intern-clone-audit.md)、[#827](../../issues/open/827-phase-arena-after-heap-model.md)

## 方針（2026-07-20）

次テーマはセルフホスト速度でよい（開発ループを数十分止める問題は機能追加より優先）。  
ただし順序は次で固定する。

```text
mem64 / fixpoint green
  → KEEP_CLOCK 付き s2 が validate でき --time が実時間を出す
  → 同一 artifact・同一 target で phase receipt
  → 最大フェーズを1つ潰す
```

次マイルストーンの名称は **「#824 early body lowering」ではない**。  
正しくは **Selfhost latency phase re-profile and dominant-hotspot removal**（#829）。  
#824 は計測結果から選ばれる候補の一つに留める。

### Acceptance を分ける

| 段階 | 目標 | 備考 |
|---|---|---|
| Cold full selfhost（stage-3） | まず **5 分未満**、次に **2 分未満** | 11.8 万行の全コンパイル |
| Incremental self-build（通常の編集反復） | 最終的に **5〜10 秒** | module cache 等が別段階 |
| ユーザープログラム | すでに sub-second（hello ≈ 0.03 s） | 速度問題の主戦場ではない |

**5〜10 秒を cold full selfhost の直近 acceptance にしてはいけない。**

## #823 以降の事実（文書のずれ修正）

次は **すでに実装済み**（#823 Progress）。二乗コピー仮説だけで現在の 23.5 分を説明してはならない。

| 項目 | 状態 |
|---|---|
| `mir_function_set_local_at` / `set_param_at` in-place | landed |
| `MirModule_set_function_at` in-place | landed |
| 2 回目の full typed sync 削除（propagate 時に type_name sync） | landed |
| reachability queue BFS + `FunctionId.raw → Mir index` | landed |
| lower / pipeline の `--time` ラベル配線 | landed（stub s2 は 0ms、KEEP_CLOCK s2 は実時間） |
| `ARUKELLT_OVERLAY_KEEP_CLOCK=1` 生成物の validate | **解決**（2026-07-21: wasm32/Memory64 とも validate OK、`--time` smoke PASS） |

したがって研究メモの旧「P0 をやれ」節は履歴である。  
現在の作業は **「P0 後も残るボトルネックを再測定し、最大フェーズを潰す」**。

### #824 の期待値（上限の目安）

Post-MIR prune（#823 A/B、同一 stubbed s2-runtime）:

| 量 | before → after | 削減 |
|---|---:|---:|
| functions | 8748 → 7991 | ≈ **8.7%** |
| blocks | 17496 → 15982 | ≈ **8.7%** |
| instructions | 373771 → 358123 | ≈ **4.2%** |

現行パイプラインは **prune 後** に sync・propagate・wasm emit を行う。  
#824（early body lowering）が直接省けるのは、主に削除される ≈4% 命令を生成するまでの `decl_emit` と、その一時 allocation である。

**`decl_emit` が壁時間の圧倒的多数と判明しない限り、23 分を数分へ落とす主役にはならない。**

## 規模（2026-07-20）

| 対象 | `.ark` | 行数 | バイト | 関数 | `use` | `clone(` |
|---|---:|---:|---:|---:|---:|---:|
| `src/compiler` | 1,923 | 118,043 | 4,593,559 | 10,814 | 8,505 | 7,264 |
| `std` | 83 | 19,007 | 587,886 | 1,545 | 36 | 62 |

（2026-07-17 時点の表は履歴。上表を現行とする。）

## Live profile (2026-07-20, WSL2, 12 cores / 23 GiB)

Host wall via `/usr/bin/time -v`。Host:
`.build/selfhost/arukellt-s2-runtime.cwasm`（AOT）。Overlay warm。

`--time` は stubbed `arukellt-s2-runtime` では全 phase `0ms`。  
KEEP_CLOCK の `arukellt-s2-clock.wasm` では実時間（2026-07-21 smoke: hello total ≈ 115–202ms）。

| Probe | Wall | Peak RSS | Notes |
|---|---:|---:|---|
| `compile docs/examples/hello.ark` | **0.03 s** | 28 MiB | ユーザー規模は sub-second |
| `check src/compiler/main.ark` | **9.14 s** | 341 MiB | frontend; stderr に ~194k `warning[` |
| Stage-3（fixpoint、s2 fingerprint hit） | **~23.5 min** | **~2.4 GiB**（終盤も増加） | etime 23:27; その後 s3 validate 失敗 |

Stage-3 終盤 6 分で RSS ≈ 1.37 → 2.40 GiB（線形増加）。時間の大半は frontend ではなく lower/backend + bump 蓄積側。

| Command | Pipeline | Observed |
|---|---|---|
| `selfhost build-compiler` | pinned → s2（`wasm32`） | 歴史的 **43–78 s** |
| `fixpoint --build`（s2 hit） | s2-runtime → s3（`wasm32-gc`） | **~23.5 min**（本計測） |

### 次に取るべき receipt（同一 artifact・同一 target）

壁時間:

`frontend / lower.decl_emit / reachability / sync / propagate / mir_opt / mir_verify / wasm emit`

可能なら **各境界で RSS** も取る（最終 RSS だけでは「遅いフェーズ」と「メモリを積んだフェーズ」が一致しない）。

結果ごとの次手:

| 最大フェーズ | 次手 |
|---|---|
| `decl_emit` | #824 |
| `propagate` | 8 回固定点走査・stack producer 探索の修正 |
| `wasm emit` | section/function 単位の再構築・clone・名前検索監査 |
| 複数フェーズで RSS だけ増 | #826 clone/intern |
| `mir_opt` / `mir_verify` | それぞれ別 issue |

## 歴史的原因メモ（#823 以前の仮説）

以下は 2026-07-17 調査時点の構造説明。P0/P1 適用後も壁が残ることは 2026-07-20 で確認済み。  
**未修正のまま残っている記述を「現在の主因」と読んではいけない。**

1. **（旧）MIR sync の二乗コピー** — in-place 化で緩和済み。残コストは再計測が必要。
2. **（旧）reachability 全体固定点** — queue BFS 化済み（≈ −10 s / 134 s）。
3. **type propagation の反復走査** — 各関数最大 8 回。まだ候補。
4. **bump allocator** — 未回収。Memory64 は OOM 回避のみ。
5. **全体 MIR 化が prune より先** — #824 候補。ただし削減幅は上表のとおり限定的。
6. **incremental/cache 無効** — cold と編集反復は別マイルストーン。
7. **workflow 倍率** — `fixpoint --build` = s2+s3。並列 agent は flat-src 競合と壁時間悪化。

### 旧 frontend / Wasmtime 固定費（2026-07-17、pinned）

| 条件 | wall | 最大 RSS |
|---|---:|---:|
| Wasmtime cold `--help` | ≈ 1.59 s | ≈ 414 MiB |
| AOT `.cwasm` | ≈ 0.06 s | — |
| flat overlay cold / hit | ≈ 11.85 s / ≈ 1.26 s | ≈ 300 MiB |
| pinned `check` | ≈ 3.92 s | ≈ 320 MiB |

AOT/overlay は秒〜十数秒級。十〜二十分級 stage-3 の主因ではない。

## マイルストーン（#829）

1. mem64 / `selfhost fixpoint --build` を green にする（#730 / #813）— **残**
2. `ARUKELLT_OVERLAY_KEEP_CLOCK=1` で生成した s2 が `wasm-tools validate` に通り、`--time` が実時間を出す — **完了（2026-07-21）**
3. 23.5 分級 workload の **phase receipt** を確定する（壁 + 境界 RSS）
4. 最大フェーズを半減させる（候補は計測後に選ぶ。既定で #824 にしない）
5. stage-3 cold をまず **5 分未満**、その後 **2 分未満**
6. （別段階）module cache 等で通常の編集反復を数秒へ

## 計測ギャップ

1. Overlay stub 経路の `--time` は意図的に 0ms（KEEP_CLOCK 経路で実時間を取る）
2. `MIR_LOWER_TRACE=1` は関数ごと出力で selfhost には不適
3. 並列 `fixpoint --build` は receipt を汚染する（同時コンパイル数を記録すること）
4. phase receipt / #829 開始には、なお `selfhost fixpoint --build` green が必要
