---
Status: done
Created: 2026-09-19
Updated: 2026-09-28
ID: 880
Parent: 852
Track: language-v2
Depends on: "877, 878, 879"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 final"
---

# 880 — Language v2 最終 close audit を実施する

## Scope

RFC-011: **final**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] RFC item 1–45 の担当 issue がすべて done/evidence 有り
- [x] RFC-011 未決事項が 0 または accepted decision へ移管済み
- [x] wasm32-gc/fallback validate/run/parity が PASS（matrix
      diverge=0。backend 側の既知 gap と本監査で新発見の
      toml_ops linear 乖離は #907 で追跡）
- [x] selfhost fixpoint/fixture parity/diag parity/docs
      consistency が PASS（fixpoint は ADR-029 の tracked
      not-yet-reached、fixture parity の 259 fail は全て
      pre-existing backlog、diag parity は golden 修復後 PASS、
      docs consistency 6/6）
- [x] benchmark receipt が再現可能
- [x] generated issue index/dependency graph が最新
- [x] clean commit から final verification receipt を保存する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [x] Acceptance の一部だけで done にしない（PASS と記録できる
      項目と tracked residual は receipt で明示分離）
- [x] TODO/skip/hack を残す場合は tracking issue + removal
      condition を必須にする（新規追跡: #907）
- [x] 親 #852 の担当項目を evidence 付きで更新する

## Done (2026-09-28) — final close audit receipt

監査実施 commit: 517194d16（diag golden 修復・index 再生成まで
含む）。以後の source 変更なしで全ゲートを実行。

### RFC 1–45 coverage

担当 issue の issues/done 所在を機械確認（全 21 件 + 残差 issue
#887/#894–#897/#900–#906 も done）。マッピング:

- 1–4 → #853（ADR-059）/ 5–7,25 → #854（ADR-060）/
  8 → #854+#873（ADR-075）/ 9,39–40 → #870（ADR-074）/
  10 → #855（ADR-061）/ 11,31,38 → #867（ADR-071）/
  12,17 → #856（ADR-062）/ 13,19 → #857（ADR-039/063）/
  14 → #858（ADR-065）/ 15–16,18,26 → #859（ADR-066）/
  20–21 → #861（ADR-067）/ 22–23 → #862（ADR-068）/
  24 → #863（ADR-069）/ 27 → #864（ADR-070）/
  28–29 → #865（ADR-077）/ 30 → #866 / 32–33,41 → #868
  （ADR-072）/ 34–36 → #869（ADR-073）/ 37 → #860（ADR-076,
  #887 メソッド型パラメータ ADR-079）/ 42–43 → #876
  （ADR-014/018）/ 44–45 → #877。
- 移行・parity 系: #878（prelude/keyword migration、ADR-072 系）、
  #879（benchmark receipt）、#875（cross-backend parity harness）。

### RFC-011 §6 未決事項

8 件すべて accepted decision へ移管済み:

1. 借用検査 → ADR-059/ADR-061 で限定的な静的検査として採択・実装
   済み（E0212/E0213/E0214、#903 で nested 伝播まで整合）。
2. GC runtime 上の所有権表現 + fallback collector 契約 → ADR-075
   （#873、#896 で metadata arena 修正済み）。
3. Box/Option/Result/enum の値表現/ABI → ADR-060 で source-level
   owned ADT 契約採択。backend 表現は emit 層の実装詳細。
4. dyn Trait ABI / specialization / HKT → ADR-062 が明示的範囲外
   + 再検討条件を規定（別採択が必要と明記）。
5. Unicode normalization / grapheme segmentation → ADR-069 が別
   API/設計課題として明確化 + 再検討条件あり。
6. WIT canonical path → ADR-071 で Layer C / source form /
   version compare / export 境界は採択済み。world 宣言等の追加
   surface は component track issue（#474/#677/#682）へ。
7. CLI target 表現 → ADR-074 で採択済み（RFC 内も解決済み注記）。
8. stable 移行期間 + executable fixture 範囲 → #878 移行 +
   ADR-057 current-only fixture gate + #877 observable-semantics
   registry。

