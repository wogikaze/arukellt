# v4: 最適化 (4 軸定量目標)

> **状態**: 未着手 — v3 完了後に着手可能

---

## 1. 版の目的

v3 までに確立した言語仕様と stdlib の上で、コンパイル時間・実行時間・メモリ使用量・Wasm バイナリサイズの 4 軸について定量的な改善目標を達成する。MIR レベルの最適化パスと、バックエンドレベルの Wasm コード改善を段階的に導入する。最適化パスは各々独立して on/off できる設計とし、問題発生時に bisect 可能にする。

---

## 2. 到達目標

| 軸 | 目標値 | 計測方法 |
|----|--------|---------|
| コンパイル時間 | `hello.ark`: 50ms 以下 | `arukellt compile --time` |
| コンパイル時間 | `parser.ark` (500 行): 500ms 以下 | `arukellt compile --time` |
| 実行時間 | `fib(35)`: C (gcc -O2) 比 1.5x 以内 | wasmtime + hyperfine 3 回中央値 |
| 実行時間 | `vec-ops` (10 万要素): C 比 2.0x 以内 | wasmtime + hyperfine 3 回中央値 |
| メモリ使用量 | コンパイラ RSS: 1000 行入力で 100MB 以下 | `/proc/self/status` VmRSS |
| メモリ使用量 | 実行時 GC ヒープ: 入力サイズ比 3x 以内 | wasmtime stats |
| バイナリサイズ | `hello.wasm` (GC-native): 1KB 以下 | `wc -c` |
| バイナリサイズ | `parser.wasm` (500 行): 50KB 以下 | `wc -c` |

---

## 3. 対象範囲

| 対象 | 変更内容 |
|------|---------|
| `crates/ark-mir/src/passes/` (新規) | MIR 最適化パスの実装 (各パスを独立ファイルで) |
| `crates/ark-mir/src/mir.rs` | `MirStats.optimization_trace` フィールド追加 |
| `crates/ark-wasm/src/emit/t3_wasm_gc.rs` | バックエンドレベル最適化 |
| `crates/arukellt/src/main.rs` | `--opt-level 0/1/2` フラグ、`--time` フラグ追加 |
| `crates/ark-driver/src/session.rs` | `--opt-level` を最適化パスに伝播 |
| `benchmarks/` | 新規ベンチマーク追加 (binary_tree, json_parse 等) |
| `tests/baselines/` | CI ベースライン更新 |
| `scripts/verify-harness.sh` | perf gate の拡張 (閾値チェック強化) |

---

## 4. 非対象範囲

- LLVM バックエンドの最適化: ADR-005 に従い LLVM は Wasm 意味論に従属。LLVM 固有最適化 (autovectorize 等) は非対象。
- プロファイルガイド最適化 (PGO): v4 では静的解析ベースの最適化のみ。PGO は v5 以降で評価。
- 並行コンパイル (rayon 等による並列化): v4 では単スレッド最適化のみ。
- GC アルゴリズムの変更: wasmtime の GC 実装に依存。Arukellt 側では制御しない。
- Wasm 命令セット拡張 (SIMD, relaxed-SIMD): ADR-007 の対象外。非目標。
- メソッド構文 (`.method()`): ADR-004 P4 として v4 後半で評価するが、最適化の本流ではない。

---

## 5. 主要設計課題

### 5.1 MIR 最適化パスの独立性

各最適化パスを `crates/ark-mir/src/passes/` に独立ファイルで実装し、トレイト (v4 で ADR-004 P4 を解禁する場合) または関数ポインタで統一インタフェースを定義する。パスは `--opt-level` によって有効/無効を切り替える:

```
--opt-level 0: 最適化なし (デバッグ用)
--opt-level 1: const_folding, dce のみ (安全な最適化)
--opt-level 2: 全パス有効 (リリース用)
```

### 5.2 MIR 最適化パスの優先順位

