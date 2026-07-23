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

- [ ] Phase 2開始後、Phase 8と最終チェックリスト完了まで自動的に次作業へ進む
- [ ] 各Phase完了時はreceiptとcheckboxを更新し、同じ作業セッションで次Phaseを開始する
- [ ] 実装単位ごとにcommitするが、commit後にユーザー確認を待たない
- [ ] narrow test失敗時は原因を切り分け、修正し、再検証してから続行する
- [ ] broad gate失敗時は成功済み部分を捨てず、失敗を分類して修正を続ける
- [ ] 一つのsubtaskがblockedでも、独立して進められる計測・fixture・docs・runner作業を続ける
- [ ] 性能gate未達なら、計測値に従ってPhase 4またはPhase 5の次の分岐を実行する
- [ ] `--allow-high-rss`成功を最終成功として扱わない
- [ ] stress greenだけで#833や#834を閉じない
- [ ] strict run 1回成功だけで昇格しない。3回連続PASSを必須とする
- [ ] #833を閉じても#834を継続し、最終昇格まで止めない

### 真の停止条件

次の場合のみ全体作業をBLOCKEDとして停止できる。

- 必須toolまたはartifactが存在せず、リポジトリ内で再生成・代替できない
- destructive operationまたは外部credentialが必要で、安全に自動実行できない
- ACCEPTED ADR同士が衝突し、実装だけでは決定できない
- 同じ非一時的失敗を、再現縮小・原因分類・少なくとも二つの修正案検証後も解消できない
- 実行環境の物理上限でstrict gateを測れず、別環境が必須である

停止時にも以下を残す。

- [ ] 最小再現
- [ ] 実行コマンド、exit code、主要ログ
- [ ] 既に除外した仮説
- [ ] 次に試す具体的変更
- [ ] 安全に完了したcommit
- [ ] 未完了checkbox

「時間がかかる」「Phaseが終わった」「次は別PR」「必要なら続ける」は停止理由にしない。

## 1. Experimental昇格の定義

正規strict command:

```bash
python3 scripts/manager.py selfhost native-executor --build
```

昇格対象は内部selfhost executor laneのみ。昇格後も次を維持する。

- [ ] `support_tier = "scaffold"`
- [ ] `run_supported = false`
- [ ] public `arukellt run --target native-cpp`を保証しない
- [ ] public C ABI、外部FFI、配布native executableを保証しない
- [ ] `#831` wasm32-gc正規fixpointをこの昇格のblockerにしない

最終必須条件:

- [ ] strict command `exit_code == 0`
- [ ] `high_rss_override == false`
- [ ] `correctness_gate_passed == true`
- [ ] `performance_gate_passed == true`
- [ ] `memory_gate_passed == true`
- [ ] `strict_gate_passed == true`
- [ ] warm executor wall `< 300,000 ms`
- [ ] executor peak RSS `<= 2.4 GiB`
- [ ] S3 wasm validation PASS
- [ ] current-source S2 == S3 byte equality PASS
- [ ] 2回生成のS3 determinism PASS
- [ ] root liveness analyzed == GC-frame functions
- [ ] root liveness skipped == 0
- [ ] root clear enabled == true
- [ ] planned clears == emitted clears
- [ ] root-clear有効状態でGC stress PASS
- [ ] strict gate 3回連続PASS
- [ ] CI、docs、state、false-done gate更新完了

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

- [ ] 全reachable MIR instructionに安定したinstruction orderがある
- [ ] unreachable instructionがeffect列へ入らない
- [ ] reference localのuse/defがemitterの実value stack由来である
- [ ] CALL引数をMIR fieldから推測していない
- [ ] CALL receiverをuseとして保持する
- [ ] CALL戻り値destinationをdefとして扱う
- [ ] LOCAL_SET sourceを実stackから取得する
- [ ] RETURN operandをuseとして扱う
- [ ] STRUCT_SET objectとvalueをuseとして扱う
- [ ] STRUCT_GET objectをuseとして扱う
- [ ] closure environment operandをuseとして扱う
- [ ] instruction effect生成とC emitで同じstack semanticsを使う
- [ ] effect生成passが不要なC本文を保持しない

