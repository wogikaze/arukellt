# ベンチマーク結果

## 実行状況

**ステータス**: ベンチマーク全 6 ケース実行済み（2026-03-25）

**実行コマンド**:

```bash
PATH="$HOME/.wasmtime/bin:$HOME/.cargo/bin:$PATH" ./harness/proto/run_bench.sh
```

**実行環境**:
- Wasmtime: `43.0.0 (be23469ec 2026-03-20)`
- WAT compiler: `wasm-tools`（`wat2wasm` 代替）

---

## プロトタイプ一覧

| ケース | GC版 | Linear版 |
|--------|------|---------|
| Case 1: Hello World | `harness/proto/gc/hello.wat` | `harness/proto/linear/hello.wat` |
| Case 2: 文字列連結 100 回 | `harness/proto/gc/string_concat.wat` | `harness/proto/linear/string_concat.wat` |
| Case 3: Vec push/pop 10k | `harness/proto/gc/vec_pushpop.wat` | `harness/proto/linear/vec_pushpop.wat` |
| Case 4: 二分木 depth 15 | `harness/proto/gc/binary_tree.wat` | `harness/proto/linear/binary_tree.wat` |
| Case 5: Result 多用 2000 回 | `harness/proto/gc/result_heavy.wat` | `harness/proto/linear/result_heavy.wat` |
| Case 6: WASI ファイル読込 | `harness/proto/gc/file_read.wat` | `harness/proto/linear/file_read.wat` |

---

## 実測結果（全 6 ケース）

`harness/proto/results.txt` の結果（実行時間は 10 回実行の中央値）:

| ケース | 指標 | GC | Linear | 比率 (GC/Linear) |
|--------|------|----|--------|------------------|
| hello | Binary Size | 356 B | 170 B | 2.09x |
| hello | Execution Time | 24 ms | 17 ms | 1.41x |
| string_concat | Binary Size | 466 B | 462 B | 1.00x |
| string_concat | Execution Time | 17 ms | 21 ms | **0.80x** (GC 優位) |
| vec_pushpop | Binary Size | 797 B | 770 B | 1.03x |
| vec_pushpop | Execution Time | 25 ms | 17 ms | 1.47x |
| binary_tree | Binary Size | 441 B | 516 B | **0.85x** (GC 優位) |
| binary_tree | Execution Time | 33 ms | 18 ms | **1.83x** |
| result_heavy | Binary Size | 433 B | 390 B | 1.11x |
| result_heavy | Execution Time | 18 ms | 16 ms | 1.12x |
| file_read | Binary Size | 566 B | 474 B | 1.19x |
| file_read | Execution Time | 16 ms | 15 ms | 1.06x |

---

## 判定基準の適用（`benchmark-plan.md` より）

| 条件 | 結果 |
|------|------|
| GC 版が全ケースで linear 版と同等以上 | **不成立**（vec_pushpop, binary_tree 等で GC が遅い） |
| linear 版が **2 ケース以上**で GC 版の 1.5x 以上高速 | **不成立**（該当: binary_tree 1.83x のみ = 1 ケース） |
| それ以外 | → **LLM フレンドリを優先して GC を採用** |

1.5x 超の判定は binary_tree（1.83x）の 1 ケースのみ。
2 ケース条件を満たさないため「それ以外」に該当 → **Wasm GC 採用**。

---

## 決定

**Wasm GC を採用する**（2026-03-25 実測に基づく確定）

根拠:
- `benchmark-plan.md` の判定基準「それ以外 → LLMフレンドリを優先して GC を採用」に該当。
- string_concat では GC が Linear より 20% 高速（GC 側の immutable copy が有利なパターン）。
- binary_tree を除き実行時間差は 1.5x 以内。
- LLM フレンドリ性（ライフタイム管理不要）の価値は変わらない。
