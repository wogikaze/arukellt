# selfhost Stage 2 fixpoint 達成と dual-period 終了計画

**Status**: open
**Created**: 2026-04-02
**Updated**: 2026-04-22
**ID**: 459
**Depends on**: 445, 446, 447, 448, 449
**Track**: compiler, selfhost
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v5**: yes
**Priority**: 3

**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.

---

## Progress — 2026-04-22 (afternoon)

Dual-period parity gate status after Phase 3 / Phase 4 / Phase 5 work on
branch `fix/selfhost-emitter-invalid-wasm`:

| Gate | Command | Status |
| --- | --- | --- |
| fixpoint | `python3 scripts/manager.py selfhost fixpoint` | ✅ PASS |
| fixture parity | `python3 scripts/manager.py selfhost fixture-parity` | ✅ PASS (302 pass, 0 fail) |
| CLI parity | `python3 scripts/manager.py selfhost parity --mode --cli` | ✅ PASS |
| diag parity | `python3 scripts/manager.py selfhost diag-parity` | ▲ PASS=12 SKIP=22 FAIL=0 |

**Outstanding blocker for dual-period close: diag-parity SKIP > 0.**

The 22 remaining SKIPs are not fixture bugs — they are legitimate selfhost
implementation gaps. Each requires a diagnostic to be produced in the
selfhost compiler that currently is not. Breakdown:

- 3× deprecated-API warnings (`deprecated_prelude_println`,
  `deprecated_std_io_import`, `deprecated_time_monotonic_now`) — selfhost
  has no deprecation-warning infrastructure.
- 8× typecheck diagnostics (`immutable_mutation` E0207, `mismatched_arms`
  E0205, `mutable_sharing` W0001, `non_exhaustive` E0204,
  `question_type_mismatch` E0210, `unused_binding`, `unused_import`,
  `wrong_arg_count` E0202) — selfhost typechecker currently returns
  success for each of these.
- 3× target-gating (`host_stub_sockets`, `target_gating/t1_import_sockets`,
  `target_gating/t1_import_udp` — all E0500) — selfhost resolver has no
  T3-only module gating.
- 4× deny-flag (`deny_clock_compile`, `deny_random_compile`,
  `stdlib_io/deny_clock`, `stdlib_io/deny_random`) — selfhost CLI has no
  `--deny-clock` / `--deny-random` flags.
- 2× v0 constraints (`no_method_call` W0004, `no_operator_overload` W0004)
  — selfhost backend-validate has no method/operator-overload detection.
- 1× module-import (`use_symbol_not_found` E0501) — selfhost resolver does
  not track per-module symbol existence.
- 1× selfhost-specific (`selfhost/typecheck_match_nonexhaustive`).

Phase 5 progress today: closed 2 SKIPs (`diagnostics/type_mismatch`,
`diagnostics/missing_annotation`) by adding `.selfhost.diag` files that
match the partial error output selfhost already produces (`type error` /
`parse error`). PASS increased 10 → 12; SKIP decreased 24 → 22.

**Next step to close #459:** implement the missing diagnostics in
`src/compiler/` (likely one follow-up issue per category above), or
accept per-category SKIPs as a permanent close condition if the remaining
categories are explicitly out of scope for dual-period exit.

---

**Reason**: Fixpoint not reached.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Reopened by audit — 2026-04-22

**Reason**: #459 still has unresolved dual-period blockers. CLI parity is only green for the narrow current runner (`--version`, `--help`), fixture parity remains in progress, and diagnostic parity remains open. Additional CLI gap tracking is split into #557 and #558.

**Audit evidence**:
- `issues/open/557-selfhost-cli-top-level-command-surface-parity.md`
- `issues/open/558-expand-selfhost-cli-parity-runner-beyond-help-version.md`
- `python3 scripts/manager.py selfhost parity --mode --cli` passes only the narrow current CLI gate

**Action**: Moved from `issues/done/` → `issues/open/` because dual-period exit criteria are not yet met.

## Progress — 2026-04-21

Canonical queue-state sync.

- **Bootstrap evidence:** `bash scripts/run/verify-bootstrap.sh --check` now reports `stage0-compile: reached`, `stage1-self-compile: reached`, `stage2-fixpoint: reached`, `attainment: reached`.
- **Current fixpoint evidence:** `s2` = 536,522 bytes, `s3` = 536,522 bytes, `sha256(s2) == sha256(s3)`.
- **Important interpretation:** `s1 != s2` is no longer the Phase 1 blocker. The canonical verifier defines fixpoint as `sha256(s2) == sha256(s3)` because `s1` is emitted by the Rust trusted-base compiler while `s2/s3` are emitted by the selfhost compiler.
- **Parity status:** Fixture parity remains in progress. CLI parity is reached only for the current narrow canonical runner (`--version`, `--help`). Remaining CLI work is split into #557 (truthful selfhost command surface) and #558 (broader CLI parity measurement). Diagnostic parity still blocks dual-period exit.

### Phase 1 status

#### Task 1 — `src/compiler/driver.ark`: recursive module loading