## 2.2 CFG liveness proof

標準式を使用する。

```text
live_out[B] = union(live_in[S]) for S in successors(B)
live_in[B]  = use[B] union (live_out[B] - def[B])
```

- [ ] predecessor worklistで収束まで解析する
- [ ] arbitrary iteration limitがない
- [ ] block数上限がない
- [ ] reference local数上限がない
- [ ] loop backedgeを処理する
- [ ] nested loopを処理する
- [ ] diamond CFGのjoinを処理する
- [ ] early returnを処理する
- [ ] unreachable blockを除外する
- [ ] block indexとBlockId対応を明示する
- [ ] PHIをsupported扱いで推測しない
- [ ] 到達PHIがある場合はcapability diagnosticで拒否する
- [ ] 解析非収束はsilent skipではなくcompiler errorにする

## 2.3 Safepoint SSOT

safepointは「命令中にallocationまたはGCへ到達し得る地点」と定義する。

- [ ] MIR opcode safepoint属性を一か所へ集約する
- [ ] CoreOp allocation属性を一か所へ集約する
- [ ] CONST_STRINGをsafepointにする
- [ ] STRUCT_NEWをsafepointにする
- [ ] 対応済みGC_STRUCT_NEWをsafepointにする
- [ ] ARRAY_NEW対応時にsafepoint属性を要求する
- [ ] 全direct CALLをsafepointにする
- [ ] allocation可能runtime helperの呼出し箇所を監査する
- [ ] non-safepointからallocatorへ到達する新経路をcheckで拒否する
- [ ] capability追加時にallocation属性未指定ならtestを失敗させる

## 2.4 Clear-plan invariants

safepoint直前にclear可能なのは、命令引数ではなく、命令後にどのsuccessorでも読まれないreference localのみ。

- [ ] current instruction usesをclearしない
- [ ] CALL argumentsをclearしない
- [ ] receiverをclearしない
- [ ] return operandをclearしない
- [ ] branch-carried valueをclearしない
- [ ] aggregate object/value operandをclearしない
- [ ] loop-carried referenceをclearしない
- [ ] overwrite前の旧referenceは必要use後にclear可能になる
- [ ] result destinationは命令後にroot slotへ反映される
- [ ] clear setはsafepoint instructionだけ保存する
- [ ] 全命令×全localの巨大行列を生成しない
- [ ] packed bitsetまたはsparse setを使う
- [ ] planner peak memoryをreceiptへ出す

## 2.5 Planner unit tests

- [ ] 直線コードのlast-use後dead
- [ ] CALL引数としてlive
- [ ] CALL戻り値が後続でlive
- [ ] CALL戻り値が即dead
- [ ] branch片側だけでuse
- [ ] branch両側でuse
- [ ] join後でuse
- [ ] loop-carried reference
- [ ] loop内だけでuse
- [ ] nested loop
- [ ] early return
- [ ] unreachable block
- [ ] local overwrite前の旧reference
- [ ] struct field store
- [ ] struct field load
- [ ] Vec store
- [ ] String clone receiver
- [ ] closure environment
- [ ] Result payload
- [ ] Option payload
- [ ] recursive call
- [ ] mutually recursive call

各testでinstruction orderごとのexpected clear local IDsを直接比較する。

## 2.6 Generated-C golden and receipt

function entry初期化とdead-root clearを別集計する。

- [ ] debug Cに`ark-root-clear` markerをemitできる
- [ ] markerにfunction、instruction、localを含める
- [ ] entry NULL initialization countを別fieldにする
- [ ] root clear assignment countを別fieldにする
- [ ] planned sitesとplanned assignmentsを分ける
- [ ] emitted sitesとemitted assignmentsを分ける
- [ ] planned assignments == emitted assignmentsをrunnerで検証する
- [ ] unreachable instructionへmarkerが出ない
- [ ] C grepだけをacceptance evidenceにしない

receipt必須field:

