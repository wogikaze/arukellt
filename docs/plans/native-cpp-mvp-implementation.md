# native-cpp MVP implementation plan

ステータス: 実装計画（決定記録ではない）
関連 ADR: [ADR-049](../adr/ADR-049-native-c99-selfhost-executor.md)
詳細仕様: [RFC-008](../rfcs/008-native-cpp-c99-backend-runtime-abi.md)
Capability SSOT: [`data/native-cpp-capabilities.toml`](../../data/native-cpp-capabilities.toml)
作成日: 2026-07-22

---

## 現状とゴール

現行 `native-cpp` は固定GNU assemblyを返すscaffoldであり、MIR lowering、C runtime、link、
native runを実装していない。
本計画はscaffoldからexperimental selfhost executorへ移行する順序を定める。

MVPの完了条件は、Linux x86-64でnative executorが比較対象S2のbuild profileを継承して
S3を生成し、同一profileのS2とS3でbyte equalityを満たすことである。
正規のWasm fixpointはADR-029のまま維持する。

## Phase 0: 文書とscaffold境界

Phase 0は実装前の契約を固定する。
backend実装、runtime実装、build、fixture実行、fixpoint、性能計測は行わない。

### 作業

- ADR-049、RFC-008、本planを作成する。
- 全MIR opcodeと全CoreOpをcapability registryへ一度ずつ登録する。
- manager commandとperformance receiptの契約をRFCへ記載する。
- capability validatorをPhase 1の最初の実装項目として計画する。
- Phase 1開始時にconstant returnのfailing fixtureを最初に追加する。

### 完了条件

- ADR-045がADR-049によりSUPERSEDEDである。
- current-stateとproject-stateがscaffoldを維持する。
- capability registryが正本の65 MIR opcodeと294 CoreOpを完全に列挙する。
- documentation validationとADR validationが成功する。
- 実装開始ゲートがOPENになる。

## Phase 1: Primitive C emitter

Phase 1はheap aggregateを必要としないscalar programをC99へlowerする。
String constantはobject runtimeに依存するためPhase 2で有効化する。

### 対象

- scalar type、unit、constant、local、temporary
- arithmetic、comparison、integer cast、float cast、masked shift
- branch、conditional branch、`br_table`、loop marker、phi
- direct call、multiple return、return
- trap、drop、nop、unreachable

### Fixture順

| 順序 | Fixture契約 |
|------|-------------|
| 1 | constant return |
| 2 | integer arithmetic |
| 3 | wrap overflow |
| 4 | masked shift |
| 5 | branch |
| 6 | loop |
| 7 | phi parallel copy |
| 8 | direct call |
| 9 | multiple return |
| 10 | trap |

### 成果物

- C emitter golden
- generated Cのclang compile test
- generated executable run test
- Wasm/native differential fixture
- capability completeness validator

### 完了条件

- Phase 1のPlanned MIR entryがSupportedへ移り、implementation ownerを持つ。
- scalar fixtureのstdout、stderr、exit codeがWasm executorと一致する。
- signed overflow、shift、division、float conversionがCの未定義動作へ依存しない。
- Unsupported entryがemit前target diagnosticになる。

## Phase 2: Object runtime

Phase 2はprocess-lifetime arenaとheap objectを実装する。

### 対象

- arena、16-byte alignment、allocation counter
- object header、struct、tuple
- enum、Option、Result、multiple payload
- String、array、Vec、slice
- primitive boxing、Any

### 完了条件

- 全objectがTypeTable所有のTypeIdとzero flagsを持つ。
- arrayが一allocationのtyped flexible array layoutを使う。
- Vecがtyped bufferを使い、`uintptr_t`万能bufferを持たない。
- Stringとaggregate fixtureがWasm/native differentialを満たす。
- allocation overflow、bounds、null、TypeId mismatchが規定された失敗になる。

## Phase 3: Higher-order機能

Phase 3はtyped indirect callとclosure familyを実装する。

### 対象

- indirect call、function reference
- closure function pointerとenvironment
- monomorphized generic instance
- trait dispatchとvtable identity
- checked castとAny unbox

### 完了条件

- indirect callは完全signatureごとのtyped function pointerを使う。
- backendにcallee名のsemantic dispatchがない。
- genericは既存monomorphization結果だけを受け取る。
- closure、trait、Any fixtureがWasm/native differentialを満たす。

## Phase 4: Host runtime

Phase 4はセルフホストコンパイラが必要とするambient host operationを実装する。

### 対象

- argsとenvironment variable
- filesystem read、write、readable-file probe
- stdin、stdout、stderr
- process exit、abort、panic
- monotonic clockとwall clock

### Fixture契約

各host operationにsuccessとerror fixtureを用意する。
filesystemはnot found、permission、invalid UTF-8、partial I/O、large-file overflowのうち
再現可能な境界を個別に検証する。

### 完了条件

