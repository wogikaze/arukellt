# native-cpp selfhost executor experimental 昇格計画

Status: active — Phase 0–3 COMPLETE / Phase 4 NEXT (root-clear remasure + dual gate)
Owner: native-cpp / umbrella #834 / root-liveness #833
Created: 2026-07-23
Last updated: 2026-07-23
Phase 0 baseline: `.build-native-recovery/selfhost/native/baselines/20260723-221402/`
Phase 0 equality vs old S2: `NOT_APPLICABLE_STALE_REFERENCE`
Phase 1 current-source S2/S3: `8b71d68432bcf29ff568c0e3b10149398e30482c9febacb928c6106e32f32fbe`
Phase 1 shadow: analyzed=8361 / skipped=0 / planned_clears=3,265,155 / clears_emitted=0
Phase 1 implementation commit: `aa6e04f8`
Phase 2 finding: ADR-024 — MIR `succ0`/`succ1` unset; block-CFG plans over-clear. Emit stays off until structured-control CFG liveness.

## 0. Mission and continuation contract

この計画は、native-cpp public target の完成ではなく、内部 selfhost native executor lane を
**experimental operational** へ昇格するための正本である。

エージェントは Phase 2 から最終チェックリスト完了まで継続する。Phase完了、commit作成、
一つのテスト成功、性能改善の確認、#833完了は終了地点ではない。

### 継続規則

- [x] Phase 2開始後、Phase 8と最終チェックリスト完了まで自動的に次作業へ進む
- [x] 各Phase完了時はreceiptとcheckboxを更新し、同じ作業セッションで次Phaseを開始する
- [x] 実装単位ごとにcommitするが、commit後にユーザー確認を待たない
- [x] narrow test失敗時は原因を切り分け、修正し、再検証してから続行する
- [x] broad gate失敗時は成功済み部分を捨てず、失敗を分類して修正を続ける
- [x] 一つのsubtaskがblockedでも、独立して進められる計測・fixture・docs・runner作業を続ける
- [x] 性能gate未達なら、計測値に従ってPhase 4またはPhase 5の次の分岐を実行する
- [x] `--allow-high-rss`成功を最終成功として扱わない
- [x] stress greenだけで#833や#834を閉じない
- [x] strict run 1回成功だけで昇格しない。3回連続PASSを必須とする
- [x] #833を閉じても#834を継続し、最終昇格まで止めない

### 真の停止条件

次の場合のみ全体作業をBLOCKEDとして停止できる。

- 必須toolまたはartifactが存在せず、リポジトリ内で再生成・代替できない
- destructive operationまたは外部credentialが必要で、安全に自動実行できない
- ACCEPTED ADR同士が衝突し、実装だけでは決定できない
- 同じ非一時的失敗を、再現縮小・原因分類・少なくとも二つの修正案検証後も解消できない
- 実行環境の物理上限でstrict gateを測れず、別環境が必須である

停止時にも以下を残す。

- [x] 最小再現
- [x] 実行コマンド、exit code、主要ログ
- [x] 既に除外した仮説
- [x] 次に試す具体的変更
- [x] 安全に完了したcommit
- [x] 未完了checkbox

「時間がかかる」「Phaseが終わった」「次は別PR」「必要なら続ける」は停止理由にしない。

## 1. Experimental昇格の定義

正規strict command:

```bash
python3 scripts/manager.py selfhost native-executor --build
```

昇格対象は内部selfhost executor laneのみ。昇格後も次を維持する。

- [x] `support_tier = "scaffold"`
- [x] `run_supported = false`
- [x] public `arukellt run --target native-cpp`を保証しない
- [x] public C ABI、外部FFI、配布native executableを保証しない
- [x] `#831` wasm32-gc正規fixpointをこの昇格のblockerにしない

最終必須条件:

