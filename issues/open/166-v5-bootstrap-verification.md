# 166: ブートストラップ検証スクリプト

**Version**: v5
**Priority**: P1
**Depends on**: #165 (Wasm Emitter)

## 概要

Stage 0 → Stage 1 → Stage 2 のブートストラップを自動検証するスクリプトを作成する。

## タスク

1. `scripts/verify-bootstrap.sh` の作成:

   ```bash
   #!/bin/bash
   set -euo pipefail
   
   # Stage 0: Rust 版でコンパイル
   cargo build --release -p arukellt
   target/release/arukellt compile src/compiler/*.ark -o arukellt-s1.wasm
   
   # Stage 1 で fixture テスト
   for f in tests/fixtures/**/*.ark; do
     wasmtime run arukellt-s1.wasm -- compile "$f" -o /tmp/test.wasm
     # validate output
   done
   
   # Stage 1 → Stage 2
   wasmtime run arukellt-s1.wasm -- compile src/compiler/*.ark -o arukellt-s2.wasm
   
   # fixpoint 検証
   sha256sum arukellt-s1.wasm arukellt-s2.wasm
   diff <(sha256sum arukellt-s1.wasm | cut -d' ' -f1) <(sha256sum arukellt-s2.wasm | cut -d' ' -f1)
   echo "BOOTSTRAP FIXPOINT VERIFIED"
   ```

2. `scripts/compare-outputs.sh`: Rust 版と Arukellt 版の phase 出力比較
3. CI 連携: verify-bootstrap.sh を verify-harness.sh に統合 (条件付き実行)

## 完了条件

- `scripts/verify-bootstrap.sh` が exit 0 で完了する
- fixpoint (sha256 一致) が確認される
- `docs/compiler/bootstrap.md` にブートストラップ手順が記載されている
