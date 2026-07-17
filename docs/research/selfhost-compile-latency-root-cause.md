# Selfhost compile latency — root cause (2026-07-17)

ステータス: 調査メモ（決定記録ではない）  
関連: [#730](../../issues/open/730-bootstrap-wasm-4gb-memory-limit.md)、実装追跡 issue は open index を参照

遅さの主因は Wasmtime の起動や構文解析ではない。次の三つが相互に増幅している。

1. 約1,900モジュール・約10,500関数をほぼ全体処理してから到達不能関数を削る。
2. MIR同期と到達可能性解析に二乗級の配列再構築・線形探索がある。
3. コンパイラ実行時の線形メモリが解放不能な bump allocator で、上記の一時配列・deep cloneがすべて累積する。

このため、計算時間だけでなく必要ヒープも数GiB〜約10GiBへ膨張する。Memory64はクラッシュを回避するが、アルゴリズムと割り当て量を改善しないため、速度問題の解決にはならない。

## 規模

| 対象 | `.ark` | 行数 | バイト | 関数 | `use` | `clone(` |
|---|---:|---:|---:|---:|---:|---:|
| `src/compiler` | 1,914 | 114,137 | 4,453,393 | 10,645 | 8,345 | 6,803 |
| bootstrap flat overlay | 1,889 | 112,611 | 4,996,939 | 10,472 | 8,229 | 6,791 |
| `std` | 83 | 18,981 | 587,283 | 1,541 | 35 | 62 |

pinned compilerは2,224,789 bytes。Wasmのメモリ節はmemory32、初期8,192 pages、すなわち512 MiB。

## 実測

### Wasmtime固定費

| 条件 | wall | 最大RSS |
|---|---:|---:|
| cold JIT、`--help` | 約1.59 s | 約414 MiB |
| Wasmtime cache hit | 約0.07 s | 約151 MiB |
| AOT `.cwasm` 実行 | 約0.06 s | 同程度 |
| AOT生成 | 約1.69 s | 約253 MiB |

AOT化で短い反復は約25倍速くなる。ただし10分級のセルフコンパイルに占める割合は1%未満。

### flat overlay

| 条件 | wall | 最大RSS |
|---|---:|---:|
| cold再生成 | 約11.85 s | 約306 MiB |
| disk cache hit | 約1.26 s | 約301 MiB |

cold時には無視できないが、10分級の主因ではない。

### frontendとcompile

pinned compiler + flat overlay + AOTで測定。

| 処理 | wall | 最大RSS | 備考 |
|---|---:|---:|---|
| entry単体 `parse` | 約0.05 s | 約131 MiB | import graph全体の測定ではない |
| resolveで故意に停止 | 約1.34 s | 約224 MiB | module load + parse + resolveの近似 |
| type errorで故意に停止 | 約3.54 s | 約319 MiB | typecheckまでの近似 |
| full `check` | 3.92 s | 328,236 KiB | 3,772 warnings、326 KiB stderr |
| raw pinned `compile` | 10.30 sでOOB | 644,552 KiB | 出力前にlinear-memory OOB |

`check`の警告内訳はW0007=3,340、W0006=286、W0005=108、W0003=20、W0001=16、W0009=2。警告整形は余計な負荷だが、秒未満級であり主因ではない。

最新のリポジトリ内記録 `issues/open/730-bootstrap-wasm-4gb-memory-limit.md` では、current s2によるstage-3について、typecheckまで約54秒、修正前lowerは10分以上timeout・RSS約0.7→10GB、prune-before-sync修正後もfull compileは約10〜11分と記録されている。これはpinned frontend測定と同一条件ではないが、遅延がcurrent compilerのlower/backendとメモリ挙動に集中していることを示す。

## 原因1: MIR同期の二乗コピー

`src/compiler/mir/typed_mir_sync_module.ark`:

- 全関数を巡回し、各関数更新後に `MirModule_set_function_at`。
- 全param・localについてsetterを呼ぶ。
- param/local 1個の更新ごとにvector全体を新規作成・コピー。

ローカル数をLとすると、1関数の同期は概ねΘ(L²)個の要素移動と新規vector生成になる。

さらに `src/compiler/mir/module_functions.ark` では、関数1個を更新するごとに全関数vectorを再構築する。関数数をFとすると、module-levelにもΘ(F²)の要素移動が発生する。

`src/compiler/mir/lower/ctx_api_module.ark` では、

1. 全value type同期
2. type propagation
3. 全value type再同期

を行う。propagation自身も各関数更新時に全関数vectorを再構築するため、module vectorの全再構築は少なくとも3系列ある。

`src/compiler/mir/lower/entry.ark` には「sync/propagate is O(locals²) per function」と明記され、#730修正では同期前にpruneするよう変更された。これはハングを解消したが、到達不能関数を含むCoreHIRからMIR関数を生成する前半コストは残る。

## 原因2: 到達可能性解析が全体固定点 × 線形名前検索

（旧）`reachability_entry` は変化がなくなるまで全体walkを反復し、`reachability_names` は call targetごとに全関数を文字列比較していた。

（2026-07-17 / #823 P1）queue BFS + 明示 `FunctionId.raw → Mir index`（`reachability_index.ark`）へ置換。CALL/REF_FUNC は `func_id_raw` 優先、未解決時のみ名前 / normal-call fallback。`--time` で `lower.reachability_fns: before=N after=M` を出力。

計測（clock stub 付き `arukellt-s2-runtime.wasm`、full selfhost、`--time`）: wall ~102 s、peak RSS ~1.32 GiB、`before=8737 after=7980`。phase ms は overlay の `clock::monotonic_now` stub により 0ms。KEEP_CLOCK 経路は wasm-validate 失敗のため未採用。残存ボトルネックは **decl_emit（未到達 body の先行 lower）** → #824。

## 原因3: type propagationの反復走査

`post_pass_type_propagate` は各関数について最大8回、全block・全instructionを固定点走査する。この処理の前後に二乗同期があり、「更新→vector再構築→再同期」の組み合わせが重い。

## 原因4: 解放しないbump allocatorがコピーを累積

#730 では、global 0をheap pointerとして単調増加し、解放しないことが根本原因として明記されている。二乗アルゴリズムとbump allocatorは独立原因ではなく、同じ一時データを「大量に作る」「一度も回収しない」という増幅関係にある。

## 原因5: 全体処理が早すぎ、pruneが遅すぎる

function index構築・全decl emitの後で初めてpruneする。prune-before-syncにより最悪の同期は避けたが、全関数のMIR生成自体は先に行われる。stage-2 overlay は prune を意図的に無効化する経路があり、`build-compiler` は全グラフ処理を避けにくい。

## 原因6: incremental/cacheが実質無効

AST cache は `AST cache disabled - needs heap investigation` として常にparseしている。`--cache-dir` は効かない。fixpoint の content-hash cache は「sourceが全く変わっていない場合にビルドを丸ごとスキップ」するだけで、module単位incrementalではない。

## 原因7: workflow上の倍率

`build-compiler` は pinned→s2 の全compiler build。`fixpoint --build` はさらに s2→s3。並列 agent の runtime lock は破損防止に必要だが待ち時間が加算される。

## 文書と実態のずれ

`docs/compiler/bootstrap.md` は warm overlay の `build-compiler` を約45〜50秒とする一方、#730 は stage-3 full compile を約10〜11分とする。pinned→s2 と current s2→s3 など条件が混在している。コマンド・artifact・target・AOT/cache・peak heap を同じ receipt に記録しない限り、性能回帰を正しく追えない。

## 優先順位

### P0: 直接原因を除去

1. `mir_function_set_local_at` / `mir_function_set_param_at` を in-place update にする。
2. `MirModule_set_function_at` を in-place update にする。
3. 2回のtyped syncを融合・差分化する。
4. phase内計測を追加する（`lower-decl-emit` / `reachability` / `sync1` / `propagate` / `sync2` / `verify` / `wasm-emit`）。

### P1: 全体処理を避ける

1. CoreHIR/typed graphで到達可能集合を作り、未到達関数をMIR化しない → **#824**（設計）。
2. reachabilityをFnId/index + queue-based traversalへ → **#823 に landed**。
3. AST cache format repair → **#825**（再有効化ではない; 計測後に優先度）。
4. 長期: module単位キャッシュと依存invalidate。

### P2: 割り当て量を下げる

1. identifier/path/literal の intern → **#826**。
2. deep `clone` 監査 → **#826**。
3. phase arena → **#827**（ADR-002 / #730 の lifetime・所有権が決まるまで試作禁止）。
4. self-build時のwarning抑制（効果は小さい）。

### P3: 固定費と運用

1. `.cwasm` 常時利用。
2. overlay の mtime manifest 判定。
3. routine iteration は stage-2 を1回共有。

## 期待できる効果

- AOT / overlay cache / warning抑制: 秒〜十数秒級。10分buildの主因ではない。
- in-place MIR update + sync差分化: 二乗コピーと未回収vectorを同時に除去するため最大の改善候補。
- early reachability / phase arena: CPU・MIRメモリ・emit全体と4GiB/10GiB問題の根治側。