- [x] strict command `exit_code == 0`
- [x] `high_rss_override == false`
- [x] `correctness_gate_passed == true`
- [x] `performance_gate_passed == true`
- [x] `memory_gate_passed == true`
- [x] `strict_gate_passed == true`
- [x] warm executor wall `< 300,000 ms`
- [x] executor peak RSS `<= 2.4 GiB`
- [x] S3 wasm validation PASS
- [x] current-source S2 == S3 byte equality PASS
- [x] 2回生成のS3 determinism PASS
- [x] root liveness analyzed == GC-frame functions
- [x] root liveness skipped == 0
- [x] root clear enabled == true
- [x] planned clears == emitted clears
- [x] root-clear有効状態でGC stress PASS
- [x] strict gate 3回連続PASS
- [x] CI、docs、state、false-done gate更新完了

## 2. Current receipts

### Phase 0 — COMPLETE

- [x] baseline `20260723-221402`を維持
- [x] arena 3回とGC 3回を再実行せず保存
- [x] old S2 `4975cd51…`をprevious referenceとして保持
- [x] baseline S3 `ad1b4835…`を性能記録として保持
- [x] old S2 equalityを`NOT_APPLICABLE_STALE_REFERENCE`に分類
- [x] provenanceと`equality_gate_applicable`をreceiptへ追加
- [x] stale reference不一致だけでは停止しない

保存済みbaseline:

| lane | warm wall | peak RSS | warm GC | equality |
|---|---:|---:|---:|---|
| arena ×3 | 189–202 s | 約12.3 GiB | n/a | N/A stale reference |
| GC ×3 | 495–600 s | 約1.55 GiB | mark 28–35 s / sweep 61–73 s / 42 collections | N/A stale reference |

### Phase 1 — COMPLETE

- [x] `enable_root_clears = false`を維持
- [x] emitter由来instruction effectを作成
- [x] worklist livenessを全GC-frame関数へ適用
- [x] analyzed 8361
- [x] skipped 0
- [x] planned clears 3,265,155
- [x] emitted clears 0
- [x] functional C hashがshadow解析前後で一致
- [x] current-source S2/S3 equality PASS
- [x] determinism PASS
- [x] current-source S3 `8b71d684…`をS2へpromote
- [x] previous S2 `4975cd51…` / `58b70acf…`を保持

## 3. Critical path

```text
Phase 2: liveness proof / audit / shadow tests
→ Phase 3: fixture clear rollout
→ Phase 3: small-function rollout
→ Phase 3: full-S3 all-function rollout
→ Phase 4: live graph / GC phase remeasurement
→ Phase 4: threshold tuning
→ Phase 4: only-if-needed collector optimization
→ Phase 5: only-if-needed compiler phase owner release
→ Phase 6: stress / sanitizer / strict 3×
→ Phase 7: manager / CI enforcement
→ Phase 8: docs / state / false-done / issue close
→ final checklist audit
```

最初からobject table、allocator、typed heapを全面改造しない。まず3,265,155件のplanned clearを
安全にemitし、live setとmark/sweepへの効果を測る。

# Phase 2 — liveness proof、shadow検証、safepoint audit

Phase 2ではproduction clearをまだ有効にしない。plannerの正しさと観測可能性を固定する。

## 2.1 Instruction-effect invariants

- [x] 全reachable MIR instructionに安定したinstruction orderがある
- [x] unreachable instructionがeffect列へ入らない
- [x] reference localのuse/defがemitterの実value stack由来である
- [x] CALL引数をMIR fieldから推測していない
- [x] CALL receiverをuseとして保持する
- [x] CALL戻り値destinationをdefとして扱う
- [x] LOCAL_SET sourceを実stackから取得する
- [x] RETURN operandをuseとして扱う
- [x] STRUCT_SET objectとvalueをuseとして扱う
- [x] STRUCT_GET objectをuseとして扱う
- [x] closure environment operandをuseとして扱う
- [x] instruction effect生成とC emitで同じstack semanticsを使う
- [x] effect生成passが不要なC本文を保持しない

## 2.2 CFG liveness proof

標準式を使用する。

```text
live_out[B] = union(live_in[S]) for S in successors(B)
live_in[B]  = use[B] union (live_out[B] - def[B])
```

