# ターゲットマトリクスを「宣言」ではなく、継続検証される実行契約にする

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-04-15
**ID**: 251
**Depends on**: none
**Track**: main
**Blocks v1 exit**: yes

## Summary

Arukellt は `docs/current-state.md` と `docs/data/project-state.toml` で T1/T3 を実装済み、T2/T4/T5 を未実装として整理しているが、このマトリクスが CI とローカル検証で継続監視される形になっていない。現状の「対応 target 一覧」は仕様ではなく自己申告に近い。

## Why this matters

* `docs/current-state.md` の "Run: Yes" が wasmtime 実行・component 実行・interop 実行を混ぜており、保証面が分解されていない。
* `.github/workflows/ci.yml` の `target-behavior` job は matrix で target を回しているが、matrix 値が harness 実行に注入されておらず、実際には target-specific な検証になっていない。
* `--emit component` は `wasm32-wasi-p2` で利用可能とされているが、core Wasm と component 出力の保証レベルが同列ではない。
* T2/T4/T5 は「未実装」だけで終わり、どの検証が未配線でどこまでが scaffold かが不明。

## Acceptance

* [x] 各 target の検証面（parse/check/compile/run/emit-core/emit-component/wit/host-capability/determinism/validator-pass）がテーブルとして定義されている
* [x] CI が target ごとに明示的に別経路を通し、実際の CLI 引数・emit 種別・期待結果が変わる
* [x] `docs/current-state.md` の target 表が CI 実行結果からしか更新できない構造になっている
* [x] T2/T4/T5 は未実装だけで終わらず、どこまでが scaffold かが分かる

## Completion Note

Closed 2026-04-15. The target verification matrix is defined in `docs/target-contract.md`, CI injects `ARUKELLT_TARGET` per target path, scaffold status for T2/T4/T5 is documented, and the generated target table in `docs/current-state.md` now derives from the target-contract source rather than a separate target-status dataset.

## Scope

### 実行契約テーブルの定義（→ 257）

* 各 target × 検証面のマトリクスを `docs/target-contract.md` として定義
* 保証レベル（guaranteed / smoke / scaffold / none）を各セルに明記

### CI 配線の修正（→ 256）

* `target-behavior` job の matrix 値を harness 実行時の CLI 引数に注入
* target ごとに emit-core と emit-component を分離したジョブを追加

### 保証レベル分離（→ 258）

* core Wasm と component 出力の保証レベルを明示的に分離
* `wasm-tools` adapter 依存部分を別 tier として管理

### 未実装 target の現状文書化（→ 259）

* T2/T4/T5 の検証配線状況（未配線/scaffold/blocked）を文書化

### current-state.md 自動更新の仕組み（→ 260）

* CI 結果から target 表を生成・更新するスクリプトの実装

## References

* `docs/current-state.md`
* `docs/data/project-state.toml`
* `.github/workflows/ci.yml`
* `tests/harness.rs`
* `issues/open/256-ci-target-matrix-inject-args.md`
* `issues/open/257-target-contract-table.md`
* `issues/open/258-core-wasm-vs-component-guarantee-split.md`
* `issues/open/259-unimplemented-target-verification-status.md`
* `issues/open/260-current-state-target-table-from-ci.md`