- [ ] `root_functions_with_frames`
- [ ] `root_functions_analyzed`
- [ ] `root_functions_skipped`
- [ ] `root_reference_local_count`
- [ ] `root_safepoint_count`
- [ ] `root_clear_sites_planned`
- [ ] `root_clear_assignments_planned`
- [ ] `root_clear_sites_emitted`
- [ ] `root_clear_assignments_emitted`
- [ ] `root_peak_slots`
- [ ] `root_planner_peak_bytes`
- [ ] `root_liveness_enabled`
- [ ] `root_liveness_fallback_count`

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

- [ ] internal optionでfixtureのみclear emitを有効化する
- [ ] `ARUKELLT_NATIVE_GC_THRESHOLD_BYTES=65536`で既存11 fixturesを実行する
- [ ] collectionが各critical call直前に起こり得るfixtureを追加する
- [ ] `String::clone(NULL)`が0
- [ ] missing-root failureが0
- [ ] invalid object pointerが0
- [ ] use-after-freeが0
- [ ] double freeが0
- [ ] reclaimed bytesが0より大きいfixtureを維持する

追加fixture:

- [ ] dead rootがcollectionで回収される
- [ ] current CALL argumentは回収されない
- [ ] current receiverは回収されない
- [ ] CALL return valueは後続useまで保持される
- [ ] branch join後のlive rootが保持される
- [ ] loop-carried rootが保持される
- [ ] overwriteされた旧objectだけが回収される
- [ ] closure environmentが保持される
- [ ] nested aggregateが保持される
- [ ] String bufferがlive中に解放されない
- [ ] Vec side bufferがlive中に解放されない
- [ ] cycle objectはroot消失後に回収される

## 3.2 Sanitizer fixture gate

- [ ] runtime CをAddressSanitizer付きでcompile
- [ ] runtime CをUndefinedBehaviorSanitizer付きでcompile
- [ ] root-clear fixturesをASanで実行
- [ ] root-clear fixturesをUBSanで実行
- [ ] use-after-free 0
- [ ] out-of-bounds root slot 0
- [ ] invalid alignment 0
- [ ] leak結果をruntime designに沿って分類
- [ ] `-Wall -Wextra -Wpedantic -Werror` PASS

## 3.3 Small-function rollout

解析は全関数へ適用したまま、emit対象だけ段階拡大する。

Stage A:

- [ ] `block_count <= 8`
- [ ] `reference_local_count <= 16`
- [ ] enabled functionsをreceiptへ出す
- [ ] disabled functionsをreceiptへ出す
- [ ] S2/S3 equality PASS
- [ ] determinism PASS
- [ ] validate PASS
- [ ] GC stress PASS

Stage B:

- [ ] `block_count <= 16`
- [ ] `reference_local_count <= 32`
- [ ] correctness gate全部PASS
- [ ] live object数とwall/RSSを測定

Stage C:

- [ ] `block_count <= 64`
- [ ] `reference_local_count <= 128`
- [ ] correctness gate全部PASS
- [ ] live object数とwall/RSSを測定

Stage D:

- [ ] emit上限を削除
- [ ] 全analyzed functionでclear有効
- [ ] fallback 0
- [ ] planned assignments == emitted assignments
- [ ] full-S3 S2 equality PASS
- [ ] full-S3 determinism PASS
- [ ] full-S3 validate PASS

## 3.4 Production enable

- [ ] `enable_root_clears = false`を削除またはtrueへ切替
- [ ] hidden silent fallbackを削除
- [ ] GC laneでroot clearを常時有効化
- [ ] arena laneも同一functional Cを使用
- [ ] GC無効時の追加runtime overheadを最小化
- [ ] root clear有効状態をreceiptへ明示
- [ ] current-source S3を必要に応じS2へpromote
- [ ] promote provenanceを記録

## Phase 3完了条件

- [ ] full compiler全関数でroot clear enabled
- [ ] root liveness skipped == 0
- [ ] root liveness fallback == 0
- [ ] planned clear assignments == emitted clear assignments
- [ ] fixture stress全件PASS
- [ ] sanitizer fixture PASS
- [ ] full-S3 equality/determinism/validate PASS
- [ ] Phase 3完了commit

Phase 3完了後は同じroot-clear有効buildでPhase 4の3回計測へ進む。