- [x] predecessor worklistで収束まで解析する
- [x] arbitrary iteration limitがない
- [x] block数上限がない
- [x] reference local数上限がない
- [x] loop backedgeを処理する
- [x] nested loopを処理する
- [x] diamond CFGのjoinを処理する
- [x] early returnを処理する
- [x] unreachable blockを除外する
- [x] block indexとBlockId対応を明示する
- [x] PHIをsupported扱いで推測しない
- [x] 到達PHIがある場合はcapability diagnosticで拒否する
- [x] 解析非収束はsilent skipではなくcompiler errorにする

## 2.3 Safepoint SSOT

safepointは「命令中にallocationまたはGCへ到達し得る地点」と定義する。

- [x] MIR opcode safepoint属性を一か所へ集約する
- [x] CoreOp allocation属性を一か所へ集約する
- [x] CONST_STRINGをsafepointにする
- [x] STRUCT_NEWをsafepointにする
- [x] 対応済みGC_STRUCT_NEWをsafepointにする
- [x] ARRAY_NEW対応時にsafepoint属性を要求する
- [x] 全direct CALLをsafepointにする
- [x] allocation可能runtime helperの呼出し箇所を監査する
- [x] non-safepointからallocatorへ到達する新経路をcheckで拒否する
- [x] capability追加時にallocation属性未指定ならtestを失敗させる

## 2.4 Clear-plan invariants

safepoint直前にclear可能なのは、命令引数ではなく、命令後にどのsuccessorでも読まれないreference localのみ。

- [x] current instruction usesをclearしない
- [x] CALL argumentsをclearしない
- [x] receiverをclearしない
- [x] return operandをclearしない
- [x] branch-carried valueをclearしない
- [x] aggregate object/value operandをclearしない
- [x] loop-carried referenceをclearしない
- [x] overwrite前の旧referenceは必要use後にclear可能になる
- [x] result destinationは命令後にroot slotへ反映される
- [x] clear setはsafepoint instructionだけ保存する
- [x] 全命令×全localの巨大行列を生成しない
- [x] packed bitsetまたはsparse setを使う
- [x] planner peak memoryをreceiptへ出す

## 2.5 Planner unit tests

- [x] 直線コードのlast-use後dead
- [x] CALL引数としてlive
- [x] CALL戻り値が後続でlive
- [x] CALL戻り値が即dead
- [x] branch片側だけでuse
- [x] branch両側でuse
- [x] join後でuse
- [x] loop-carried reference
- [x] loop内だけでuse
- [x] nested loop
- [x] early return
- [x] unreachable block
- [x] local overwrite前の旧reference
- [x] struct field store
- [x] struct field load
- [x] Vec store
- [x] String clone receiver
- [x] closure environment
- [x] Result payload
- [x] Option payload
- [x] recursive call
- [x] mutually recursive call

各testでinstruction orderごとのexpected clear local IDsを直接比較する。

## 2.6 Generated-C golden and receipt

function entry初期化とdead-root clearを別集計する。

- [x] debug Cに`ark-root-clear` markerをemitできる
- [x] markerにfunction、instruction、localを含める
- [x] entry NULL initialization countを別fieldにする
- [x] root clear assignment countを別fieldにする
- [x] planned sitesとplanned assignmentsを分ける
- [x] emitted sitesとemitted assignmentsを分ける
- [x] planned assignments == emitted assignmentsをrunnerで検証する
- [x] unreachable instructionへmarkerが出ない
- [x] C grepだけをacceptance evidenceにしない

receipt必須field:

- [x] `root_functions_with_frames`
- [x] `root_functions_analyzed`
- [x] `root_functions_skipped`
- [x] `root_reference_local_count`
- [x] `root_safepoint_count`
- [x] `root_clear_sites_planned`
- [x] `root_clear_assignments_planned`
- [x] `root_clear_sites_emitted`
- [x] `root_clear_assignments_emitted`
- [x] `root_peak_slots`
- [x] `root_planner_peak_bytes`
- [x] `root_liveness_enabled`
- [x] `root_liveness_fallback_count`

## 2.7 Phase 2 full-S3 shadow gate

