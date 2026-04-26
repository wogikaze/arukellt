# v3: 標準ライブラリ整備

> **Status**: **completed / historical roadmap**
> v3 stdlib track (`issues 039–059`) is complete and those issue files now live in `issues/done/`.
> For the current open queue, consult `issues/open/index.md`. For the current shipped behavior, consult `docs/current-state.md`.

---

## 1. この文書の位置づけ

このページは、v3 stdlib track で何を目標にしていたか・どう整理していたかを残す **historical roadmap** です。
現在の active queue を表す文書ではありません。

## 2. 結果サマリ

v3 では以下の整理が完了しました。

- stdlib module system (`use std::*`) の導入
- scalar completeness track の完了
- manifest-backed stdlib reference と module docs の整備
- prelude migration / deprecated API 移行方針の整備
- v3 stdlib fixture integration の完了
- stability docs / stdlib guide の整備

関連する tracked issues は `issues/done/039-*.md` 〜 `issues/done/059-*.md` に残っています。

## 3. 現在との関係

- この文書内の「進行中」「open issue」表現は **当時の計画状態** を記録したものです。
- 現在の open queue は stdlib track ではなく、WASI / `std::host` rollout を中心に進んでいます。
- stdlib surface の current snapshot は `docs/stdlib/README.md` と `docs/current-state.md` を見てください。

## 4. 当時の目的

v2 までに確立した言語基盤の上で、モノモーフ関数群 (`Vec_new_i32`, `map_i32_i32` 等) を体系化された標準ライブラリへ昇格させ、モジュール体系・命名規約・API 安定性ルールを整理することが v3 の目的でした。

## 5. 主要 work items（historical）

当時の主要トラック:

- module system infrastructure
- scalar type completeness
- `std::core`, `std::text`, `std::bytes`
- collections / seq / fs / io / time / random / process / env / cli
- `std::wasm`, `std::wit`, `std::component`
- prelude migration
- stability labels and reference docs
- fixture integration into `verify-harness.sh`

## 6. 関連する成果物

- `docs/stdlib/reference.md`
- `docs/stdlib/modules/`
- `docs/stdlib/prelude-migration.md`
- `docs/stdlib/stability-policy.md`
- `issues/done/057-prelude-migration.md`
- `issues/done/058-stability-docs.md`
- `issues/done/059-v3-fixture-harness-integration.md`

## 7. 検証の観点（historical completion notes）

v3 の完了にあたっては、次の種別の確認が組み込まれました。

- manifest-driven fixture execution
- stdlib manifest consistency checks
- generated stdlib reference / docs synchronization
- prelude migration / deprecated API guidance
- full repository verification via `scripts/manager.py`

現在の gate 数や fixture 件数は固定値ではなく、`docs/current-state.md` と実際の harness 出力を参照してください。

## 8. 読み替えルール

このページを読むときは、次のように扱ってください。

- **計画内容**として読む
- **現在の open work**としては読まない
- **現状の source of truth**としては使わない

current-first で判断したい場合は:

- open queue → `issues/open/index.md`
- current behavior → `docs/current-state.md`
- stdlib current surface → `docs/stdlib/README.md`