# Phase 4 — 性能再計測と必要最小限のGC最適化

## 4.1 Root-clear後baseline

arenaとGCをそれぞれ3回計測する。Phase 0 baselineは上書きしない。

- [ ] 新しいtimestamped baseline directoryを作る
- [ ] source commitを保存
- [ ] S2/S3 hashを保存
- [ ] runtime hashを保存
- [ ] profile fingerprintを保存
- [ ] arena 3回
- [ ] GC 3回
- [ ] cold/warmを分離
- [ ] cache hit/missを保存
- [ ] run 1/run 2のGC statsを分離

比較指標:

- [ ] warm wall
- [ ] peak RSS
- [ ] collections
- [ ] live objects per collection
- [ ] max live objects
- [ ] object bytes
- [ ] side buffer bytes
- [ ] object table bytes
- [ ] root slots scanned
- [ ] marked objects
- [ ] mark time
- [ ] sweep time
- [ ] table rebuild time
- [ ] malloc_trim time
- [ ] allocation time
- [ ] reclaimed object bytes
- [ ] reclaimed side bytes

## 4.2 Gate分岐

### Branch A — wall <300s and RSS <=2.4GiB

- [ ] collector構造の大改造をしない
- [ ] correctnessを再確認
- [ ] Phase 6へ進む

### Branch B — live set縮小済み、collection回数が多い

threshold policyを計測ベースで調整する。

- [ ] collectionごとのlive bytesを記録
- [ ] collection間allocated bytesを記録
- [ ] RSS headroomを記録
- [ ] `live/2` policyを測定
- [ ] `live` policyを測定
- [ ] `live*3/2` policyを測定
- [ ] 2.4 GiBを超えない最速policyを選ぶ
- [ ] minimum growthを固定
- [ ] maximum growthをRSS budgetから制約
- [ ] default threshold変更はbenchmark receipt付きで行う
- [ ]変更後3回計測

### Branch C — mark time支配

まずiterative markへ変更する。

- [ ] explicit mark stackを追加
- [ ] recursive markを除去
- [ ] root scanからmark stackへpush
- [ ] Struct childをmark stackへpush
- [ ] Vec childをmark stackへpush
- [ ] cycle処理を維持
- [ ] mark stack peak sizeをreceiptへ出す
- [ ] deep graph fixtureを追加
- [ ] correctness gate全部PASS
- [ ]変更後3回計測

それでも未達ならtyped scanを検討する。

- [ ] Struct reference bitmapまたはlayout ID設計
- [ ] scalar fieldをscanしない
- [ ] reference fieldだけ直接mark
- [ ] Vec element kindをscalar/reference/mixedで表現
- [ ] scalar-only Vecをscanしない
- [ ] reference-only Vecをhash lookupなしでscan
- [ ] mixed Vecのtag/bitmap ownerを明確化
- [ ] root slotsをexact referenceとして扱う
- [ ] debug membership checkは維持
- [ ] release mark pathのhash lookupを削減
- [ ] object table縮小効果を測定
- [ ] correctness gate全部PASS
- [ ]変更後3回計測

### Branch D — sweep/table rebuild支配

- [ ] sweep timeをobject freeとside freeに分離
- [ ] table rebuild timeを分離
- [ ] tombstone方式を比較
- [ ] full rehash頻度を測定
- [ ] load factorとtombstone率でrehash条件を固定
- [ ] table capacity shrink条件を固定
- [ ] live countに対するtable過大率を記録
- [ ]変更後3回計測

### Branch E — allocator/trim支配

- [ ] `posix_memalign`時間を計測
- [ ] individual free時間を計測
- [ ] `malloc_trim(0)`時間を計測
- [ ] trimを毎collection、条件付き、終了時のみで比較
- [ ] 大量回収時のみtrimする条件を検証
- [ ] 必要ならfixed-size objectのsize-class/slabを検討
- [ ] String/Vec side allocationを分離測定
- [ ]変更後3回計測

## Phase 4完了条件