最適化パスの適用順序 (deps あり):

1. `const_folding.rs` — 定数畳み込み: `BinOp(Const(a), Add, Const(b))` → `Const(a+b)`
2. `dce.rs` — デッドコード除去: 到達不能 BasicBlock 削除、未使用 LocalId 除去
3. `copy_propagation.rs` — コピー伝播: `let x = y; use(x)` → `use(y)`
4. `inline.rs` — インライン化: 呼び出し 1 回 + 本体 10 命令以下の関数を展開
5. `licm.rs` — ループ不変式移動: ループ内不変計算をループ外に移動
6. `escape_analysis.rs` — エスケープ解析: 関数外に漏れない struct を scalar replacement
7. `gc_hint.rs` — GC ヒント: 短命オブジェクトのパターン検出 (wasmtime ランタイム依存)

### 5.3 バックエンド最適化 (T3)

`crates/ark-wasm/src/emit/t3_wasm_gc.rs` での Wasm バイナリ改善:
- 不要な `local.get`/`local.set` ペアの削除 (peephole)
- 同一文字列リテラルの data segment dedup
- 定数条件 `if` の除去
- `struct.get` + 即座 `struct.set` のパターン最適化

### 5.4 ベンチマーク比較対象の選定

| 比較言語 | バージョン固定 | 理由 |
|---------|--------------|------|
| C (gcc -O2) | gcc 12+ | 最速の参照点 |
| Rust (--release) | 現行 stable | Wasm GC との比較基準 |
| Go | 現行版 | GC 言語での比較 |
| Grain | 現行版 | Wasm-native GC 言語との比較 |

---

## 6. 実装タスク

1. **`crates/ark-mir/src/passes/` ディレクトリ構造の設計**  
   - `mod.rs`: `OptimizationPass` trait (または関数ポインタ表) の定義
   - `const_folding.rs`, `dce.rs`, `copy_propagation.rs`, `inline.rs`, `licm.rs`, `escape_analysis.rs`, `gc_hint.rs`
   - 各パスは `fn run(module: &mut MirModule, level: OptLevel) -> PassStats` シグネチャを持つ

2. **`MirStats.optimization_trace` の追加** (`crates/ark-mir/src/mir.rs`)  
   - `optimization_trace: Vec<(PassName, ReductionStats)>` フィールド
   - `ARUKELLT_DUMP_PHASES=optimized-mir` 環境変数で最適化前後の MIR を stderr に出力

3. **定数畳み込みの実装** (`passes/const_folding.rs`)  
   - `MirStmt::Assign { rvalue: Rvalue::BinOp(Const(a), op, Const(b)) }` → 計算結果の `Const`
   - `MirStmt::Assign { rvalue: Rvalue::UnaryOp(Const(a), op) }` → 計算結果
   - `MirTerminator::BranchIf(Const(true/false), ...)` → 無条件 `Goto`

4. **DCE の実装** (`passes/dce.rs`)  
   - 到達不能 BasicBlock: エントリから BFS して未到達ブロックを削除
   - 未使用 `LocalId`: 定義されるが読まれない局所変数を削除 (副作用のある Rvalue は保持)

5. **インライン化の実装** (`passes/inline.rs`)  
   - 呼び出し回数 1 回かつ本体 10 命令以下の条件判定
   - `CallBuiltin` は対象外 (ビルトインはバックエンドで直接 emit)
   - 再帰関数は対象外 (無限展開を防ぐ)

6. **`--opt-level` フラグと `--time` フラグの追加** (`crates/arukellt/src/main.rs`)  
   - `--opt-level 0/1/2` で最適化レベル指定 (デフォルト: 1)
   - `--time` フラグで各パスの実行時間を stderr に出力 (コンパイル時間計測用)

