# #807 — Fixture parity: 367 remaining failures クローズ計画

ステータス: 進行中（L2 tranche）  
親 issue: [#807](../../issues/open/807-fixture-parity-367-remaining-failures.md)  
担当 subagent lane: `wave/807-fixture-parity`  
作業 worktree: `.worktrees/wave-807-fixture-parity`  
作成日: 2026-07-25  
更新: 2026-07-26

## 1. 現状とゴール

- レシート起点: `fixture_parity` fail=1089（issue 文面の 367 は古い）。
- 目標: 失敗数をゼロにし、`verify full receipt` を更新する。
- New-failure ratchet: 失敗数は減少のみ許容。

### L2 tranche（2026-07-26）

1. **Harness:** `--filter-dir` 追加。P2 stdio imports を host-linker 経由に（instantiate 失敗の大量解消）。
2. **ネスト lock:** `_ensure_current_selfhost` が flock を再取得しないよう修正。
3. **bool 型名:** `&&`/`||`/`!` の MIR local に `bool` を付与 → `.to_string()` が true/false。
4. **整数 widen:** 共有 `runtime.i8_to_i32` handler が callee 名を見て extend8/16 / zero-extend。

計測:
- コア4ディレクトリ: PASS=41 FAIL=0 SKIP=1
- 全 suite: **PASS=1029 FAIL=327 SKIP=259**（開始レシート fail=1089 から減少）

## 2. 前提・依存

- #287（fixture parity harness）done。
- `docs/data/verify-full-receipt.json` に失敗リストの正本あり。

## 3. フェーズと完了条件

### Phase 0 — 失敗リスト取得と分類
- `jq '.checks[] | select(.check_id == "fixture_parity") | .items[] | select(.result == "fail")'` で取得。
- 失敗タイプ別に分類:
  - `current wasm trap at runtime, pinned OK`
  - `current wasm invalid, pinned OK`
  - stdout 不一致など

### Phase 1 — ハーネス拡張（必要なら）
- `scripts/selfhost/checks.py` の `_load_manifest_fixtures` に `--filter-dir` オプションを追加し、特定ディレクトリだけの実行を可能にする。

### Phase 2 — 並列サブレーンでバグ修正
- 失敗 fixture をディレクトリ別にサブレーンに分割:
  - arrays, associated_fn, closure_capture
  - control, operators, match_extensions
  - functions, generics, generics_v1
  - enums, for_loops, from_trait, display_trait
  - collections, hashmap, option, result
  - stdlib_core, stdlib_hashmap, stdlib_hashset
  - stdlib_bytes, stdlib_csv, stdlib_env, stdlib_fs
  - stdlib_cli, stdlib_collections_compiler, stdlib_component
  - scalar, operators, question_mark, opt
  - selfhost, integration, host, examples, hello
- 各サブレーンは `wave/807-fixture-parity-<dir>` ブランチを使う。

### Phase 3 — 集約とレシート更新
- 全サブレーンを `wave/807-fixture-parity` に統合。
- `python3 scripts/manager.py verify full` で `verify-full-receipt.json` を再生成。

## 4. 作業レーン・並列可否

- 並列可能。ただし `selfhost fixture-parity` は `runtime_lock` で直列化される。
- 各サブレーンは異なる fixture ディレクトリを担当し、競合を避ける。

## 5. 検証コマンド

```bash
python3 scripts/manager.py selfhost fixture-parity
python3 scripts/manager.py verify full
python3 scripts/manager.py verify quick
```

## 6. リスク

- 新規失敗の追加は回帰とみなされる。
- 失敗原因が自己ホストコンパイラの深い lowering バグの場合、修正が大きくなる。
- `verify full` は時間がかかる。

## 7. 進捗更新規則

- 各ディレクトリ完了後に失敗数を記録し、親オーケストレータへ報告。
- `docs/data/verify-full-receipt.json` は最終統合時に一度だけ再生成する。