- [ ] GC warm wall `<300,000 ms`
- [ ] GC peak RSS `<=2.4 GiB`
- [ ] equality/determinism/validate維持
- [ ] 改善の主要因をreceiptで説明可能
- [ ] 不要な全面改造を避けた
- [ ] Phase 4完了commit

wall/RSSを同時に満たせない場合は停止せずPhase 5へ進む。

# Phase 5 — compiler live IRの段階解放（必要時）

Phase 4後もgate未達の場合のみ実施する。

## 5.1 Retention report

- [ ] object kind別live count
- [ ] String / Vec / Struct別live bytes
- [ ] allocation helper別live count
- [ ] compiler phase origin別live count
- [ ] stack rootsとglobal rootsを分離
- [ ] root slot別retained object概算をdebug modeで取得
- [ ] collectionごとの上位retained categoryを出す

調査候補:

- [ ] source text buffers
- [ ] parser AST
- [ ] CoreHIR
- [ ] MIR module/body
- [ ] type/signature registries
- [ ] module loader caches
- [ ] monomorphization caches
- [ ] diagnostics buffers
- [ ] dump/debug state
- [ ] output buffers

## 5.2 Owner release

manual object freeではなく、不要owner referenceを切る。

- [ ] parse終了後の不要source/AST ownerを解放
- [ ] CoreHIR完了後の不要AST cacheを解放
- [ ] MIR lowering後の不要CoreHIR ownerを解放
- [ ] emit終了済みMIR bodyを保持しない
- [ ] dump無効時にdump stateを作らない
- [ ] cacheが全compile graphを保持していないか修正
- [ ] phase境界でownerを切った後にのみoptional GCを試す
- [ ] phase GC回数とwall効果を測定
- [ ] correctness gate全部PASS
- [ ]変更後3回計測

## Phase 5完了条件

- [ ] retained object上位カテゴリを説明可能
- [ ]不要phase ownerがrootから外れる
- [ ] wall/RSS同時gate PASS
- [ ] Wasm executorの意味論を変更していない
- [ ] Phase 5完了commit

# Phase 6 — final correctness、安全性、strict 3×

## 6.1 GC stress matrix

- [ ] 既存11 fixtures PASS
- [ ] root-clear追加fixtures PASS
- [ ] threshold 64 KiB PASS
- [ ] threshold 1 MiB PASS
- [ ] default threshold PASS
- [ ] collection count >0のfixtureを含む
- [ ] reclaimed bytes >0のfixtureを含む
- [ ] cycle回収PASS
- [ ] live String保持PASS
- [ ] live Vec保持PASS
- [ ] call argument保持PASS
- [ ] call receiver保持PASS
- [ ] call return保持PASS
- [ ] loop-carried保持PASS

## 6.2 Runtime safety

- [ ] ASan fixture PASS
- [ ] UBSan fixture PASS
- [ ] use-after-free 0
- [ ] double free 0
- [ ] invalid alignment 0
- [ ] root slot OOB 0
- [ ] mark stack overflow 0
- [ ] C99 warning-as-error compile PASS

## 6.3 Full-S3 correctness

- [ ] GC有効でS3を2回生成
- [ ] S3 run 1 == S3 run 2
- [ ] current-source S2 == S3
- [ ] wasm-tools validate PASS
- [ ] output profileがS2 manifestを継承
- [ ] targetをwasm32-gcへhardcodeしない
- [ ] dump on/offでfunctional hash不変
- [ ] cache hit/missでfunctional hash不変
- [ ] arenaとGCでfunctional hash一致

## 6.4 Strict 3×

同一基準マシン、同一commit、同一S2、同一profileで実行する。

- [ ] strict run 1 exit 0
- [ ] strict run 1 warm wall <300s
- [ ] strict run 1 RSS <=2.4GiB
- [ ] strict run 2 exit 0
- [ ] strict run 2 warm wall <300s
- [ ] strict run 2 RSS <=2.4GiB
- [ ] strict run 3 exit 0
- [ ] strict run 3 warm wall <300s
- [ ] strict run 3 RSS <=2.4GiB
- [ ] 3回ともcorrectness gate true
- [ ] 3回ともperformance gate true
- [ ] 3回ともmemory gate true
- [ ] 3回ともstrict gate true
- [ ] 3回ともhigh-RSS override false
- [ ] 3回ともwarningなし
- [ ] worst wallをpromotion receiptへ保存
- [ ] worst RSSをpromotion receiptへ保存