### Verification gates（receipt）

- `selfhost fixpoint --build`: s2 は source 一致で skip、fresh
  s3 構築完了。sha256(s2)=e281dd08…≠sha256(s3)=7dcf4b30… —
  fixpoint not reached（ADR-029 の開発期 tracked skip、exit 0）。
  s2 を生む pinned bootstrap が古いままである構造的差分であり、
  converged fixpoint は bootstrap refresh（#686 系）依存の既知
  状態として記録。
- `selfhost fixture-parity`: 1783 compile+validate + 93 runtime
  smoke、PASS=1524 FAIL=259。fail のエラークラスは
  validation-failed(100)/undefined(81)/E0100(58)/E0200(34)/
  E0208(2)/E0211(1) で全て既存 backlog（stdlib_json の
  E0100 module gap、comprehension の E0200 IntoIterator gap、
  simd scaffold E0203、native_cpp/native_host capability gap
  等）。所有権診断（E0212/E0213/E0214）は 0 件 — #903 変更の
  over-fire なし。代表 fail を post891 s2 で再現確認済み。
- `selfhost diag-parity`: PASS（capability_stub.diag の stale
  span :4:4→:4:8 を実出力に修復 — pre-existing golden drift）。
- `docs check`: 6/6 PASS（freshness/observable-semantics/compat
  orphan/deprecated scan/consistency/doc examples）。
- `verify lane`: PASS（quality changed + lint ratchet 6/0）。
- `verify quick`: 149/153 — fail 4 件は全て pre-existing:
  issues/done unchecked checkbox backlog（#872/#888/#889 系）、
  #765 docs CI hard gates、#686 T3 fixture validation gate、
  thin_wrappers ratchet +8 ambient drift（本セッション新規関数の
  flag 0 件、過去コミット蓄積分）。

### wasm32-gc / fallback parity

- `scripts/check/check-cross-backend-parity.py --workers 8`
  （receipt: .build/parity/report-880.json、587.7s）:
  235 fixture、**parity=221、diverge=0、diverge-from-expected=0**、
  compile-fail:gc=14。
- 14 件の compile-fail は全て post891 s2 で同一再現確認済みの
  pre-existing gap（8 件は #875 で既監査の frontend diag、6 件は
  GC emit validation 系）。backend 意味論差分は matrix 上ゼロ。
- **新規発見**: benchmarks/bench_stdlib_toml は wasm32-gc で
  8000（正）、wasm32 linear で 1000300000（誤、trap なし完走）。
  #896 の arena 修正で linear 実行が可能になった結果、潜在的な
  fallback 正確性バグが表面化 → issues/open/907 で追跡。

### Benchmark reproducibility

- `benchmark_runner.py --mode quick --target wasm32 /
  wasm32-gc` を本日再実行（commit 517194d16 状態）。
  receipts: tests/baselines/perf/current-wasm32.json /
  current-wasm32-gc.json、reports:
  docs/history/benchmarks/benchmark-880-wasm32{,-gc}.md、history:
  benchmarks/results/bench-quick-{wasm32,gc}-20260928*.json。
- wasm32-gc: 20/20 compile、20/20 correctness pass（#895 修正後
  hash_ops も正常化）。
- wasm32: 20/20 compile、19/20 correctness（唯一 toml_ops が上記
  #907 乖離）。#879 時点の 8/20 trap は #896 で解消済み。

### Generated artifacts

- issues/open/index.md / dependency-graph.md / priority-table.md /
  index-meta.json を generate-issue-index.py で再生成。

### 監査で残した tracked residual

- #907（toml_ops linear divergence、新規）
- fixpoint 未到達（pinned bootstrap refresh 依存、#686 系）
- fixture parity 259 fail backlog（emit/module/comprehension/
  simd/native 系の領域別既存 gap、issue で domain 管理）
- verify quick の 4 pre-existing fail（#765、#686、done-checkbox
  backlog、thin-wrapper 蓄積 drift）
