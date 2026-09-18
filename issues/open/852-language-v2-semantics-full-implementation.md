---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #686, #691, #693, #694, #695, #728, #729, #801"
Orchestration class: architecture-implementation
Orchestration upstream: None
Blocks v{N}: language-v2
Priority: 1
Source: "ADR-058 / RFC-011 full implementation umbrella"
---

# 852 — 言語 v2 意味論再設計を全項目実装する

## Summary

[ADR-058](../../docs/adr/ADR-058-language-v2-semantics-redesign.md) と
[RFC-011](../../docs/rfcs/011-language-v2-semantics-redesign.md) に記録した
45 項目を、設計採択、正規仕様、コンパイラ、stdlib、Wasm backend、
linear-memory fallback、移行診断、fixture、性能 receipt まで完了させる umbrella issue。

この issue 自体は「45 項目を 1 commit で変更する」ためのものではない。
独立に検証できる変更は必ず focused child issue に分割し、原則
**1 issue = 1 logical commit** で実装する。この issue は全 child issue の完了証拠を集約する。

## Canonical sources

- ADR-058: 言語意味論と target/backend 表現の責務分離
- RFC-011: 45 項目の設計カタログ、未決事項、採択条件
- ADR-002 / ADR-013 / ADR-035: 現行 GC / wasm32-gc 契約
- ADR-014 / ADR-018: stability と仕様文書の分類
- ADR-031: import / WIT 境界
- ADR-036 / ADR-044 / RFC-004: Trait / stdlib
- ADR-038 / ADR-039 / ADR-046: operator / ? / free-function migration

ADR/RFC と実装が食い違った場合、実装を正として黙って進めない。
accepted decision を追加または supersede してから実装する。

## Completion contract

- [ ] RFC-011 の 45 項目がすべて「仕様 + 実装 + fixture/receipt」の三点で完了している
- [ ] PROPOSED/DRAFT のまま implementation を既成事実化した項目がない
- [ ] 既存 ACCEPTED ADR と衝突する変更には successor/superseding ADR がある
- [ ] 独立 work unit は child issue に分割され、各 child issue に acceptance と evidence がある
- [ ] wasm32-gc と fallback backend が通常の source semantics を共有する
- [ ] Wasm GC 対応環境では Wasm GC lowering が primary/default path として実際に使用される
- [ ] Wasm GC 非対応環境では linear-memory runtime fallback で同じ通常プログラムが実行できる
- [ ] 共有・循環参照を含むプログラムが backend の違いだけで source-level illegal にならない
- [ ] stable API 変更には ADR-014 に従う migration/deprecation path がある
- [ ] normative example に未説明の skip が残らない
- [ ] 性能主張は推測値ではなく benchmark receipt で裏付けられる
- [ ] final audit で false-done が 0 件

## Phase 0 — 設計を実装可能な決定へ分割する

この phase ではコードを先行させない。RFC-011 の未決事項を解消し、
accepted decision と executable acceptance を用意する。

- [ ] Ownership / borrowing / mutability / Copy / Clone / Box / ADT semantics の successor ADR
- [ ] Wasm GC primary lowering + linear-memory tracing-GC fallback の backend ADR
- [ ] Closure capture / escape / lifetime の successor ADR
- [ ] Trait coherence / associated type / blanket impl / dyn dispatch の successor ADR
- [ ] Numeric conversion / overflow / NaN / division / shift / evaluation order の successor ADR
- [ ] Unicode / String / slice semantics の successor ADR
- [ ] module visibility / pub / Component export / import-use-WIT boundary の successor ADR
- [ ] grammar / desugaring / Prelude / keyword policy の successor ADR
- [ ] stability / normative spec / executable-example policy の必要な更新
- [ ] 各 decision に normal / diagnostic / cross-target fixture 計画を付ける

Phase 0 完了前に、その decision に依存する implementation child issue を close しない。

## Phase 1 — Ownership、mutability、値意味論

対象: RFC 項目 1–7, 10, 25。

- [ ] move-after-use を型検査する
- [ ] shared borrow と mutable borrow の排他規則を実装する
- [ ] `let` / `let mut` の mutability を binding 経由の操作として一貫させる
- [ ] mutability と `Copy` 可能性を独立に扱う
- [ ] user-defined struct/enum の `Copy` は明示 opt-in にする
- [ ] tuple / array / struct / enum / Option / Result を owned value semantics へ統一する
- [ ] `Vec` / `String` を「参照型」という source-level 分類に依存させない
- [ ] `Box<T>` を heap 上の `T` の unique owner として仕様・実装する
- [ ] explicit `clone` と implicit `Copy` の診断を実装する
- [ ] closure の borrow capture / move capture / escape を検査する
- [ ] slice を ownership/borrow と整合する view として実装する
- [ ] borrow/move/escape のエラー位置と fix suggestion を fixture 化する
- [ ] GC backend の heap representation が暗黙 alias semantics を復活させないことを検証する