7. **ベンチマーク追加** (`benchmarks/`)  
   - `binary_tree.ark` (depth=15)、`vec_push_pop.ark` (10 万要素)、`string_concat.ark` (1 万回)
   - 各ベンチマークの期待出力・実行コマンドを `benchmarks/README.md` に固定
   - `scripts/run-benchmarks.sh` を追加

8. **CI ベースライン更新と回帰検知** (`tests/baselines/`, `scripts/verify-harness.sh`)  
   - `tests/baselines/perf/` にベンチマーク結果を JSON で保存
   - verify-harness.sh の perf gate: コンパイル時間 +20%, 実行時間 +10%, バイナリサイズ +15% で failure
   - `scripts/update-baselines.sh` でベースラインを手動更新するコマンドを作成

9. **メソッド構文 (ADR-004 P4) の評価** (v4 後半)  
   - ADR-004 P4 の開始条件: v4 の最適化パスが全て stable であること
   - 評価対象: `.map()`, `.filter()`, `.len()`, `.push()` 等の基本メソッド
   - 解禁する場合は ADR-004 補遺として記録し、`ark-parser` + `ark-resolve` + `ark-typecheck` に追加

---

## 7. 検証方法

```bash
# 全 fixture (最適化前後で同じ結果)
cargo test -p arukellt --test harness -- --nocapture

# 最適化レベル別のビルド検証
arukellt compile --opt-level 0 tests/fixtures/basic/fib.ark -o fib_o0.wasm
arukellt compile --opt-level 2 tests/fixtures/basic/fib.ark -o fib_o2.wasm
# 出力が同じことを確認
wasmtime run fib_o0.wasm && wasmtime run fib_o2.wasm

# ベンチマーク実行
scripts/run-benchmarks.sh --compare-lang c,rust,go

# perf gate
scripts/verify-harness.sh  # 拡張 perf gate 含む

# MIR dump 確認
ARUKELLT_DUMP_PHASES=optimized-mir arukellt compile tests/fixtures/basic/fib.ark
```

---

## 8. 完了条件

| 条件 | 判定方法 |
|------|---------|
| `fib(35)`: C 比 1.5x 以内 | hyperfine 計測値 |
| `vec-ops` (10 万): C 比 2.0x 以内 | hyperfine 計測値 |
| `hello.wasm` バイナリ: 1KB 以下 | `wc -c` |
| コンパイル時間 `hello.ark`: 50ms 以下 | `arukellt compile --time` |
| 全 fixture が `--opt-level 0/1/2` 全レベルで pass する | 数値確認 |
| `--opt-level 2` の結果と `--opt-level 0` の結果が意味論的に同一である | fixture harness で検証 |
| 7 つの MIR 最適化パスが独立 on/off できる | `--no-pass=dce` 等で確認 |
| `ARUKELLT_DUMP_PHASES=optimized-mir` が動作する | コマンド実行 + 出力確認 |
| `scripts/verify-harness.sh` の拡張 perf gate が通る | exit code 0 |
| `benchmarks/README.md` にベンチマーク方法が記載されている | ファイル確認 |

---

## 9. 次版 (v5) への受け渡し

v5 が開始できる前提条件:

1. v4 の全完了条件が達成されていること
2. 7 つの MIR 最適化パスが独立 on/off でき、Arukellt 版コンパイラに移植可能な設計であること
3. コンパイル時間目標 (hello.ark 50ms, parser.ark 500ms) を達成していること (v5 Stage1/Stage2 のビルド実用性の前提)
4. v5 セルフホスト必要 stdlib 関数チェックリスト (v3 で作成) が全件 Stable で実装されていること
5. v5 着手前に `docs/language/spec.md` の凍結版を作成すること (ADR-006 の要件)

**v4 → v5 に渡す成果物**:

| 成果物 | パス |
|--------|------|
| MIR 最適化パス群 | `crates/ark-mir/src/passes/` |
| ベンチマーク基準 | `benchmarks/`, `tests/baselines/perf/` |
| `--opt-level` CLI フラグ | `crates/arukellt/src/main.rs` |
| 言語仕様凍結版 | `docs/language/spec.md` |
| 移行ガイド | `docs/migration/v3-to-v4.md` |