平均値ではなく全runがgateを満たすこと。

## Phase 6完了条件

- [ ] stress matrix全件PASS
- [ ] sanitizer PASS
- [ ] full-S3 correctness PASS
- [ ] strict 3回連続PASS
- [ ] Phase 6完了commit

Phase 6完了後は停止せずPhase 7へ進む。

# Phase 7 — manager、receipt、CI enforcement

## 7.1 Command behavior

- [ ] strict command既定を低RSS GC laneにする
- [ ] overrideを暗黙使用しない
- [ ] `--allow-high-rss`をlocal escape hatchとしてのみ残す
- [ ] CIで`--allow-high-rss`を拒否
- [ ] correctness失敗時にarenaへfallbackしない
- [ ] performance失敗時にexit 0へしない
- [ ] missing receiptを失敗にする
- [ ] receipt schema mismatchを失敗にする
- [ ] stale reference N/Aはbaseline modeだけで許可
- [ ] strict promotion modeではcurrent-source equality必須

## 7.2 Receipt final schema

- [ ] schema version
- [ ] source commit
- [ ] dirty state
- [ ] S2 hash
- [ ] S3 run hashes
- [ ] runtime hash
- [ ] profile fingerprint
- [ ] cache state
- [ ] equality applicability/status
- [ ] correctness/performance/memory/strict booleans
- [ ] root liveness stats
- [ ] GC timing stats
- [ ] cold/warm run separation
- [ ] high-RSS override flag
- [ ] promotion eligibility flag

## 7.3 CI placement

- [ ] PR quickでcapability checker
- [ ] PR quickでroot liveness unit tests
- [ ] PR quickでGC stress fixtures
- [ ] PR quickでruntime C warning-as-error
- [ ] scheduledまたはmerge gateでfull strict executor
- [ ] release gateでADR-029正規fixpoint維持
- [ ] native executorを正規fixpointの代替にしない
- [ ] CI artifactとしてpromotion receiptを保存

## Phase 7完了条件

- [ ] manager behavior tests PASS
- [ ] CI configuration check PASS
- [ ] override禁止がtestで証明される
- [ ] strict receipt validation PASS
- [ ] `verify quick` PASS
- [ ] Phase 7完了commit

# Phase 8 — docs、state、false-done、昇格

## 8.1 Docs sync

更新対象:

- [ ] `docs/current-state.md`
- [ ] `docs/adr/ADR-049-native-c99-selfhost-executor.md`
- [ ] `docs/rfcs/008-native-cpp-c99-backend-runtime-abi.md`
- [ ] `docs/plans/native-cpp-experimental-promotion.md`
- [ ] `docs/plans/native-cpp-mvp-implementation.md`
- [ ] `docs/data/project-state.toml`
- [ ] `data/native-cpp-capabilities.toml`
- [ ] `issues/open/833-*`
- [ ] `issues/open/834-*`
- [ ] false-done gate 641

記載事項:

- [ ] arena/GC dual modeを正式記載
- [ ] root clear有効を記載
- [ ] root clear統計定義を記載
- [ ] warm測定区間を統一
- [ ] 古い8分/10–11分値を最新receiptへ置換
- [ ] arena値とGC値を混同しない
- [ ] strict 3× receiptを記載
- [ ] target stateとexecutor lane stateを分離
- [ ] public native target完成ではないと明記
- [ ] `--allow-high-rss`を成功条件から除外
- [ ] #831との分離を維持
- [ ] historical MVP planをhistorical化または完了反映

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

- [ ] target stateとexecutor lane stateを分離
- [ ] machine-readable schemaで検証
- [ ] generated docsへ反映
- [ ] 自由文字列だけに依存しない

## 8.3 False-done gate