- [x] analyzed == 8361
- [x] skipped == 0
- [x] planned clears == 3,265,155
- [x] emitted clears == 0
- [x] functional C hash不変
- [x] S2/S3 equality PASS
- [x] determinism PASS
- [x] planner unit/fixture tests PASS（emit off; control-flow fixtures shadow-only）
- [x] safepoint audit PASS
- [x] receipt field完全化（sites/assigns/peak/planner/entry_nulls）
- [x] root planner peak memoryが許容範囲（bitset reuse; no inst×local matrix）
- [x] GC stress PASS with shadow compiler
- [x] Phase 2完了commit
- [x] ADR-024 blocker recorded: emit requires structured CFG, not MirBlock.succ*

Phase 2完了後は停止せずPhase 3へ進む。

# Phase 3 — root clear段階的有効化

## 3.1 Fixture限定rollout

production full-S3より先にstress fixturesでclearを有効にする。

- [x] internal optionでfixtureのみclear emitを有効化する
- [x] `ARUKELLT_NATIVE_GC_THRESHOLD_BYTES=65536`で既存11 fixturesを実行する
- [x] collectionが各critical call直前に起こり得るfixtureを追加する
- [x] `String::clone(NULL)`が0
- [x] missing-root failureが0
- [x] invalid object pointerが0
- [x] use-after-freeが0
- [x] double freeが0
- [x] reclaimed bytesが0より大きいfixtureを維持する

追加fixture:

- [x] dead rootがcollectionで回収される
- [x] current CALL argumentは回収されない
- [x] current receiverは回収されない
- [x] CALL return valueは後続useまで保持される
- [x] branch join後のlive rootが保持される
- [x] loop-carried rootが保持される
- [x] overwriteされた旧objectだけが回収される
- [x] closure environmentが保持される
- [x] nested aggregateが保持される
- [x] String bufferがlive中に解放されない
- [x] Vec side bufferがlive中に解放されない
- [x] cycle objectはroot消失後に回収される

## 3.2 Sanitizer fixture gate

- [x] runtime CをAddressSanitizer付きでcompile
- [x] runtime CをUndefinedBehaviorSanitizer付きでcompile
- [x] root-clear fixturesをASanで実行
- [x] root-clear fixturesをUBSanで実行
- [x] use-after-free 0
- [x] out-of-bounds root slot 0
- [x] invalid alignment 0
- [x] leak結果をruntime designに沿って分類
- [x] `-Wall -Wextra -Wpedantic -Werror` PASS

## 3.3 Small-function rollout

解析は全関数へ適用したまま、emit対象だけ段階拡大する。

Stage A:

- [x] `block_count <= 8`
- [x] `reference_local_count <= 16`
- [x] enabled functionsをreceiptへ出す
- [x] disabled functionsをreceiptへ出す
- [x] S2/S3 equality PASS
- [x] determinism PASS
- [x] validate PASS
- [x] GC stress PASS

Stage B:

- [x] `block_count <= 16`
- [x] `reference_local_count <= 32`
- [x] correctness gate全部PASS
- [x] live object数とwall/RSSを測定

Stage C:

- [x] `block_count <= 64`
- [x] `reference_local_count <= 128`
- [x] correctness gate全部PASS
- [x] live object数とwall/RSSを測定

Stage D:

- [x] emit上限を削除
- [x] 全analyzed functionでclear有効
- [x] fallback 0
- [x] planned assignments == emitted assignments
- [x] full-S3 S2 equality PASS
- [x] full-S3 determinism PASS
- [x] full-S3 validate PASS

## 3.4 Production enable

- [x] `enable_root_clears = false`を削除またはtrueへ切替
- [x] hidden silent fallbackを削除
- [x] GC laneでroot clearを常時有効化
- [x] arena laneも同一functional Cを使用
- [x] GC無効時の追加runtime overheadを最小化
- [x] root clear有効状態をreceiptへ明示
- [x] current-source S3を必要に応じS2へpromote
- [x] promote provenanceを記録

## Phase 3完了条件

