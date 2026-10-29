# verify quick / lane baseline — 2026-10-29

Status: 調査メモ（決定記録ではない）。
対象: `wave/verify-quick`、base `3c2d00cef`（merge 917 ADR-088 callee-side param modes）。
環境: wasmtime 46.0.1、`ARUKELLT_CC=clang`。

このメモは merge/CI ゲートの赤状態を「修正したもの」と「意図的に残すもの」に
分類し、残すものへ owner と解除条件を付けるための記録である。allowlist では
ないため、どのゲートもここに書いただけでは黙らせられない。

## 前提: s2 を先に用意する

`verify quick` は起動時点で `.build/selfhost/arukellt-s2.wasm` がある場合だけ
heap-patched runtime を `ARUKELLT_SELFHOST_WASM` に設定する。無い場合、close
gate と doc example は `bootstrap/arukellt-selfhost.wasm`（pin）へフォールバック
する。pin は ADR-088 の `mut` parameter modes（#951）より前のため現行
`src/compiler/**` を parse できず、fixture に帰属された parse/typecheck error
（例: `raw_array_set_unchecked: expected 7, found 3`、E0100 visibility）を大量に
出す。これはコードの回帰ではなく環境要因である。

したがって fresh worktree では最初に 1 回だけ
`python3 scripts/manager.py selfhost build-compiler` を実行し、s2 を作ってから
quick を回す。pin → s2 の cold build は 20–40 分かかる（build 中の表示どおり）。

## ベースラインで確認した失敗と分類

| ゲート | 失敗 | 分類 | 処置 |
|---|---|---|---|
| `scripts/tests/test_gate.py` | fixture-floor / overlapping-outputs の 2 件が `run:` と `test:` を分離しない mock で失敗。gate-676 が削除済み `std/host/fs/write.ark` を参照 | test drift | 修正済み |
| `scripts/check/gate-676-std-host-fs-env-process.py` | `std/host/fs/write.ark` 不在、`pub fn read_to_string(` 等の旧 free-function marker | stale marker（ADR-046 で `Fs` method へ移行済み） | 修正済み |
| `selfhost fmt-parity` | `delimiters_in_strings` / `imports_interleaved` / `imports_unsorted` の 3 件。`.expected` が過去の codemod 前の内容（`v::Vec` alias、free `concat`、`use std::text` 欠落） | stale golden | 現行 formatter 出力で再生成 |
| `check-ark-lint-smoke.py` | `docs/examples/hello.ark` が W0011 を 3 件出力（std の nested else-if が entry file に帰属） | attribution bug | post-resolve lint を main source decls に限定 |
| `verify quick` manifest completeness | evil merge `81d095fda` が `removed_path/time/random` の diag 6 件を脱落。`removed_io_compat_free_function` は未登録。`modules/bare_use_root_fallback/sub/*.ark` は package 内 support module なのに disk scan が entry と誤認 | merge drift / checker gap | manifest 復元・登録、package-aware scan に修正 |
| CQ-STRUCT-007 | `docs/history/benchmarks/benchmark-{686,879,880}-*.md` 6 件が banner を持つが `.generated-files` 未登録 | generated boundary 未宣言 | `.generated-files` に generator/再生成コマンド付きで登録 |
| `check-ark-code-quality.py` | `lines_ge_200` が 10 > baseline 2 | 実 debt | 未修正（下記） |
| `quality structure` CQ-STRUCT-002 | `src/compiler/mir/lower/*` と `src/compiler/wasm/*` の import cycle 9 件 | 実 debt | 未修正 |
| `quality structure` CQ-STRUCT-005 | subsystem child module の `pub fn` 17 件 / forbidden `use mir::lower` | 実 debt | 未修正 |
| `quality structure` CQ-STRUCT-008 | `src/compiler/corehir/decl_impl_target.ark:11: use parser::kinds` | 実 debt | 未修正 |
| `check-issue-evidence-rules.py` | baseline 外の evidence 5 件以上 | issue hygiene | 未修正 |
| `check-false-done-hygiene.py` | #932 が Status: open のまま `issues/done/` にある | issue hygiene | 未修正 |
| `issues/done/` unchecked | #932 / #973 / #975 に未チェック項目 | issue hygiene | 未修正 |
| `gate-765-docs-ci-hard-gates.py` | `docs/compiler/error-codes.md` の `skip-doc-check` 9 件が budget 3 超過かつ owner/expires 欠落 | docs contract | 未修正 |
| `check-documentation-publication-contract.py` | `docs/stdlib/modules/process.md` に `Target/profile availability: mixed` が無い | docs contract | 未修正 |
| `check`（単体 compiler source） | `src/compiler/main.ark` を直接 check すると E0100 visibility 7 件（#931） | 既知 residual | 未修正（overlay build では発生しない） |