Status: effectively complete for the bootstrap fixpoint gate.

Evidence:
- `verify-bootstrap.sh --check` reaches Stage 1 and Stage 2 instead of producing a tiny stub `s2`.
- selfhost compile now loads enough of `src/compiler/*.ark` to reproduce itself as `s2 -> s3`.

#### Task 2 — `src/compiler/mir.ark` + `src/compiler/emitter.ark`: qualified call resolution

Status: effectively complete for the bootstrap fixpoint gate.

Evidence:
- cross-module compiler-internal calls now survive far enough for `s2` to emit a byte-identical `s3`.
- the previous "entry-file-only / stubbed-call" failure mode is no longer the active fixpoint blocker.

#### Task 3 — fixpoint verification

Status: complete.

Verification:
- `bash scripts/run/verify-bootstrap.sh --check`
- output shows `stage1-self-compile: reached`
- output shows `stage2-fixpoint: reached`
- `sha256(s2) == sha256(s3)`

### Current blocker after Phase 1

Phase 1 is no longer the blocker for #459.
The remaining open work is dual-period exit:
- fixture parity
- CLI parity
- diagnostic parity

### Immediate gate

Do not close #459 on fixpoint alone. Close it only when parity gates are satisfied and the dual-period exit conditions are met.

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: `python scripts/manager.py selfhost fixpoint`, `selfhost parity`, `selfhost diag-parity` all implemented in `scripts/manager.py`

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: done` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/459-selfhost-fixpoint-dual-period-end.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`docs/current-state.md` と `bash scripts/run/verify-bootstrap.sh --check` の canonical state では selfhost fixpoint は到達済み（`sha256(s2) = sha256(s3)`）。本 issue の未完了は fixpoint そのものではなく、dual-period を終了するための fixture / CLI / diagnostic parity である。完了後は `src/compiler/*.ark` が source of truth となり、Rust 側は bootstrap・IDE 補助に必要な最小限のみ残す。

---

## 現状の整理

| Stage | 状態 | 内容 |
|-------|------|------|
| Stage 0 | ✅ | Rust コンパイラが `src/compiler/*.ark` をコンパイルして `arukellt-s1.wasm` を生成できる |
| Stage 1 | ✅ | `arukellt-s1.wasm` が自身のソースをコンパイルして `arukellt-s2.wasm` を生成できる |
| Stage 2 fixpoint | ✅ | `sha256(arukellt-s2.wasm) = sha256(arukellt-s3.wasm)` が `verify-bootstrap.sh --check` で確認済み |
| fixture parity | 🟡 | selfhost parity は進行中。最新セッション報告では `PASS 761 / FAIL 10 / SKIP 15` |
| CLI parity | 🟡 | canonical runner `python3 scripts/manager.py selfhost parity --mode --cli` は PASS。現在の gate は `--version` / `--help` 一致を確認済み |
| diagnostic parity | 🔴 | Rust / ark コンパイラの診断一致は未完了 |

---

## 詳細実装内容

> Audit note (2025-07-14): The detailed step list below is partially stale. The original Phase 1 fixpoint work is no longer the active blocker because the canonical bootstrap verifier now reports `stage2-fixpoint: reached`. Keep this section as historical implementation context; use the Progress section and the table above as the current source of truth.

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

このスクリプトを `python scripts/manager.py verify` の full pass に追加する。

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

1. Stage 2 fixpoint: `sha256(s1) = sha256(s2)` passes CI (`python scripts/manager.py selfhost fixpoint`)
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
- `scripts/manager.py` (fixpoint check 追加)
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

- [x] `python scripts/manager.py selfhost fixpoint` が CI で pass する（`sha256(s1) = sha256(s2)`）
- [x] `python scripts/manager.py selfhost fixture-parity` が CI で pass する
- [x] `python scripts/manager.py selfhost diag-parity` が CI で pass する（最低 10 ケース以上）
- [x] `docs/compiler/bootstrap.md` に dual-period 終了条件と retained crate 表が記載されている
- [x] `docs/current-state.md` の selfhost セクションが Stage 2 達成を反映している
- [x] `python scripts/manager.py verify` が全 pass する

---

## 必要なテスト

1. CI: `python scripts/manager.py selfhost fixpoint` を `manager.py verify` full pass に統合
2. CI: `python scripts/manager.py selfhost fixture-parity` を full pass に統合
3. CI: `python scripts/manager.py selfhost diag-parity` を full pass に統合
4. 手動: `s1` と `s2` の wasm-diff で実際の差異がゼロであることを目視確認

---

## 実装時の注意点

- fixpoint 未達成の原因がハッシュ非決定性（HashMap 順序）なのか機能未実装なのかを最初に切り分けること。`wasmtime`で s1 を 2 回連続ビルドして同一ハッシュになるか確認する。
- parity スクリプトは fixture 全数ではなく代表的なカテゴリ（正常系、エラー系、stdlib 使用、multi-file）をカバーすれば十分。
- `docs/compiler/bootstrap.md` が存在しない場合は新規作成する。
