# 並列 issue close wave オーケストレーション計画

ステータス: 計画  
作成日: 2026-07-25  
対象 issue: #834, #807, #822, #727, #824, #826, #810, #809, #696, #704, #705

## 1. 目的

依存が解消済みの open issue 11 件を subagent レーンで並列に調査・実装し、`python3 scripts/manager.py verify quick` が通る状態で `master` に段階的にマージする。

## 2. レーン一覧

- L1 `#834` — Memory64 wasm32-gc bootstrap pin (`docs/plans/834-wasm32-gc-bootstrap-pin.md`)
- L2 `#807` — Fixture parity 367 failures (`docs/plans/807-fixture-parity-remaining-failures.md`)
- L3 `#822` — Representation-dependent stdlib migration (`docs/plans/822-representation-dependent-stdlib-migration.md`)
- L4 `#727` — arukellt_host bridge retirement (`docs/plans/727-host-bridge-retirement.md`)
- L5 `#824` — Early body lowering (`docs/plans/824-early-body-lowering.md`)
- L6 `#826` — Symbol/path interning + clone audit (`docs/plans/826-symbol-path-intern-clone-audit.md`)
- L7 `#810` — Component interop failures (`docs/plans/810-component-interop-failures.md`)
- L8 `#809` — WAT roundtrip failure (`docs/plans/809-wat-roundtrip-failure.md`)
- L9 `#696` — Debug trait / format macros (`docs/plans/696-debug-trait-format-macros.md`)
- L10 `#704` — std::json full compliance (`docs/plans/704-std-json-full-compliance.md`)
- L11 `#705` — std::toml full compliance (`docs/plans/705-std-toml-full-compliance.md`)

## 3. 共通作業手順

各 subagent は以下を守る。

```bash
# 1. worktree 作成（名前は各計画ファイルに記載）
git worktree add .worktrees/wave-<id>-<slug> -b wave/<id>-<slug>
cd .worktrees/wave-<id>-<slug>

# 2. ビルドアーティファクト分離
export ARUKELLT_BUILD_DIR="$PWD/.build"

# 3. 実装
# ...

# 4. レーン gate（各計画ファイルの検証コマンドを実行）
python3 scripts/manager.py verify lane
# または issue 固有 gate

# 5. commit（.worktrees/wave-<id>-<slug> 内で）
# 6. 親オーケストレータがマージ
```

## 4. 前提タスク P0

- `#820` の `scripts/tests/test_stdlib_inline.py` WAT export 正規表現修正を先に完了する。
- L3 `#822` は P0 がないと `test_stdlib_inline` の新規 probe 追加と `verify quick` が不安定。

## 5. マージ順序

並列実装後、原則として独立なレーンから順次マージする。衝突しやすい順を推奨する。

1. P0 `#820` 修正（前提）
2. L8 `#809`、L9 `#696`、L10 `#704`、L11 `#705` — stdlib / parser 系を先行させ、std 表面の変更を早期に確定
3. L6 `#826`、L5 `#824` — コンパイラ内部最適化（他レーンとの競合が少ない）
4. L3 `#822` — CoreOp / stdlib 移行（P0 後）
5. L4 `#727` — host-linker / Rust（影響範囲が別）
6. L7 `#810` — component model（大規模、最後にまとめて）
7. L2 `#807` — fixture parity は継続的に減らし、好機にマージ
8. L1 `#834` — bootstrap pin は大容量メモリ環境で実施し、最後または独立 CI でマージ

## 6. マージ毎の検証

```bash
# 各マージ後
python3 scripts/check/check-t3-wasm-validate.py
python3 scripts/manager.py verify lane --gate t3

# 最終
python3 scripts/manager.py verify quick
```

## 7. リスク

- `#834` は 23GiB WSL では OOM する可能性がある。32GiB 以上のホストまたは CI で実行。
- `#807` と `#810` はスコープが大きく、複数サブレーンに分割しても数週間かかる可能性がある。
- `#822` のジェネリック Vec 変異は fallback resolver 拡張が別途必要なため、今回の wave では段階的完了とする。
- `#727` と `#822` は `data/core-ops.toml` / `std::host::*` で重複する可能性がある。マージは親オーケストレータが衝突確認後に行う。

## 8. 進捗更新規則

- 各 subagent は作業開始・フェーズ完了時に親オーケストレータへ通知する。
- issue 状態の変更（open → done）は親オーケストレータが `verify quick` 通過後に統一して行う。
- `docs/current-state.md` と `docs/data/project-state.toml` は最終マージ後に `python3 scripts/gen/generate-docs.py` で再生成する。