`s3`/pin 不在時に出る close-gate / doc-example / CLI smoke の失敗は、s2 を用意
すると解消することを確認した（`check-docs`、`check-cli-guarantees all`、
close gate の実行済み分 #639/#652/#673/#679/#443）。

## 修正後の確認

- `python3 -m pytest scripts/tests/test_gate.py -q` → 22 passed。
- `python3 scripts/check/gate-676-std-host-fs-env-process.py` → PASS。
- `bash scripts/check/check-generated-files.sh` → PASS。
- `check-docs-consistency.py` / `check-docs-freshness.py` → OK。
- fmt 3 件: 現行 s2 で fmt → `.expected` 一致 → 再 fmt で不変 → check 0 を確認。
- `python3 scripts/manager.py verify lane` → PASS（clean-HEAD の s2 を
  `ARUKELLT_SELFHOST_WASM` に指定）。
- `python3 scripts/manager.py verify quick`（s2 不在のベースライン）→
  157 checks / 145 pass / 12 fail。
- fresh s2 のビルドはこのホストで完走できなかった: pin → s2 は 40 分の文書化
  上限を超えて停止し worktree の runtime lock を保持したままになった。clean-HEAD
  の s2 から現行 source をコンパイルする近道は、un-repinned s2 の quadratic
  producer hang（`check-fresh-s2-smoke.py` のコメント参照）で 15 分 timeout。
  このため lint scoping 修正は format / `lint --local` / 単体 check（既知の
  E0100 7 件のみ）まで確認し、修正後 binary での smoke 再実行は未達。

## 残す失敗の owner と解除条件

- CQ-STRUCT-002 / 005 / 008（owner: compiler）: ADR-044/046 の method / facade
  移行と cycle 解消。直近の追加分は #951（ADR-088）と #917 の lane。解除条件は
  `python3 scripts/manager.py quality structure` の該当行が消えること。
- `lines_ge_200`（owner: compiler）: 該当 10 行を 120 字原則に合わせて分割するか、
  ratchet を上げる場合は open issue と理由を伴う別変更。baseline の黙った更新は
  しない。
- issue hygiene（owner: orchestration）: `$issue-close-review` を通して
  #932 / #973 / #975 の checkbox と Status を整合させる。
- docs contract（owner: docs）: `docs/compiler/error-codes.md` の skip marker に
  owner/expires を付けるか、budget を policy 変更として扱う。process.md の
  availability axis は generator 入力（`std/manifest.toml`）側で揃える。
- E0100 visibility（owner: resolver, #931）: package 内 module の可視性解決。
  overlay build が正規経路のため quick の赤には影響しない。

## 再現

```bash
export PATH=/home/wogikaze/.local/share/mise/installs/wasmtime/46.0.1/wasmtime-v46.0.1-x86_64-linux:$PATH
export ARUKELLT_CC=clang
python3 scripts/manager.py selfhost build-compiler   # fresh worktree で 1 回
python3 scripts/manager.py verify quick
python3 scripts/manager.py verify lane
```

## 追記: CQ-STRUCT-002 / 005 / 008 の解消（wave/cqstruct）

上の分類のうち、owner: compiler の 3 ルールを `wave/cqstruct` で修正した。
`python3 scripts/manager.py quality structure` は 27 findings（9 cycle +
18 boundary）から `errors=0 warnings=0 advisories=0` になった。

- CQ-STRUCT-002: closure 系 lower module から emit hook / env-cell / layout /
  capture-type を下位 module（`ctx_emit_core`、`ctx_env_cells`、
  `closure_env_layout`、`closure_capture_type`）へ分離し、`closure_pcells` は
  rewrite hook を持たない emit core を使う。mono instance index は
  `ctx_mono_index` へ、container element 解決は `intrinsic_vec_type` へ移した。
- CQ-STRUCT-005: `emit_spec` を `mir` 境界へ、`wat_disasm` を `driver/` へ移し、
  child module の 14 個の `pub fn` を内部可視へ戻した。`strip_struct_type_prefix`
  は `ctx_struct_layout` に集約し、`assign.ark` / `aggregate_field.ark` の重複と
  未使用 copy を削除した。
- CQ-STRUCT-008: `decl_impl_target.ark` は `parser::kinds` ではなく
  `corehir::frontend_kind_map::nk_impl_generics()` を参照する。