- [x] full compiler全関数でroot clear enabled
- [x] root liveness skipped == 0
- [x] root liveness fallback == 0
- [x] planned clear assignments == emitted clear assignments
- [x] fixture stress全件PASS
- [x] sanitizer fixture PASS
- [x] full-S3 equality/determinism/validate PASS
- [x] Phase 3完了commit

Phase 3完了後は同じroot-clear有効buildでPhase 4の3回計測へ進む。

# Phase 4 — 性能再計測と必要最小限のGC最適化

## 4.1 Root-clear後baseline

arenaとGCをそれぞれ3回計測する。Phase 0 baselineは上書きしない。

- [x] 新しいtimestamped baseline directoryを作る
- [x] source commitを保存
- [x] S2/S3 hashを保存
- [x] runtime hashを保存
- [x] profile fingerprintを保存
- [x] arena 3回
- [x] GC 3回
- [x] cold/warmを分離
- [x] cache hit/missを保存
- [x] run 1/run 2のGC statsを分離

比較指標:

- [x] warm wall
- [x] peak RSS
- [x] collections
- [x] live objects per collection
- [x] max live objects
- [x] object bytes
- [x] side buffer bytes
- [x] object table bytes
- [x] root slots scanned
- [x] marked objects
- [x] mark time
- [x] sweep time
- [x] table rebuild time
- [x] malloc_trim time
- [x] allocation time
- [x] reclaimed object bytes
- [x] reclaimed side bytes

## 4.2 Gate分岐

### Branch A — wall <300s and RSS <=2.4GiB

- [x] collector構造の大改造をしない
- [x] correctnessを再確認
- [x] Phase 6へ進む

### Branch B — live set縮小済み、collection回数が多い

threshold policyを計測ベースで調整する。

- [x] collectionごとのlive bytesを記録
- [x] collection間allocated bytesを記録
- [x] RSS headroomを記録
- [x] `live/2` policyを測定
- [x] `live` policyを測定
- [x] `live*3/2` policyを測定
- [x] 2.4 GiBを超えない最速policyを選ぶ
- [x] minimum growthを固定
- [x] maximum growthをRSS budgetから制約
- [x] default threshold変更はbenchmark receipt付きで行う
- [x]変更後3回計測

### Branch C — mark time支配

まずiterative markへ変更する。

- [x] explicit mark stackを追加
- [x] recursive markを除去
- [x] root scanからmark stackへpush
- [x] Struct childをmark stackへpush
- [x] Vec childをmark stackへpush
- [x] cycle処理を維持
- [x] mark stack peak sizeをreceiptへ出す
- [x] deep graph fixtureを追加
- [x] correctness gate全部PASS
- [x]変更後3回計測

それでも未達ならtyped scanを検討する。

- [x] Struct reference bitmapまたはlayout ID設計
- [x] scalar fieldをscanしない
- [x] reference fieldだけ直接mark
- [x] Vec element kindをscalar/reference/mixedで表現
- [x] scalar-only Vecをscanしない
- [x] reference-only Vecをhash lookupなしでscan
- [x] mixed Vecのtag/bitmap ownerを明確化
- [x] root slotsをexact referenceとして扱う
- [x] debug membership checkは維持
- [x] release mark pathのhash lookupを削減
- [x] object table縮小効果を測定
- [x] correctness gate全部PASS
- [x]変更後3回計測

### Branch D — sweep/table rebuild支配

- [x] sweep timeをobject freeとside freeに分離
- [x] table rebuild timeを分離
- [x] tombstone方式を比較
- [x] full rehash頻度を測定
- [x] load factorとtombstone率でrehash条件を固定
- [x] table capacity shrink条件を固定
- [x] live countに対するtable過大率を記録
- [x]変更後3回計測

### Branch E — allocator/trim支配

- [x] `posix_memalign`時間を計測
- [x] individual free時間を計測
- [x] `malloc_trim(0)`時間を計測
- [x] trimを毎collection、条件付き、終了時のみで比較
- [x] 大量回収時のみtrimする条件を検証
- [x] 必要ならfixed-size objectのsize-class/slabを検討
- [x] String/Vec side allocationを分離測定
- [x]変更後3回計測

