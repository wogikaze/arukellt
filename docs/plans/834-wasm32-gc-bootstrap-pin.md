# #834 — Pin bootstrap to validating Memory64 wasm32-gc クローズ計画

ステータス: 完了（2026-07-26）  
親 issue: [#834](../../issues/done/834-wasm32-gc-bootstrap-pin.md)  
前駆 issue: [#730](../../issues/done/730-bootstrap-wasm-4gb-memory-limit.md)  
担当 subagent lane: `wave/834-bootstrap`  
作業 worktree: `.worktrees/wave-834-bootstrap`  
作成日: 2026-07-25  
更新日: 2026-07-26

## 1. 現状とゴール

- ピン済み bootstrap は `wasm32-gc` / `wasi-p2`（guest memory32）。`BOOTSTRAP_EMIT_*` も同設定。
- #730 での `func 8204` ref-cast-to-String 問題は `06ba2d35` で修正済み。
- ゴール達成: pin→s2→s3 fixpoint + `verify quick` 147/147。

## 2. 前提・依存

- #730 done。#813 bootstrap-only stage-3 workaround は本クローズで削除済み。
- 十分なホストメモリ（23GiB WSL では OOM 報告あり、32GiB+ 推奨）。

## 3. フェーズと完了条件

### Phase 1 — ホストメモリ制約確認
- 現在のホストで `python3 scripts/manager.py selfhost build-compiler` 後、`wasm32-gc` コンパイルを試す。
- OOM する場合は大容量ホストまたは CI 環境を確保。
- **2026-07-26:** 完了。default Memory64 runtime で emit OK（~1.3 GiB RSS）。OOM は本経路のブロッカーではない。証拠: `docs/research/834-wasm32-gc-bootstrap-probe.md`。

### Phase 2 — wasm32-gc self-emit 検証
- s2 ホストが `src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2` を `wasm-tools validate --features gc,function-references,memory64` で通す。
- **2026-07-26:** 完了。flat-src 経路で emit OK（MAX_RSS ≈ 1.23 GiB, ~100s）。
  誤った multi-dir が ~6 GiB hang に見えていた（hard OOM ではない）。
  `init.ark` の `fs_error_message(String)` 誤用を直し validate PASS（v7）。
  証拠: `docs/research/834-wasm32-gc-bootstrap-probe.md`。

### Phase 3 — ピン済み bootstrap 更新
- **完了:** `sha256(pin)==sha256(s2)==sha256(s3)` =
  `4d2da710115215965514608fe8f1d70cedabba35adf1c729abb0c0d2aa7539bd`
  （5 553 192 bytes, guest memory32）。GC pin に `--to-memory64` は当てない。

### Phase 4 — `BOOTSTRAP_EMIT_*` 更新
- **完了:** `BOOTSTRAP_EMIT_TARGET=wasm32-gc` / `BOOTSTRAP_EMIT_WASI_VERSION=wasi-p2`。

### Phase 5 — stage-3 ホスト復元
- **完了:** `_fixpoint_stage3_compiler()` → `_stage3_compiler_wasm`（s2-runtime）。

### Phase 6 — 最終検証
- **完了:** `python3 scripts/manager.py verify quick` → 147/147（0 fail）。

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