## Phase 2 — Trait、error、generic API、Iterator

対象: RFC 項目 12–19, 26, 37。既存 #691, #693, #694, #695 と重複する作業は
新規に二重実装せず、child issue として関連付ける。

- [ ] `Self`、型引数付き Trait、associated type/function を確定・実装する
- [ ] coherence / orphan-equivalent rule / conflicting impl diagnostic を実装する
- [ ] blanket impl の解決順と termination rule を固定する
- [ ] static dispatch と `dyn Trait` の責務を分離する
- [ ] `From` / `Into` / `TryFrom` / `TryInto` の重複しない関係を実装する
- [ ] `?` の Option / Result / heterogeneous error conversion を Trait solver に統合する
- [ ] magic operator method を operator Trait へ移行する
- [ ] public free function の canonical method/Trait API への移行を完了する
- [ ] monomorphic helper を public API から外し generic API へ統一する
- [ ] generic parameter count に実装都合の仕様上限を設けない
- [ ] public `Any` の必要性を再評価し、必要な dynamic polymorphism は明示 Trait 境界にする
- [ ] compiler-internal Error sentinel を user type system から分離する
- [ ] `HashMap<K,V>` を generic Trait API として完結させる
- [ ] Iterator / IntoIterator / FromIterator / lazy adapter / collect を一つの体系にする

## Phase 3 — 数値、評価順、Unicode、String

対象: RFC 項目 20–24, 27。

- [ ] 異なる concrete numeric type 間の implicit promotion を原則禁止する
- [ ] lossless / checked / wrapping / saturating conversion を明示 API として区別する
- [ ] integer overflow の debug/release 差を作るか否かを仕様どおり実装する
- [ ] integer division by zero、signed overflow edge、shift out-of-range を規定する
- [ ] IEEE-754 NaN / inf / comparison / conversion の観測可能挙動を fixture 化する
- [ ] expression category ごとの evaluation order を normative spec と実装で一致させる
- [ ] function args / binary ops / aggregate init / indexing / assignment の副作用順序を fixture 化する
- [ ] String の encoding を固定し byte / Unicode scalar / grapheme API を混同しない
- [ ] `char` API が整数 sentinel を返す旧面を廃止または移行する
- [ ] invalid UTF / boundary / slicing behavior を規定・検証する
- [ ] effect system を導入しない限り let-generalization は syntactic non-expansive/value restriction で形式化する

## Phase 4 — Module、syntax、desugaring、namespace

対象: RFC 項目 11, 28–36, 38, 41。

- [ ] source visibility の `pub` と Component/WIT export を分離する
- [ ] source module import と WIT binding acquisition の canonical path を確定する
- [ ] `import` / `use` の役割を一意にする
- [ ] f-string / comprehension / iteration sugar を hygienic desugaring にする
- [ ] desugaring が shadow 可能な stdlib function name の通常 lookup に依存しない
- [ ] lang item / pre-resolved symbol / intrinsic 等の canonical mechanism を確定する
- [ ] Prelude は「最小」を目的化せず、小さく stable な固定集合として定義する
- [ ] `true` / `false` を lexical literal と Prelude binding の二重定義にしない
- [ ] multi-clause function grouping を declaration adjacency に依存させない
- [ ] multi-clause `where` の pattern-bound variable scope を正しくする
- [ ] newline / semicolon / continuation grammar を曖昧ケースまで規定する
- [ ] name mangling の具体形式を language spec から外し ABI/backend 資料へ移す
- [ ] hard reserved / contextual / future-reserved keyword を分類する
- [ ] future-reserved keyword を「未実装だから」という理由だけで解放しない
- [ ] parser / formatter / LSP / docs examples が新構文で一致する

## Phase 5 — Wasm GC primary backend と non-GC fallback

対象: RFC 項目 8, 9, 39, 40。既存 #686 / #801 / #728 / #729 と重なる場合は
それらを child work として再利用する。

### Wasm GC primary path

- [ ] managed struct / tuple / enum / Option / Result / closure env / collection representation を Wasm GC type へ lower する
- [ ] 対応可能な aggregate で `struct` / `array` / `ref` instructions/types を実際に emit する
- [ ] Wasm GC target が linear-memory object header/collector を不要にできる範囲を明記する
- [ ] emitted module を `wasm-tools validate` の GC feature で検証する
- [ ] primary target で GC path が silent fallback していないことを binary inspection fixture で保証する

### Linear-memory fallback