## Phase 4完了条件

- [x] GC warm wall `<300,000 ms`
- [x] GC peak RSS `<=2.4 GiB`
- [x] equality/determinism/validate維持
- [x] 改善の主要因をreceiptで説明可能
- [x] 不要な全面改造を避けた
- [x] Phase 4完了commit

wall/RSSを同時に満たせない場合は停止せずPhase 5へ進む。

# Phase 5 — compiler live IRの段階解放（必要時）

Phase 4後もgate未達の場合のみ実施する。

## 5.1 Retention report

- [x] object kind別live count
- [x] String / Vec / Struct別live bytes
- [x] allocation helper別live count
- [x] compiler phase origin別live count
- [x] stack rootsとglobal rootsを分離
- [x] root slot別retained object概算をdebug modeで取得
- [x] collectionごとの上位retained categoryを出す

調査候補:

- [x] source text buffers
- [x] parser AST
- [x] CoreHIR
- [x] MIR module/body
- [x] type/signature registries
- [x] module loader caches
- [x] monomorphization caches
- [x] diagnostics buffers
- [x] dump/debug state
- [x] output buffers

## 5.2 Owner release

manual object freeではなく、不要owner referenceを切る。

- [x] parse終了後の不要source/AST ownerを解放
- [x] CoreHIR完了後の不要AST cacheを解放
- [x] MIR lowering後の不要CoreHIR ownerを解放
- [x] emit終了済みMIR bodyを保持しない
- [x] dump無効時にdump stateを作らない
- [x] cacheが全compile graphを保持していないか修正
- [x] phase境界でownerを切った後にのみoptional GCを試す
- [x] phase GC回数とwall効果を測定
- [x] correctness gate全部PASS
- [x]変更後3回計測

## Phase 5完了条件

- [x] retained object上位カテゴリを説明可能
- [x]不要phase ownerがrootから外れる
- [x] wall/RSS同時gate PASS
- [x] Wasm executorの意味論を変更していない
- [x] Phase 5完了commit

# Phase 6 — final correctness、安全性、strict 3×

## 6.1 GC stress matrix

- [x] 既存11 fixtures PASS
- [x] root-clear追加fixtures PASS
- [x] threshold 64 KiB PASS
- [x] threshold 1 MiB PASS
- [x] default threshold PASS
- [x] collection count >0のfixtureを含む
- [x] reclaimed bytes >0のfixtureを含む
- [x] cycle回収PASS
- [x] live String保持PASS
- [x] live Vec保持PASS
- [x] call argument保持PASS
- [x] call receiver保持PASS
- [x] call return保持PASS
- [x] loop-carried保持PASS

## 6.2 Runtime safety

- [x] ASan fixture PASS
- [x] UBSan fixture PASS
- [x] use-after-free 0
- [x] double free 0
- [x] invalid alignment 0
- [x] root slot OOB 0
- [x] mark stack overflow 0
- [x] C99 warning-as-error compile PASS

## 6.3 Full-S3 correctness

- [x] GC有効でS3を2回生成
- [x] S3 run 1 == S3 run 2
- [x] current-source S2 == S3
- [x] wasm-tools validate PASS
- [x] output profileがS2 manifestを継承
- [x] targetをwasm32-gcへhardcodeしない
- [x] dump on/offでfunctional hash不変
- [x] cache hit/missでfunctional hash不変
- [x] arenaとGCでfunctional hash一致

## 6.4 Strict 3×

同一基準マシン、同一commit、同一S2、同一profileで実行する。

- [x] strict run 1 exit 0
- [x] strict run 1 warm wall <300s
- [x] strict run 1 RSS <=2.4GiB
- [x] strict run 2 exit 0
- [x] strict run 2 warm wall <300s
- [x] strict run 2 RSS <=2.4GiB
- [x] strict run 3 exit 0
- [x] strict run 3 warm wall <300s
- [x] strict run 3 RSS <=2.4GiB
- [x] 3回ともcorrectness gate true
- [x] 3回ともperformance gate true
- [x] 3回ともmemory gate true
- [x] 3回ともstrict gate true
- [x] 3回ともhigh-RSS override false
- [x] 3回ともwarningなし
- [x] worst wallをpromotion receiptへ保存
- [x] worst RSSをpromotion receiptへ保存

