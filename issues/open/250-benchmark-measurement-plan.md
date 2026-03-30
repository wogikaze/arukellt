# benchmark measurement plan: GC vs linear memory decision data

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-30
**ID**: 250
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: no

## Summary

ADR-002（GC vs non-GC メモリモデル選択）の決定に必要なベンチマークデータがまだ収集されていない。
6 つのベンチマークケース（Hello World, 文字列連結, Vec push/pop, 二分木, Result エラー処理, WASI I/O）を実装・計測し、
GC 版と linear memory 版を比較する。

元ドキュメント: `docs/process/benchmark-plan.md`（issue 化により移動済み）

## Acceptance

- [ ] 6 ケース全てのベンチマーク実装が `benchmarks/` に存在する
- [ ] `mise bench` で GC 版と linear memory 版の比較が実行できる
- [ ] 計測結果が `benchmarks/results/` に保存されている
- [ ] ADR-002 の判定基準に照らして決定が下せる状態になっている

## Scope

### Case 1: Hello World（最小バイナリサイズ比較）

- [ ] バイナリサイズ
- [ ] 起動時間

成功基準: Wasm GC ≤ 2KB, linear memory ≤ 4KB

### Case 2: 文字列連結 100 回（アロケーション負荷比較）

- [ ] 実行時間
- [ ] ピークメモリ使用量
- [ ] バイナリサイズ

成功基準: 実行時間 C 比 2.0x 以内, メモリ入力サイズの 3 倍以内

### Case 3: Vec push/pop 10k（コンテナ性能比較）

- [ ] 実行時間
- [ ] ピークメモリ使用量
- [ ] バイナリサイズ

成功基準: 実行時間 C 比 1.5x 以内

### Case 4: 二分木 depth 20（再帰と参照比較）

- [ ] 実行時間
- [ ] ピークメモリ使用量
- [ ] バイナリサイズ

成功基準: 実行時間 C 比 1.5x 以内, メモリ ≤ ノード数 × ノードサイズ × 2

### Case 5: Result ベースエラー処理（エラー経路オーバーヘッド）

- [ ] 実行時間（エラーあり）
- [ ] 実行時間（エラーなし比較）
- [ ] バイナリサイズ

成功基準: エラーあり版がエラーなし版の 1.2x 以内

### Case 6: WASI ファイル読込（I/O 経路比較）

- [ ] 実行時間
- [ ] バイナリサイズ

成功基準: ファイルアクセス時間 + 1ms 以内

## 判定基準

| 条件 | 決定 |
|------|------|
| GC 版が全ケースで linear 版と同等以上 | GC を採用 |
| linear 版が 2 ケース以上で GC 版の 1.5x 以上高速 | linear を採用 |
| それ以外 | LLM フレンドリを優先して GC を採用 |

## References

- `docs/adr/ADR-002-memory-model.md`
- `benchmarks/`
- `scripts/run-benchmarks.sh`
- `mise.toml`