- [ ] Wasm GC feature 非対応 target 用 allocator を実装する
- [ ] tracing collector と root management を実装する
- [ ] cycle collection を含む managed graph semantics を実装する
- [ ] closure env / enum / Option / Result / shared graph を fallback representation へ lower する
- [ ] fallback module が Wasm GC proposal instructions を要求しないことを validate する
- [ ] GC availability の違いだけで source program を reject しない

### Cross-backend equivalence

- [ ] alias/move/borrow の source semantics が backend によって変わらない
- [ ] cyclic graph fixture が wasm32-gc / fallback の両方で同じ結果になる
- [ ] destructor/finalization 相当の観測可能仕様がある場合は両 backend で一致する
- [ ] closure capture / enum match / Option/Result / Vec/String / HashMap representative fixture が一致する
- [ ] target-specific error は host API / ABI / SIMD 等、本当に capability dependent なものだけに限定する

### Target configuration

- [ ] GC、memory width、WASI profile、Component emit を一つの target 名に詰め込まない設定モデルを確定する
- [ ] `wasm32-gc` と Memory64 の関係を明示設定へ整理する
- [ ] primary/default selection と fallback selection の CLI/manifest rule を fixture 化する

## Phase 6 — Stability、spec、executable documentation

対象: RFC 項目 42–45。

- [ ] Stable = compatibility guarantee として ADR-014 と spec を一致させる
- [ ] provisional / experimental / unimplemented の意味を一意にする
- [ ] redesign 中の current-state と future proposal を同じ normative section で混ぜない
- [ ] normative example は compile/run/check 可能な fixture に接続する
- [ ] `skip-doc-check` 相当は理由・owner・exit condition を要求する
- [ ] alias/copy/move/borrow/evaluation/numeric/Unicode/closure/Trait を normative semantics に集約する
- [ ] zero-from-scratch implementation に必要な observable behavior が ADR/RFC に散在したままになっていない
- [ ] docs consistency gate を CI に含める

## Phase 7 — Compatibility migration

- [ ] stable API ごとに deprecation window を決める
- [ ] old alias semantics から move/borrow semantics への compiler diagnostic/fix-it を提供する
- [ ] implicit numeric conversion の migration diagnostics を提供する
- [ ] old free function / magic method / import syntax の migration path を提供する
- [ ] old `Any` / sentinel / String API の migration path を必要に応じて提供する
- [ ] deprecated compatibility layer の removal condition を明示する
- [ ] sample projects / stdlib / compiler selfhost source を new semantics へ移行する

## Phase 8 — Performance and size validation

Wasm GC を「効率が良いはず」という理由だけで採択済み扱いにしない。
同じ workload と toolchain で primary/fallback を測る。

- [ ] representative micro + application workloads を固定する
- [ ] `.wasm` file size を記録する
- [ ] cold startup time を記録する
- [ ] steady-state runtime を記録する
- [ ] allocation throughput / collection pause or total GC time を記録する
- [ ] peak RSS / linear-memory high-water mark を記録する
- [ ] compile time / compiler RSS も backend 別に記録する
- [ ] Wasm GC が有利な workload と不利な workload を結果どおり記録する
- [ ] 根拠のない相対値や想定チャートを docs に入れない
- [ ] benchmark receipt に commit, runtime version, wasm-tools version, host, commands, raw result を保存する

## Master 45-item completion matrix

この matrix は close gate。Phase checklist を終えても、ここに未完了が 1 個でもあれば close しない。

- [ ] 01 暗黙の共有参照
- [ ] 02 `let mut` / mutability
- [ ] 03 struct / tuple value semantics
- [ ] 04 fixed array copy/move
- [ ] 05 `Option<T>` / `Result<T,E>`
- [ ] 06 enum value/backend representation
- [ ] 07 `Box<T>`
- [ ] 08 Wasm GC primary + fallback memory model
- [ ] 09 target capability boundary
- [ ] 10 closure capture / escape / lifetime
- [ ] 11 `pub` vs Component export
- [ ] 12 Trait core semantics
- [ ] 13 `?` / From conversion
- [ ] 14 operator magic method retirement
- [ ] 15 public free-function retirement
- [ ] 16 generic public API
- [ ] 17 generic parameter count rule
- [ ] 18 public `Any`
- [ ] 19 internal Error sentinel separation
- [ ] 20 integer/float implicit conversion
- [ ] 21 mixed numeric operation rules
- [ ] 22 overflow / NaN / division / shift semantics
- [ ] 23 evaluation order
- [ ] 24 String / Unicode semantics
- [ ] 25 slice semantics
- [ ] 26 `HashMap<K,V>`
- [ ] 27 let-generalization / value restriction
- [ ] 28 hygienic desugaring
- [ ] 29 f-string lowering
- [ ] 30 `import` / `use`
- [ ] 31 WIT import canonical path
- [ ] 32 Prelude policy
- [ ] 33 boolean literal duplication
- [ ] 34 multi-clause grouping
- [ ] 35 multi-clause `where`
- [ ] 36 newline / semicolon grammar
- [ ] 37 Iterator unification
- [ ] 38 name mangling specification boundary
- [ ] 39 target configuration decomposition
- [ ] 40 `wasm32-gc` / Memory64 separation
- [ ] 41 keyword reservation policy
- [ ] 42 stability label semantics
- [ ] 43 frozen/current spec vs redesign
- [ ] 44 executable normative examples
- [ ] 45 observable/operational semantics consolidation