- path encoding、embedded NUL、errno mappingがRFC-008と一致する。
- argsはargv[0]を除外する。
- clock unitがmonotonic nanosecondsとwall-clock millisecondsを維持する。
- network、random、WITを実装済みと誤認させない。

## Phase 5: Selfhost integration

Phase 5はmanager、clang、cache、receiptを結び、native executor laneを完成させる。

### 到達順

| 順序 | 到達点 |
|------|--------|
| 1 | `arukellt-native --help` |
| 2 | native executorがhelloをWasmへcompile |
| 3 | compiler fixture subsetのdifferential |
| 4 | compiler source全体をnative executableへcompile |
| 5 | native executorが`s3.wasm`を生成 |
| 6 | 同一build profileで`sha256(s2) == sha256(s3)` |
| 7 | 同じnative executorで二回生成してdeterminism確認 |
| 8 | performance receipt取得 |
| 9 | warm S3が5分未満 |
| 10 | `executor_peak_rss_bytes <= 2.4 GiB` |

最終性能目標はwarm S3を2分未満とする。
thresholdやbaselineを変更するときはbenchmark governanceを適用する。

### Manager command

実装後のcommand contractは次とする。

```bash
python3 scripts/manager.py selfhost native-executor --build
```

実装command:

```bash
python3 scripts/manager.py selfhost native-executor --build
```

S2のbuild-profile manifestを継承し、同一profileのS2/S3 byte equality・determinism・
performance receiptを検証する。`docs/data/verification-commands.toml`へ登録する。

### 完了条件

- cache keyがRFC-008の全入力を含む。
- byte equality失敗が両hashとreceipt pathを報告する。
- determinism二回分のhashがreceiptへ残る。
- native lane失敗時に正規fixpointへ黙ってfallbackしない。
- ADR-029の正規fixpoint commandと意味を変更しない。

## PR境界

PRは次の責務境界で分割する。

| PR | Scope |
|----|-------|
| 1 | ADR、RFC、plan、capability schema |
| 2 | primitive C typeとsymbol mangling |
| 3 | CFG、branch、phi |
| 4 | arithmetic、cast、trap semantics |
| 5 | direct callとindirect call ABI |
| 6 | arenaとobject header |
| 7 | aggregate、enum、Option、Result |
| 8 | String、array、Vec、slice |
| 9 | Any、closure、generic、trait dispatch |
| 10 | host runtime |
| 11 | manager、clang、cache |
| 12 | selfhost executor lane |
| 13 | byte equality、determinism、performance gate |

各PRは一つ前のfixtureをgreenにし、次の未対応機能をstubで成功させない。
同じownerを変更するPRは表の順で統合する。

## 初期Unsupported

MVP初期値でUnsupportedとする範囲は次である。

- v128とSIMD
- futureとasync
- WITとComponent Model
- networkとhost random
- external FFI、public C ABI、non-Linux host、cross compilation

compiler自身のMIRにUnsupportedが現れた場合は、capability validatorがself-compileを拒否する。
coverage receiptを確認して対応phaseを変更し、Unsupportedのままstubを入れない。

## 検証コマンド

Phase 0の文書変更で実行する既存commandは次だけとする。

```bash
python3 scripts/manager.py docs regenerate
python3 scripts/manager.py docs check
python3 scripts/check/check-adrs.py
```

Phase 1以降の編集loopでは、実装時点の
[`verification-commands.toml`](../data/verification-commands.toml)に登録されたcommandを使う。
現時点で存在する基礎commandは次である。

```bash
python3 scripts/manager.py fmt --check
python3 scripts/manager.py verify lane
python3 scripts/manager.py verify fixtures
python3 scripts/manager.py selfhost build-compiler
```

`selfhost native-executor --build`、capability completeness、C emitter golden、clang compile、
native run、Wasm/native differential、receipt gateは未実装である。
各ownerの実装PRでcommand registryへ追加する。

正規fixpoint regressionはPhase 5の完了時またはCIで次を実行する。

```bash
python3 scripts/manager.py selfhost fixpoint
```

日常のemitter rebuildやPhase 0文書作業では実行しない。

## Current-state更新規則

ADR、RFC、plan、capability registryの作成だけではtarget stateを変更しない。
Phase 5の実装と検証が完了するまで`support_tier = "scaffold"`、
`implementation_state = "scaffold"`、`run_supported = false`を維持する。

各phase完了時はcapability entry、issue、必要なcurrent-state SSOTを同じ変更で更新する。
進捗件数と一時的なfixture結果を本planへ追記しない。

## リスクと依存

- compilerのtyped MIRに必要なsubword型またはsignature情報がなければ、backendで推測せず上流を直す。
- process-lifetime arenaがRSS gateを超えた場合はADR-049の再検討条件に従う。
- CoreOp registryはmigration中であるため、placeholder signatureをnative ABIの正本にしない。
- String意味論の文書ドリフトはnative実装で変更せず、Wasm differentialを基準にする。
- 正規fixpointの実行時間問題はnative laneで緩和し、ADR-029の証明内容を弱めない。
