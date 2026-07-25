# #834 — Pin bootstrap to validating Memory64 wasm32-gc クローズ計画

ステータス: 計画  
親 issue: [#834](../../issues/open/834-wasm32-gc-bootstrap-pin.md)  
前駆 issue: [#730](../../issues/done/730-bootstrap-wasm-4gb-memory-limit.md)  
担当 subagent lane: `wave/834-bootstrap`  
作業 worktree: `.worktrees/wave-834-bootstrap`  
作成日: 2026-07-25

## 1. 現状とゴール

- 現在のピン済み bootstrap は `wasm32` / `wasi-p1`。
- 目標: Memory64 `wasm32-gc` / `wasi-p2` の selfhost コンパイラを生成し、ピンして `BOOTSTRAP_EMIT_*` を更新、`verify quick` を 0 失敗で通す。
- #730 での `func 8204` ref-cast-to-String 問題は `06ba2d35` で修正済み。

## 2. 前提・依存

- #730 done、#813 workaround あり。
- 十分なホストメモリ（23GiB WSL では OOM 報告あり、32GiB+ 推奨）。

## 3. フェーズと完了条件

### Phase 1 — ホストメモリ制約確認
- 現在のホストで `python3 scripts/manager.py selfhost build-compiler` 後、`wasm32-gc` コンパイルを試す。
- OOM する場合は大容量ホストまたは CI 環境を確保。

### Phase 2 — wasm32-gc self-emit 検証
- s2 ホストが `src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2` を `wasm-tools validate --features gc,function-references,memory64` で通す。

### Phase 3 — ピン済み bootstrap 更新
- `python3 scripts/manager.py selfhost fixpoint --build` を実行し `sha256(s2)==sha256(s3)` を安定化。
- `bootstrap/arukellt-selfhost.wasm` を新しい s2-runtime と置換。
- `bootstrap/PROVENANCE.md` を更新（sha256, size, target Memory64 wasm32-gc / wasi-p2）。

### Phase 4 — `BOOTSTRAP_EMIT_*` 更新
- `scripts/selfhost/checks.py` の `BOOTSTRAP_EMIT_TARGET` を `wasm32-gc`、`BOOTSTRAP_EMIT_WASI_VERSION` を `wasi-p2` に変更。

### Phase 5 — stage-3 ホスト復元
- `_fixpoint_stage3_compiler()` から #813 bootstrap-only workaround を削除し、s2-runtime を返すようにする。

### Phase 6 — 最終検証
- `python3 scripts/manager.py verify quick` が 0 失敗。

## 4. 作業レーン・並列可否

- 独立レーン。ただし `selfhost fixpoint --build` は `runtime_lock` で直列化される。
- 他の selfhost ビルドレーンと同時に走らせるとロック待ちが発生するため、親オーケストレータが `build-compiler` / `fixpoint` を集中管理する。

## 5. 検証コマンド

```bash
wasm-tools validate --features gc,function-references,memory64 .build/selfhost/arukellt-s2.wasm
python3 scripts/manager.py selfhost fixpoint --build
sha256sum .build/selfhost/arukellt-s2.wasm .build/selfhost/arukellt-s3.wasm
python3 scripts/manager.py verify quick
```

## 6. リスク

- ホストメモリ不足で OOM。
- wasm32-gc emit に新たな検証問題が残っている可能性。
- fixpoint が複数ラウンド必要になる可能性。

## 7. 進捗更新規則

- 各 Phase 完了後に親オーケストレータへ報告。
- `bootstrap/PROVENANCE.md` と `scripts/selfhost/checks.py` の変更は同じ commit にまとめる。