## Required verification

各 child issue は変更面に応じた最小 gate を持つ。この umbrella の close 時には、
その時点で repository が定める canonical full gates を clean commit から実行し、
コマンド、exit code、PASS/FAIL/SKIP、artifact hash を receipt に保存する。

最低限、存在する限り次を含める。

```bash
python scripts/manager.py verify
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
python scripts/check/check-docs-consistency.py
python scripts/check/check-issue-headers.py
```

Wasm artifacts については対応する `wasm-tools validate` を実行し、
Wasm GC primary artifact と fallback artifact の両方を検証する。

## Child-issue discipline

1. 一つの child issue は一つの意味論/実装境界だけを扱う。
2. child issue は affected ADR/RFC item number を明記する。
3. child issue は before/after fixture と negative diagnostic fixture を持つ。
4. backend 変更は wasm32-gc と fallback の影響を明記する。
5. 仕様変更を伴う場合、docs/spec update を同じ work unit の acceptance に含める。
6. 一時 compatibility hack は owner、removal condition、tracking issue を必須にする。
7. child issue 完了後、この #852 の phase checklist と 45-item matrix を evidence 付きで更新する。

## STOP_IF

- Accepted ADR と実装案が衝突し successor decision がない
- source semantics を backend capability の都合で黙って変更する必要が出た
- wasm32-gc と fallback で通常プログラムの意味が分岐する
- fixture failure / SKIP 増加を「移行中」で正当化しようとしている
- stable API を migration plan なしで破壊する
- benchmark の raw evidence なしに性能改善を完了条件へ使う
- 一つの child issue が複数の独立 decision を混ぜ始める

STOP_IF に該当した場合、この umbrella を止める必要はない。
該当 child issue を blocked にし、独立 child issue は継続する。

## Final false-done checklist

- [ ] 45-item matrix 45/45
- [ ] Phase 0–8 の全 acceptance 完了
- [ ] 全 successor ADR が accepted または意図的に rejected/superseded され理由が残っている
- [ ] RFC-011 の「未決事項」が 0、または別 accepted decision へ明示移管されている
- [ ] wasm32-gc primary lowering が実 artifact で確認できる
- [ ] fallback backend が Wasm GC 非対応 feature set で validate/run できる
- [ ] cyclic/shared graph cross-backend fixture が PASS
- [ ] selfhost fixpoint と canonical parity gates が PASS
- [ ] normative docs examples gate が PASS
- [ ] migration/deprecation tracker に orphan entry がない
- [ ] benchmark receipt が再現可能
- [ ] generated issue index / dependency graph が最新
- [ ] clean commit から full verification receipt を取得
- [ ] close note に child issues、commits、ADRs、receipts を一覧化

## Close-note evidence schema

```text
umbrella: #852
final commit: <sha>
ADR-058 status: <status>
RFC-011 status: <status>

45-item matrix: 45/45
phase completion: 0✓ 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓

backend semantics:
  wasm32-gc: <PASS + artifact hash>
  fallback:  <PASS + artifact hash>
  cyclic/shared graph parity: <PASS>

canonical gates:
  verify: <rc/counts>
  fixpoint: <rc/hash>
  fixture parity: <PASS/FAIL/SKIP>
  diag parity: <PASS/FAIL/SKIP>
  docs consistency: <rc>

performance receipt:
  path: <path>
  wasm32-gc size/startup/runtime/RSS: <values>
  fallback size/startup/runtime/RSS: <values>

child issues:
  <id> <commit> <result>
  ...

remaining exceptions: none | <accepted ADR + tracking issue>
```

## Non-goals

- Rust と完全同一の表面構文・ABIにすること
- Wasm GC 非対応環境を理由に Wasm GC primary path を弱めること
- 全 backend で同じ内部 representation を強制すること
- ベンチマーク結果に関係なく「Wasm GC は必ず高速/小さい」と結論づけること
- 45 項目を一つの巨大 commit で実装すること
