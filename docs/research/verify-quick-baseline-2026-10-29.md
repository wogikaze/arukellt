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
