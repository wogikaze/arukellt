# 529 — 100% Selfhost Transition: Script-Gated Execution Plan

**Purpose**: 作業者に渡して実行できる手順書。各ステップにスクリプトによるゲートがある。ゲートを通過するまで次のステップへ進まない。

**Repo**: `~/arukellt`
**Base branch**: `fix/selfhost-emitter-invalid-wasm` (current)

---

## 現在の状態 (2026-04-22)

| 項目 | 状態 |
|------|------|
| fixpoint (s2==s3) | ✅ PASS |
| fixture parity | 🔴 TBD — runner が timeout / 未実行 |
| CLI parity (narrow: --version/--help) | ✅ PASS |
| CLI parity (broad: 全コマンド surface) | 🔴 OPEN (#557, #558) |
| diagnostic parity | 🔴 24件 DIAG_PARITY_SKIP 残 |
| dual-period exit (#459) | 🔴 blocked |

---

## 作業の進め方のルール

1. **1ステップずつ進む**。前のゲートが PASS するまで次に進まない
2. **ファイルを変更するのは実装ステップのみ**。ゲート確認コマンドは読み取り専用
3. **ゲートコマンドが失敗したら STOP**。原因を報告してから実装に入ること
4. **1つの課題に1ブランチ**。複数の修正を混在させない
5. **コミット前に `python scripts/manager.py verify quick` を必ず通す**

---

## Phase 1: CLI Command Surface Parity (#557)

**Goal**: selfhost `--help` が広告するコマンドと実際に実装しているコマンドを一致させる

**Branch**: `fix/selfhost-cli-command-surface`

### 現状確認 (変更なし)

```bash
# selfhost が今何を広告しているか
wasmtime run .build/selfhost/arukellt-s1.wasm -- --help 2>&1

# Rust CLI が広告しているか
./target/debug/arukellt --help 2>&1

# selfhost で実装済みコマンド確認
grep -E '"compile"|"check"|"run"|"build"|"test"|"parse"' src/compiler/main.ark
```

**現在判明している事実**:
- selfhost 実装済み: `parse`, `compile`, `check`, `run`, `build`, `test`
- Rust CLI 全16コマンド: `compile`, `init`, `build`, `fmt`, `run`, `check`, `test`, `targets`, `script`, `component`, `lsp`, `debug-adapter`, `lint`, `analyze`, `doc`, `compose`
- 未実装コマンド: `init`, `fmt`, `targets`, `script`, `component`, `lsp`, `debug-adapter`, `lint`, `analyze`, `doc`, `compose`

### 実装 — 未実装コマンドに stub handler を追加

**対象ファイル**: `src/compiler/main.ark`

各未実装コマンドに対して以下のパターンで追加する:

```
if eq(name, "init") { return CMD_NOT_YET() }
if eq(name, "fmt") { return CMD_NOT_YET() }
// ... 残りも同様
```

`CMD_NOT_YET()` の実装:
- stderr に `"error: command not yet implemented in selfhost compiler"` を出力
- exit code 1 で終了

### ゲート 1-A (PASS するまで進まない)

```bash
python3 scripts/manager.py selfhost parity --mode --cli
# 期待: exit 0
# 期待出力に含まれること: "✓ selfhost parity --cli"
```

追加確認:
```bash
wasmtime run .build/selfhost/arukellt-s1.wasm -- init 2>&1
echo "exit: $?"
# 期待: exit 1 + "not yet implemented" メッセージ
```

### コミット

```bash
git add src/compiler/main.ark
git commit -m "feat(selfhost): add stub handlers for unimplemented CLI commands (#557)"
```

### issue close

`issues/open/557-selfhost-cli-top-level-command-surface-parity.md` を `issues/done/` へ移動し、解決証拠を記録する。

---

## Phase 2: CLI Parity Runner 拡張 (#558)

**Goal**: `scripts/manager.py selfhost parity --mode --cli` の測定範囲を拡大する

**Branch**: `fix/selfhost-cli-parity-runner`

### 現状確認 (変更なし)

```bash
grep -A 30 'def run_cli_parity' scripts/selfhost/checks.py
```

### 実装 — `scripts/selfhost/checks.py` の CLI parity runner 拡張

追加する検証ケース (優先度順):

1. `--version` — exit 0, バージョン文字列含む (既存)
2. `--help` — exit 0, コマンド一覧含む (既存)
3. 未知コマンド `foobar` — exit 非ゼロ, エラーメッセージ含む
4. `compile` 引数なし — exit 非ゼロ, usage hint 含む
5. `check` 引数なし — exit 非ゼロ
6. `run` 引数なし — exit 非ゼロ

各ケース: Rust CLI と selfhost の exit code が一致すること。stdout/stderr の exact match は求めない。

**対象ファイル**: `scripts/selfhost/checks.py` — `run_cli_parity()` 関数内に追記

### ゲート 2-A (PASS するまで進まない)

```bash
python3 scripts/manager.py selfhost parity --mode --cli
# 期待: exit 0
# ログに追加コマンドの検証が含まれること
```

### コミット

```bash
git add scripts/selfhost/checks.py
git commit -m "feat(selfhost-harness): extend CLI parity runner beyond --version/--help (#558)"
```

### issue close

`issues/open/558-expand-selfhost-cli-parity-runner-beyond-help-version.md` を `issues/done/` へ移動。

---

## Phase 3: Fixture Parity 測定と修正

**Goal**: `python3 scripts/manager.py selfhost fixture-parity` を PASS させる

**Branch prefix**: `fix/selfhost-fixture-parity-<category>`

### Step 3-0: ゲート — 現状測定 (変更なし)

```bash
timeout 300 python3 scripts/manager.py selfhost fixture-parity 2>&1 | tee /tmp/fixture-parity-baseline.txt
grep 'PASS=\|FAIL=\|SKIP=' /tmp/fixture-parity-baseline.txt
```

**このゲートを通過する条件**: 測定が完了し、FAIL/SKIP 件数が記録できること。

### Step 3-1: 失敗 fixture をカテゴリ別に整理

```bash
grep '✗\|FAIL' /tmp/fixture-parity-baseline.txt | grep '\.ark' | sort
```

カテゴリと対応する selfhost ファイル:
| カテゴリ | 対象ファイル |
|----------|--------------|
| stdlib_sort | `src/compiler/emitter.ark` |
| generics | `src/compiler/typechecker.ark` |
| closures | `src/compiler/emitter.ark` |
| arrays | `src/compiler/mir.ark` |

### Step 3-2: 最優先カテゴリを1つ選んで修正

- 1ブランチ = 1カテゴリ
- 修正後にそのカテゴリのみ再測定:

```bash
# Rust でコンパイル
./target/debug/arukellt compile tests/fixtures/<cat>/<file>.ark \
  --target wasm32-wasi-p1 -o /tmp/rust-out.wasm
wasmtime run /tmp/rust-out.wasm

# selfhost でコンパイル
wasmtime run .build/selfhost/arukellt-s1.wasm -- compile \
  tests/fixtures/<cat>/<file>.ark --target wasm32-wasi-p1 -o /tmp/self-out.wasm
wasmtime run /tmp/self-out.wasm

# 出力を比較
diff <(wasmtime run /tmp/rust-out.wasm) <(wasmtime run /tmp/self-out.wasm)
```

3-2 を FAIL=0 になるまで繰り返す。

### ゲート 3-Final (PASS するまで Phase 4 に進まない)

```bash
timeout 300 python3 scripts/manager.py selfhost fixture-parity 2>&1 | tail -5
# 期待: exit 0
# 期待: "✓ all N run: fixtures match between Rust compiler and selfhost"
```

---

## Phase 4: Diagnostic Parity

**Goal**: `DIAG_PARITY_SKIP` の 24 件を 0 にする

**Branch prefix**: `fix/selfhost-diag-parity-<category>`

### Step 4-0: ゲート — 現状測定 (変更なし)

```bash
timeout 120 python3 scripts/manager.py selfhost diag-parity 2>&1 | tee /tmp/diag-parity-baseline.txt
grep 'PASS=\|FAIL=\|SKIP=' /tmp/diag-parity-baseline.txt
```

**現在の SKIP リスト** (24件):
```python
# scripts/selfhost/checks.py の DIAG_PARITY_SKIP より
diagnostics/deprecated_prelude_println.ark
diagnostics/deprecated_std_io_import.ark
diagnostics/deprecated_time_monotonic_now.ark
diagnostics/immutable_mutation.ark
diagnostics/mismatched_arms.ark
diagnostics/missing_annotation.ark
diagnostics/mutable_sharing.ark
diagnostics/non_exhaustive.ark
diagnostics/question_type_mismatch.ark
diagnostics/type_mismatch.ark
diagnostics/unused_binding.ark
diagnostics/unused_import.ark
diagnostics/wrong_arg_count.ark
deny_clock_compile.ark
deny_random_compile.ark
host_stub_sockets.ark
module_import/use_symbol_not_found.ark
selfhost/typecheck_match_nonexhaustive.ark
stdlib_io/deny_clock.ark
stdlib_io/deny_random.ark
target_gating/t1_import_sockets.ark
target_gating/t1_import_udp.ark
v0_constraints/no_method_call.ark
v0_constraints/no_operator_overload.ark
```

### Step 4-1: 推奨修正順序

1. `diagnostics/type_mismatch.ark` — 基本型エラー
2. `diagnostics/wrong_arg_count.ark` — 引数数エラー
3. `diagnostics/unused_binding.ark` — 未使用変数警告
4. `diagnostics/unused_import.ark` — 未使用 import 警告
5. `module_import/use_symbol_not_found.ark` — モジュール解決エラー
6. `v0_constraints/` — v0 文法制約 (2件)
7. `deny_*` / `stdlib_io/deny_*` — host capability 制限 (4件)
8. `target_gating/` — target フラグ依存 (2件)
9. 残りの `diagnostics/` (6件)

各修正後:
1. `DIAG_PARITY_SKIP` から該当エントリを削除する
2. 下記ゲートで確認

```bash
timeout 120 python3 scripts/manager.py selfhost diag-parity 2>&1 | tail -5
# 期待: SKIP が1件減っていること
```

### ゲート 4-Final (PASS するまで Phase 5 に進まない)

```bash
timeout 120 python3 scripts/manager.py selfhost diag-parity 2>&1 | tail -5
# 期待: exit 0, FAIL=0, SKIP=0
```

---

## Phase 5: dual-period exit (#459 close)

**Goal**: `#459` を正式に close する

### 最終チェックリスト — すべて PASS が必要

```bash
# 1. fixpoint
python3 scripts/manager.py selfhost fixpoint
# 期待: exit 0

# 2. fixture parity
timeout 300 python3 scripts/manager.py selfhost fixture-parity
# 期待: exit 0

# 3. CLI parity (broad)
python3 scripts/manager.py selfhost parity --mode --cli
# 期待: exit 0

# 4. diagnostic parity
timeout 120 python3 scripts/manager.py selfhost diag-parity
# 期待: exit 0

# 5. 全体 harness
python3 scripts/manager.py verify quick
# 期待: exit 0
```

すべて PASS したら:
```bash
# issues/open/459-selfhost-fixpoint-dual-period-end.md に証拠を記録してから
mv issues/open/459-selfhost-fixpoint-dual-period-end.md issues/done/

# docs 更新
# docs/current-state.md の bootstrap status table を更新
python3 scripts/gen/generate-docs.py

# commit
git add issues/done/459-selfhost-fixpoint-dual-period-end.md docs/
git commit -m "close #459: dual-period exit conditions met"
```

---

## Phase 6 以降 (dual-period exit 後 — 今は対象外)

- Phase 5: Rust core compiler crates deletion (`crates/ark-driver` 等)
- Phase 6: IDE / LSP selfhost 移行
- Phase 7: Cargo workspace 完全削除

詳細は `issues/open/529-100-percent-selfhost-transition-plan.md` を参照。

---

## 参照

- `issues/open/529-100-percent-selfhost-transition-plan.md` — 設計原典
- `issues/open/459-selfhost-fixpoint-dual-period-end.md` — dual-period exit gate
- `issues/open/557-selfhost-cli-top-level-command-surface-parity.md` — CLI surface #557
- `issues/open/558-expand-selfhost-cli-parity-runner-beyond-help-version.md` — CLI runner #558
- `scripts/selfhost/checks.py` — parity 測定ロジック
- `scripts/manager.py` — エントリポイント
- `src/compiler/main.ark` — selfhost CLI entrypoint