確認結果（同一 worktree、`ARUKELLT_CC=clang`）:

- `quality structure`: `errors=0 warnings=0 advisories=0`
- `verify fixtures`: 1889 compile+validate PASS / 0 FAIL、runtime smoke 109、
  test 36 PASS
- `verify lane`: PASS（W0011 ratchet 含む）
- `docs check`: 6/6 PASS
- `selfhost fixpoint`: PASS、sha256(s2)==sha256(s3)=
  `74c8d1c0e4676f8fd097f9082a01a4f9df59c9bdd627c7b968933d38337938c4`

未解消として残るのは、このメモの範囲外である `lines_ge_200`（10 > baseline 2）と
direct `check src/compiler/main.ark` の E0100 7 件（#931）である。

## 追記: wave/verify-quick2 再ベースライン（2026-10-29、base `f1e5c736c`）

fresh worktree `.worktrees/wave-verify-quick2`、`ARUKELLT_BUILD_DIR=.build-lane`、
fresh s2（初回 59.2s、`.ark` 修正後 92.8s）で再測定した。

修正して PASS にしたもの:

- issue hygiene: #932 を `Status: done` にし、2026-10-19 の receipt evidence
  （guest samples 6178 → 2248、s2 == s3、validate PASS、plan/receipt 更新）を
  本文へ追加。#973 は retire した claim、#975 は #953 の duplicate として、
  acceptance を実際の結末（fixture を追加していない箇所は「harness に stdin
  sidecar がない」）で checked にした。done/ の unchecked は 0。
- issue evidence rules: #814/#825/#927/#929/#942 に実行済みの evidence を追加。
  `check-issue-evidence-rules.py` PASS（0 new、813 grandfathered）。
- docs CI hard gates: `gate-765` が permanent notation
  （`grammar-fragment` / `invalid-example` / `notation`）を deferral budget から
  除外し、`check-doc-examples.py` の #877 contract と揃えた。error-codes.md は
  invalid-example 8 件 + deferral 3 件で budget 3 に収まる。
- publication contract: process page の generator input に
  `availability_axis = "mixed"` を追加（`Env::var` が `wasm32` で未対応のため）。
  `docs/stdlib/generation-schema.md` に使用条件を記録。
- compiler constant-like functions: `emit_rung_census.ark` の `RUNG_*` と
  `helpers_io_p2.ark` の `P2_*` は、subsystem の table と唯一の consumer が
  同居している実 table。manager に prefix 限定・理由付きの例外として登録した
  （他 file の定数は隠さない）。
- CQ-STRUCT-005: `corehir/name_match.ark` を `NameMatch` namespace type へ、
  `mir/lower/signature_registry_build_mono.ark` の leaf index helpers を
  `SigRegLeafIndex` の associated functions へ移した。`quality structure` は
  27 findings から `errors=0 warnings=0 advisories=0`。
- test_gate / gate-676: `Process::id` の method surface に追従。22 passed。
- fmt-parity: `imports_interleaved.expected` は #917 の source import 削除に
  未追従だったため現行 formatter 出力へ再生成（再 fmt で不変を確認）。9/9 PASS。
- T3: fresh s2 で 596 pass / 0 fail。T3 gate の cache key が fixture ごとに
  compiler 12 MB hash と `--version` 2 回を実行していたのを 1 回にした
  （cold cache 23m37s の主要因）。
- close gates: #472/#500 の npm install は sandbox が `~/.npm` に書けない
  環境要因だった。`_playground_env` が `NPM_CONFIG_CACHE` を
  `.build/npm-cache` に既定化し、`--no-cache` の 25 enforced が PASS。

確認結果（同 worktree、fresh s2）:

- `python3 scripts/manager.py verify quick`: 157 checks / 157 pass。
- `python3 scripts/manager.py verify lane`: PASS。
- `python3 scripts/manager.py docs check`: 6/6 PASS。
- `python3 scripts/manager.py quality structure`: PASS（0/0/0）。
- `check-ark-code-quality.py`: `lines_ge_200: 0`（baseline は 2 のまま）。

quick の外に残るもの:

- direct `check src/compiler/main.ark` の E0100 7 件（#931）。overlay build の
  正規経路では発生しない。
- process page の module badge は `Env::var` の `wasm32` 欠落をまだ単一 axis で
  表示する。page-level は mixed、per-symbol table が正本。types/methods の
  availability を generator が扱うのは別作業。
- `lines_ge_200` の baseline 値 2 は実測 0 に対する余裕として残る（gate PASS）。