---

## 10. この版で特に気をつけること

1. **最適化による意味論変更の禁止**: 最適化パスの追加条件は「全 fixture test が --opt-level 0 と同じ結果を出すこと」。特に DCE で副作用を持つ式 (I/O、関数呼び出し) を削除しないこと。
2. **GC 最適化パス (`gc_hint.rs`) の限界**: wasmtime の GC アルゴリズムへのヒント提供は wasmtime の custom proposal (GC hints) 依存。wasmtime が対応していない場合は `gc_hint.rs` を no-op にし、将来有効化できる設計にしておく。
3. **インライン化の爆発**: 再帰関数 + 深い呼び出しチェーンでインライン化が爆発するリスクがある。インライン展開の最大深度 (デフォルト: 3) を設定し、超えた場合はインライン化をスキップする。
4. **ベースライン更新の手順**: `tests/baselines/perf/` の値を更新する際は `scripts/update-baselines.sh` を使い、差分をコミットに含める。手動でベースラインファイルを書き換えない。
5. **コンパイル時間 50ms の難しさ**: lex + parse + resolve + typecheck + lower + emit の合計 50ms は、特に typecheck が遅い場合に達成困難。まず計測して律速ステップを特定してから最適化する。typecheck の並列化 (rayon) は v4 で評価対象に含めるが、全体設計を壊さない範囲に限定する。
6. **ADR-004 P4 (メソッド構文) の時期**: メソッド構文は最適化パスの安定化を待ってから評価する。最適化パス実装中にパーサー変更が混じると、デグレードの原因特定が困難になる。
7. **binary_tree ベンチマークの 1.83x 問題**: ADR-002 のベンチマーク結果で `binary_tree` は 1.83x (GC-native の最悪ケース)。v4 でエスケープ解析 + scalar replacement により改善を試みる。改善できない場合は 1.83x をドキュメントし、理由と回避策を `docs/process/benchmark-results.md` に記録する。

---

## 11. この版で必ず残すドキュメント

| ドキュメント | パス | 内容 |
|------------|------|------|
| ベンチマーク計画・結果 | `benchmarks/README.md` | 方法、比較対象、結果表 |
| 言語仕様凍結版 | `docs/language/spec.md` | v5 着手前に作成する凍結版 |
| 最適化パス説明 | `docs/compiler/pipeline.md` | 各 MIR パスの説明、適用条件 |
| v3→v4 移行ガイド | `docs/migration/v3-to-v4.md` | CLI フラグ変更、API 変更点 |
| 現状ドキュメント更新 | `docs/current-state.md` | v4 完了状態、ベンチマーク結果サマリー |

---

## 12. 未解決論点

1. **トレイト (ADR-004 P4) の最終判断**: メソッド構文の導入タイミングを v4 後半で評価するが、型推論・名前解決の複雑化コストが高い場合は v5 まで延期する可能性がある。判断は ADR-004 補遺として記録。
2. **LTO (Link-Time Optimization)**: Wasm の複数モジュールにわたる LTO は Component Model と絡む。v4 では単一モジュール最適化のみを対象とし、cross-module LTO は v5 以降で評価。
3. **GC stats API**: `wasmtime stats` の GC ヒープ計測は wasmtime の API 依存。wasmtime が GC stats を公開していない場合は、メモリ使用量の目標を `VmRSS` のみで計測する。
4. **`--opt-level` デフォルト値**: デフォルトを 0 (デバッグ優先) か 1 (安全な最適化) にするかは UX 判断。AtCoder 使用 (T1) では最適化不要なケースが多い。デフォルトを 1 とし、`--debug` で 0 に切り替えられるようにする案を v4 着手時に最終決定する。
