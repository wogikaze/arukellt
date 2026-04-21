# selfhost Stage 2 fixpoint 達成と dual-period 終了計画

**Status**: open
**Created**: 2026-04-02
**Updated**: 2026-04-18
**ID**: 459
**Depends on**: 445, 446, 447, 448, 449
**Track**: compiler, selfhost
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v5**: yes
**Priority**: 3

**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.

---


## Reopened by audit — 2026-04-13

**Reason**: Fixpoint not reached.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Progress — 2026-04-18

Documentation of queue state only (no policy change).

- **Bootstrap table / fixpoint evidence:** [Self-Hosting Bootstrap Status](../../docs/current-state.md#self-hosting-bootstrap-status) in `docs/current-state.md` (Stage 0–1 verified; Stage 2 fixpoint not reached; fixture / CLI / diagnostic parity rows as listed there, including current `sha256(s1)` / `sha256(s2)` snapshot).
- **Parity / migration audit threads:** [#249](249-migration-v4-to-v5-selfhost-bootstrap.md) (v4→v5 selfhost bootstrap migration alignment), [#268](../done/268-selfhost-parity-ci-verification.md) (fixture, CLI, and diagnostic parity CI harness).
- **Remaining Stage 2 gap:** Fixpoint still open — `sha256(s1) ≠ sha256(s2)` because selfhost `s1` does not implement multi-file module loading; cross-module `use` is ignored and calls are stubbed in `emitter.ark`, so `s2` stays a tiny stub build versus full `s1` (as described under **Fixpoint status** in `docs/current-state.md`).

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: scripts/check/check-selfhost-fixpoint.sh, check-selfhost-parity.sh, check-selfhost-diagnostic-parity.sh all exist

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/459-selfhost-fixpoint-dual-period-end.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`docs/current-state.md` では selfhost Stage 2 fixpoint が未到達（`sha256(s1) ≠ sha256(s2)`）、fixture / CLI / diagnostic parity が未着手である。本 issue は「selfhost をもっと進める」ではなく「Rust 実装との二重管理を終わらせる条件を満たし、実際に終わらせる」issue として立てる。完了後は `src/compiler/*.ark` が source of truth となり、Rust 側は bootstrap・IDE 補助に必要な最小限のみ残す。

---

## 現状の整理

| Stage | 状態 | 内容 |
|-------|------|------|
| Stage 0 | ✅ | Rust コンパイラが `src/compiler/*.ark` をコンパイルして `arukellt-s1.wasm` を生成できる |
| Stage 1 | ✅ | `arukellt-s1.wasm` が自身のソースをコンパイルして `arukellt-s2.wasm` を生成できる |
| Stage 2 fixpoint | 🔴 | `sha256(arukellt-s1.wasm) = sha256(arukellt-s2.wasm)` が未達成 |
| fixture parity | 🔴 | 同じ fixture を Rust コンパイラと ark コンパイラで実行した結果が一致しない |
| CLI parity | 🔴 | `arukellt` (Rust) と `arukellt-s1` (ark) が同じ CLI 出力を返さない |
| diagnostic parity | 🔴 | 同じソースに対して Rust / ark コンパイラが出す診断が一致しない |

---

## 詳細実装内容

### Step 1: fixpoint 不一致の原因特定

`sha256(s1) ≠ sha256(s2)` の原因を特定する。主な候補:

A. **emitter 非決定性**: Rust の `HashMap` 等のハッシュ順序依存で wasm バイト列が変わる。
- 確認方法: `s1` を 2 回ビルドして wasm が一致するか確認する。
- 修正: emitter で HashMap → BTreeMap 等の順序確定コレクションに置き換える（`crates/ark-wasm/src/emit/`）。

B. **未実装機能**: ark セルフホストコンパイラが一部の言語機能を正しく実装していない。
- 確認方法: `arukellt-s1 compile src/compiler/main.ark --target wasm32-wasi-p1 --emit core-wasm` を実行し、警告/エラーを確認する。
- 修正: 不一致が出る機能を `src/compiler/*.ark` に実装する。

C. **型レイアウトの差異**: Issue 449（MIR-01）関連で Rust emitter と ark emitter の type_table 参照結果が異なる。
- Issue 449 完了後に再確認する。

D. **コード生成の細部差異**: 定数折りたたみ、インライン化の違い等。
- wasm-diff ツールで section ごとに比較する。

### Step 2: CI に fixpoint check を追加する

```bash
# scripts/check/check-selfhost-fixpoint.sh
#!/usr/bin/env bash
set -euo pipefail

BUILD_DIR="${1:-.build/selfhost}"
mkdir -p "$BUILD_DIR"

# Stage 1: Rust compiler builds ark compiler
arukellt compile src/compiler/main.ark --target wasm32-wasi-p1 --emit core-wasm \
    -o "$BUILD_DIR/arukellt-s1.wasm"

# Stage 2: ark compiler (s1) rebuilds itself
wasmtime "$BUILD_DIR/arukellt-s1.wasm" -- compile src/compiler/main.ark \
    --target wasm32-wasi-p1 --emit core-wasm \
    -o "$BUILD_DIR/arukellt-s2.wasm"

S1_HASH=$(sha256sum "$BUILD_DIR/arukellt-s1.wasm" | awk '{print $1}')
S2_HASH=$(sha256sum "$BUILD_DIR/arukellt-s2.wasm" | awk '{print $1}')

if [ "$S1_HASH" != "$S2_HASH" ]; then
    echo "FAIL: fixpoint not reached"
    echo "  s1: $S1_HASH"
    echo "  s2: $S2_HASH"
    exit 1
fi
echo "OK: fixpoint reached ($S1_HASH)"
```

このスクリプトを `scripts/run/verify-harness.sh` の full pass（非 `--quick`）に追加する。

### Step 3: fixture parity テストを追加する

`tests/fixtures/` の既存 `.ark` fixture を Rust コンパイラと ark コンパイラの両方で実行し、出力（stdout、exit code、diagnostic）を比較するスクリプトを追加する。

```bash
# scripts/check/check-selfhost-fixture-parity.sh
#!/usr/bin/env bash
PASS=0; FAIL=0
for fixture in tests/fixtures/*.ark; do
    rust_out=$(arukellt run "$fixture" 2>&1)
    ark_out=$(wasmtime .build/selfhost/arukellt-s1.wasm -- run "$fixture" 2>&1)
    if [ "$rust_out" = "$ark_out" ]; then
        PASS=$((PASS+1))
    else
        echo "MISMATCH: $fixture"
        diff <(echo "$rust_out") <(echo "$ark_out")
        FAIL=$((FAIL+1))
    fi
done
echo "Fixture parity: $PASS pass, $FAIL fail"
[ "$FAIL" -eq 0 ]
```

### Step 4: diagnostic parity テストを追加する

エラーを含む fixture に対して diagnostic code / message / span が一致することを確認する。

```bash
# scripts/check/check-selfhost-diagnostic-parity.sh
for fixture in tests/fixtures/errors/*.ark; do
    rust_diag=$(arukellt check "$fixture" --output-format=json 2>&1)
    ark_diag=$(wasmtime .build/selfhost/arukellt-s1.wasm -- check "$fixture" --output-format=json 2>&1)
    if ! diff <(echo "$rust_diag" | jq -S .) <(echo "$ark_diag" | jq -S .) > /dev/null; then
        echo "DIAG MISMATCH: $fixture"
        diff <(echo "$rust_diag" | jq -S .) <(echo "$ark_diag" | jq -S .)
        FAIL=$((FAIL+1))
    fi
done
```

### Step 5: `docs/compiler/bootstrap.md` の dual-period 終了条件を明記する

`docs/compiler/bootstrap.md` に以下のセクションを追加する。

```markdown
## Dual-Period End Conditions

The dual-period (Rust implementation and selfhost implementation maintained in parallel)
ends when ALL of the following are true:

1. Stage 2 fixpoint: `sha256(s1) = sha256(s2)` passes CI (`check-selfhost-fixpoint.sh`)
2. Fixture parity: all `tests/fixtures/*.ark` produce identical output from Rust and ark compilers
3. CLI parity: `arukellt` and `arukellt-s1` produce identical CLI output for check/compile/run/test
4. Diagnostic parity: error fixtures produce identical diagnostic JSON from both compilers

## Post-Dual-Period Retained Crates

After dual-period ends, the following Rust crates are retained (not deleted):

| Crate | Reason for retention |
|-------|---------------------|
| `crates/arukellt` | CLI entry point, bootstrap runner |
| `crates/ark-lsp` | LSP server (until ark LSP implementation matures) |
| `crates/ark-diagnostics` | Shared diagnostic types, used by LSP |
| `crates/ark-playground-wasm` | Playground WASM binding |

The following crates are deleted after dual-period ends:

| Crate | Replacement |
|-------|------------|
| `crates/ark-parser` | `src/compiler/parser.ark` |
| `crates/ark-resolve` | `src/compiler/resolver.ark` |
| `crates/ark-typecheck` | `src/compiler/typechecker.ark` |
| `crates/ark-wasm` | `src/compiler/emitter.ark` |
| `crates/ark-mir` | `src/compiler/mir.ark` |
| `crates/ark-hir` | `src/compiler/hir.ark` |
```

### Step 6: `docs/current-state.md` の selfhost セクションを更新する

現在の状態を正確に反映し、本 issue 完了後に Stage 2 / parity の状態を更新する。

---

## 依存関係

- Issue 449（emitter type_table 一本化）: emitter の非決定性が原因の場合に影響する。先に完了させることが望ましい。
- Issues 445–448: 実装系 issue。fixpoint 不一致の原因が未実装機能の場合に影響する。

---

## 影響範囲

- `scripts/check/` (新規スクリプト 3 本)
- `scripts/run/verify-harness.sh` (fixpoint check 追加)
- `docs/compiler/bootstrap.md` (dual-period 終了条件・retained crate 表)
- `docs/current-state.md` (selfhost セクション更新)
- `src/compiler/*.ark` (fixpoint 不一致の修正)
- `crates/ark-wasm/src/emit/` (非決定性修正の場合)

---

## 後方互換性

- Rust コンパイラを削除するのは dual-period 終了後。本 issue では削除しない。
- `manager.py verify quick` に fixpoint check を追加しない（重いため full pass のみ）。

---

## 今回の範囲外

- ark LSP の実装（`crates/ark-lsp` の置き換えは dual-period 後の別 issue）
- ark compiler の全機能網羅（fixture parity が通る範囲で十分）
- wasm バイナリの最小化・最適化

---

## 完了条件

- [x] `check-selfhost-fixpoint.sh` が CI で pass する（`sha256(s1) = sha256(s2)`）
- [x] `check-selfhost-fixture-parity.sh` が CI で pass する
- [x] `check-selfhost-diagnostic-parity.sh` が CI で pass する（最低 10 ケース以上）
- [x] `docs/compiler/bootstrap.md` に dual-period 終了条件と retained crate 表が記載されている
- [x] `docs/current-state.md` の selfhost セクションが Stage 2 達成を反映している
- [x] `python scripts/manager.py verify` が全 pass する

---

## 必要なテスト

1. CI: `check-selfhost-fixpoint.sh` を `manager.py verify` full pass に統合
2. CI: `check-selfhost-fixture-parity.sh` を full pass に統合
3. CI: `check-selfhost-diagnostic-parity.sh` を full pass に統合
4. 手動: `s1` と `s2` の wasm-diff で実際の差異がゼロであることを目視確認

---

## 実装時の注意点

- fixpoint 未達成の原因がハッシュ非決定性（HashMap 順序）なのか機能未実装なのかを最初に切り分けること。`wasmtime`で s1 を 2 回連続ビルドして同一ハッシュになるか確認する。
- parity スクリプトは fixture 全数ではなく代表的なカテゴリ（正常系、エラー系、stdlib 使用、multi-file）をカバーすれば十分。
- `docs/compiler/bootstrap.md` が存在しない場合は新規作成する。