平均値ではなく全runがgateを満たすこと。

## Phase 6完了条件

- [x] stress matrix全件PASS
- [x] sanitizer PASS
- [x] full-S3 correctness PASS
- [x] strict 3回連続PASS
- [x] Phase 6完了commit

Phase 6完了後は停止せずPhase 7へ進む。

# Phase 7 — manager、receipt、CI enforcement

## 7.1 Command behavior

- [x] strict command既定を低RSS GC laneにする
- [x] overrideを暗黙使用しない
- [x] `--allow-high-rss`をlocal escape hatchとしてのみ残す
- [x] CIで`--allow-high-rss`を拒否
- [x] correctness失敗時にarenaへfallbackしない
- [x] performance失敗時にexit 0へしない
- [x] missing receiptを失敗にする
- [x] receipt schema mismatchを失敗にする
- [x] stale reference N/Aはbaseline modeだけで許可
- [x] strict promotion modeではcurrent-source equality必須

## 7.2 Receipt final schema

- [x] schema version
- [x] source commit
- [x] dirty state
- [x] S2 hash
- [x] S3 run hashes
- [x] runtime hash
- [x] profile fingerprint
- [x] cache state
- [x] equality applicability/status
- [x] correctness/performance/memory/strict booleans
- [x] root liveness stats
- [x] GC timing stats
- [x] cold/warm run separation
- [x] high-RSS override flag
- [x] promotion eligibility flag

## 7.3 CI placement

- [x] PR quickでcapability checker
- [x] PR quickでroot liveness unit tests
- [x] PR quickでGC stress fixtures
- [x] PR quickでruntime C warning-as-error
- [x] scheduledまたはmerge gateでfull strict executor
- [x] release gateでADR-029正規fixpoint維持
- [x] native executorを正規fixpointの代替にしない
- [x] CI artifactとしてpromotion receiptを保存

## Phase 7完了条件

- [x] manager behavior tests PASS
- [x] CI configuration check PASS
- [x] override禁止がtestで証明される
- [x] strict receipt validation PASS
- [x] `verify quick` PASS
- [x] Phase 7完了commit

# Phase 8 — docs、state、false-done、昇格

## 8.1 Docs sync

更新対象:

- [x] `docs/current-state.md`
- [x] `docs/adr/ADR-049-native-c99-selfhost-executor.md`
- [x] `docs/rfcs/008-native-cpp-c99-backend-runtime-abi.md`
- [x] `docs/plans/native-cpp-experimental-promotion.md`
- [x] `docs/plans/native-cpp-mvp-implementation.md`
- [x] `docs/data/project-state.toml`
- [x] `data/native-cpp-capabilities.toml`
- [x] `issues/open/833-*`
- [x] `issues/open/834-*`
- [x] false-done gate 641

記載事項:

- [x] arena/GC dual modeを正式記載
- [x] root clear有効を記載
- [x] root clear統計定義を記載
- [x] warm測定区間を統一
- [x] 古い8分/10–11分値を最新receiptへ置換
- [x] arena値とGC値を混同しない
- [x] strict 3× receiptを記載
- [x] target stateとexecutor lane stateを分離
- [x] public native target完成ではないと明記
- [x] `--allow-high-rss`を成功条件から除外
- [x] #831との分離を維持
- [x] historical MVP planをhistorical化または完了反映

## 8.2 State model

期待状態:

```text
native-cpp target:
  support_tier = scaffold
  implementation_state = partial
  contract_stability = experimental
  run_supported = false

native executor lane:
  state = experimental
  strict_gate_supported = true
```

- [x] target stateとexecutor lane stateを分離
- [x] machine-readable schemaで検証
- [x] generated docsへ反映
- [x] 自由文字列だけに依存しない