- [ ] gate 641が`support_tier=scaffold`を許容
- [ ] `implementation_state=partial`を許容
- [ ] executor lane experimentalを要求
- [ ] strict receipt存在を要求
- [ ] `high_rss_override == false`を要求
- [ ] correctness/performance/memory/strict全trueを要求
- [ ] root liveness enabledを要求
- [ ] skipped/fallback 0を要求
- [ ] strict 3× evidenceを要求
- [ ] stale old receiptでPASSしない

## 8.4 Issue closure and promotion

#833 close条件:

- [ ] production root clear enabled
- [ ] CFG/call/loop fixtures PASS
- [ ] skipped/fallback 0
- [ ] root clearによるUAF 0
- [ ] dual wall/RSS gateへの寄与をreceiptで示す

#834 close条件:

- [ ] Phase 2–8すべて完了
- [ ] 最終チェックリスト全項目完了
- [ ] strict 3× PASS
- [ ] CI enforcement完了
- [ ] docs/state/false-done同期完了
- [ ] issue close review PASS
- [ ] open→done移動
- [ ] issue index再生成
- [ ] promotion commit作成

## Phase 8完了条件

- [ ] docs check PASS
- [ ] state consistency PASS
- [ ] false-done check PASS
- [ ] issue index生成PASS
- [ ] `verify quick` PASS
- [ ] #833 closed
- [ ] #834 closed
- [ ] native executor lane experimentalへ昇格

# Final Experimental Promotion Checklist

このセクションが最終終了条件である。一つでも未完了なら作業を終了しない。

## Correctness

- [ ] current-source S2/S3 byte equality PASS
- [ ] S3/S3 determinism PASS
- [ ] wasm validation PASS
- [ ] arena/GC functional output一致
- [ ] dump on/off一致
- [ ] cache hit/miss一致
- [ ] missing-root failure 0
- [ ] invalid-clear failure 0
- [ ] GC stress全件PASS
- [ ] sanitizer fixture PASS

## Root liveness

- [ ] 全GC-frame関数を解析
- [ ] analyzed == functions with frames
- [ ] skipped == 0
- [ ] fallback == 0
- [ ] CALL args/receiver/return対応
- [ ] CFG join対応
- [ ] loop backedge対応
- [ ] unreachable除外
- [ ] safepoint SSOT
- [ ] production clear enabled
- [ ] planned assignments == emitted assignments
- [ ] entry NULL初期化とdead-root clearを別集計

## Performance

- [ ] warm wall `<300,000 ms`
- [ ] peak RSS `<=2.4 GiB`
- [ ] strict run 3回すべてPASS
- [ ] high-RSS override未使用
- [ ] warningなし
- [ ] mark/sweep/table/trim内訳取得
- [ ] baseline前後比較保存
- [ ] worst-case receipt保存

## Operational

- [ ] strict command exit 0
- [ ] correctness gate true
- [ ] performance gate true
- [ ] memory gate true
- [ ] strict gate true
- [ ] cache hit path PASS
- [ ] cache miss path correctness PASS
- [ ] CI override禁止
- [ ] silent fallbackなし
- [ ] receipt schema検証あり

## Documentation and governance

- [ ] current-state更新
- [ ] ADR-049更新
- [ ] RFC-008更新
- [ ] historical plan整理
- [ ] project-state更新
- [ ] capability registry更新
- [ ] false-done gate更新
- [ ] #831分離維持
- [ ] public native targetではないことを明記
- [ ] #833 issue close review PASS
- [ ] #834 issue close review PASS
- [ ] docs check PASS
- [ ] verify quick PASS

## Final state

- [ ] `native-cpp` target remains scaffold / partial / experimental / run_supported=false
- [ ] native selfhost executor lane is experimental
- [ ] strict gate supported=true
- [ ] current-stateとmachine-readable stateが一致
- [ ] promotion receiptが保存されている
- [ ] 全変更がcommit済み
- [ ] working tree clean

全項目完了後のみ次の最終状態を報告する。

```text
NATIVE_CPP_EXPERIMENTAL_PROMOTION: COMPLETE
STRICT_RUNS: 3/3 PASS
HIGH_RSS_OVERRIDE: false
ISSUE_833: CLOSED
ISSUE_834: CLOSED
```