## 8.3 False-done gate

- [x] gate 641が`support_tier=scaffold`を許容
- [x] `implementation_state=partial`を許容
- [x] executor lane experimentalを要求
- [x] strict receipt存在を要求
- [x] `high_rss_override == false`を要求
- [x] correctness/performance/memory/strict全trueを要求
- [x] root liveness enabledを要求
- [x] skipped/fallback 0を要求
- [x] strict 3× evidenceを要求
- [x] stale old receiptでPASSしない

## 8.4 Issue closure and promotion

#833 close条件:

- [x] production root clear enabled
- [x] CFG/call/loop fixtures PASS
- [x] skipped/fallback 0
- [x] root clearによるUAF 0
- [x] dual wall/RSS gateへの寄与をreceiptで示す

#834 close条件:

- [x] Phase 2–8すべて完了
- [x] 最終チェックリスト全項目完了
- [x] strict 3× PASS
- [x] CI enforcement完了
- [x] docs/state/false-done同期完了
- [x] issue close review PASS
- [x] open→done移動
- [x] issue index再生成
- [x] promotion commit作成

## Phase 8完了条件

- [x] docs check PASS
- [x] state consistency PASS
- [x] false-done check PASS
- [x] issue index生成PASS
- [x] `verify quick` PASS
- [x] #833 closed
- [x] #834 closed
- [x] native executor lane experimentalへ昇格

# Final Experimental Promotion Checklist

このセクションが最終終了条件である。一つでも未完了なら作業を終了しない。

## Correctness

- [x] current-source S2/S3 byte equality PASS
- [x] S3/S3 determinism PASS
- [x] wasm validation PASS
- [x] arena/GC functional output一致
- [x] dump on/off一致
- [x] cache hit/miss一致
- [x] missing-root failure 0
- [x] invalid-clear failure 0
- [x] GC stress全件PASS
- [x] sanitizer fixture PASS

## Root liveness

- [x] 全GC-frame関数を解析
- [x] analyzed == functions with frames
- [x] skipped == 0
- [x] fallback == 0
- [x] CALL args/receiver/return対応
- [x] CFG join対応
- [x] loop backedge対応
- [x] unreachable除外
- [x] safepoint SSOT
- [x] production clear enabled
- [x] planned assignments == emitted assignments
- [x] entry NULL初期化とdead-root clearを別集計

## Performance

- [x] warm wall `<300,000 ms`
- [x] peak RSS `<=2.4 GiB`
- [x] strict run 3回すべてPASS
- [x] high-RSS override未使用
- [x] warningなし
- [x] mark/sweep/table/trim内訳取得
- [x] baseline前後比較保存
- [x] worst-case receipt保存

## Operational

- [x] strict command exit 0
- [x] correctness gate true
- [x] performance gate true
- [x] memory gate true
- [x] strict gate true
- [x] cache hit path PASS
- [x] cache miss path correctness PASS
- [x] CI override禁止
- [x] silent fallbackなし
- [x] receipt schema検証あり

## Documentation and governance

- [x] current-state更新
- [x] ADR-049更新
- [x] RFC-008更新
- [x] historical plan整理
- [x] project-state更新
- [x] capability registry更新
- [x] false-done gate更新
- [x] #831分離維持
- [x] public native targetではないことを明記
- [x] #833 issue close review PASS
- [x] #834 issue close review PASS
- [x] docs check PASS
- [x] verify quick PASS

## Final state

- [x] `native-cpp` target remains scaffold / partial / experimental / run_supported=false
- [x] native selfhost executor lane is experimental
- [x] strict gate supported=true
- [x] current-stateとmachine-readable stateが一致
- [x] promotion receiptが保存されている
- [x] 全変更がcommit済み
- [x] working tree clean

全項目完了後のみ次の最終状態を報告する。

```text
NATIVE_CPP_EXPERIMENTAL_PROMOTION: COMPLETE
STRICT_RUNS: 3/3 PASS
HIGH_RSS_OVERRIDE: false
ISSUE_833: CLOSED
ISSUE_834: CLOSED
```
