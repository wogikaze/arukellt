# native-cpp public run (`run_supported=true`) 昇格計画

ステータス: 実行計画（決定記録ではない）
関連 ADR: [ADR-050](../adr/ADR-050-experimental-public-native-c99-run.md)、[ADR-049](../adr/ADR-049-native-c99-selfhost-executor.md)
詳細仕様: [RFC-008](../rfcs/008-native-cpp-c99-backend-runtime-abi.md)
追跡 issue: [#649](../../issues/open/649-t4-native-full-lowering.md)
作成日: 2026-07-25

---

## 1. 目標状態

ユーザーが次を実行できる状態とする。

```text
arukellt run program.ark --target native-cpp -- arg1 arg2
```

経路: `Ark source → MIR → C99 → clang → native executable → 実行`

| 軸 | 昇格後 |
|----|--------|
| `support_tier` | `scaffold` 維持 |
| `implementation_state` | `partial` |
| `contract_stability` | `experimental` |
| `run_supported` | `true`（最終 promotion commit のみ） |
| host | Linux x86-64 / LP64 / little endian |
| C compiler | clang 14+ |
| public ABI / external FFI / cross compile | なし |

`run_supported=true` は対応済み機能の公開 native 実行を意味する。全 opcode 完成を意味しない。
未対応は compile-time diagnostic。`support_tier=supported` は別作業。

## 2. Critical path

`ADR-050 → --emit c契約 → host launcher → process semantics → public fixture corpus → closure/PHI判断 → installed layout → CI/receipt → run_supported=true`

フラグだけの先行変更は禁止する。

## 3. 現状差分

成立済み:

- [x] `compile --target native-cpp -o output.c` で C99 生成
- [x] MIR direct call / CFG / scalar / String / Vec / Struct の部分 lowering
- [x] C runtime + mark-sweep GC + root liveness production clear
- [x] clang で実行可能ファイル化可能
- [x] scalar / CFG / GC stress / sanitizer tests
- [x] native selfhost executor strict 3× PASS
- [x] Linux x86-64 内部 executor contract

不足:

- [ ] 公開 CLI run 経路（host launcher）
- [ ] `--emit c` 正式契約と default `.c` 出力
- [ ] clang 探索の public 共通化
- [ ] runtime 公開配置 / installed layout
- [ ] args / stdio / cwd / env / exit / signal 契約
- [ ] public fixture corpus / parity / promotion receipt
- [ ] `run_supported=true` + release guarantee

---

# Phase 0 — 公開 native run の契約

## 0.1 ADR-050

- [x] ADR-050 accepted
- [x] ADR-049 との責務境界を明記
- [x] Linux x86-64 / clang 14+ / private ABI / no FFI / no cross compile
- [x] `run_supported` と `support_tier` を分離
- [x] ADR-029 fixpoint を置換しない
- [x] `native-llvm` に影響させない

## 0.2 CLI 契約（設計）

- [x] run: `--` 前後の compiler options / program args
- [x] stdio / cwd / env / exit code 継承
- [x] compile: `--emit c`、default emit=`c`、`-o` 省略時 `<input>.c`
- [x] `--emit exe` は初期版で追加しない（host launcher が link を所有）
- [x] issue #649 を C99 / ADR-050 前提へ全面更新

## Phase 0 完了条件

- [x] ADR-050 accepted
- [x] ADR-049 境界明記
- [x] CLI / host / toolchain / ABI / 非目標が確定
- [x] issue #649 更新
- [x] 実装開始後に未確定の設計判断が残っていない（詳細は本 plan の後続 Phase）

---

# Phase 1 — host launcher

- [ ] `scripts/run/native-cpp-runner.py`
- [ ] `scripts/native/toolchain.py`（clang 14+、executor と共有）
- [ ] `arukellt-selfhost.sh` が native-cpp run のみ host launcher へ route
- [ ] compile は selfhost へ従来どおり
- [ ] 再帰 dispatch 防止 hidden env
- [ ] `--target=native-cpp` 対応または明示拒否
- [ ] `--` 以降を target 検出に使わない
- [ ] public flags: `-std=c99 -O2 -DNDEBUG -Wall -Wextra -Wpedantic`（`-march=native` 除外）
- [ ] temp cleanup / optional keep-on-debug
- [ ] cache: `.build/native-cpp/run-cache/`（昇格前に identity 実装）

## Phase 1 完了条件

- [ ] `arukellt run hello.ark --target native-cpp` が native executable を起動
- [ ] exit / stdio / cwd / env 継承
- [ ] temp が通常時に残らない
- [ ] cache hit/miss 同結果（cache 実装後）

---

# Phase 2 — compiler emit contract

- [x] emit kind `c` を SSOT 化
- [x] target×emit 許可 matrix
- [x] native-cpp で wasm/wat/component/wit/all 拒否
- [x] Wasm target で `--emit c` 拒否
- [x] default output: native-cpp → `.c`
- [x] `project-state` から `target_run_supported` / default emit / allowed emits を生成
- [x] drift gate（generated target contract + unit tests）

## Phase 2 完了条件

- [x] `compile --target native-cpp --emit c` 正式動作
- [x] `-o` なしで `.c` 生成
- [x] 不正 emit 組合せを拒否
- [x] この時点では `run_supported=false` 維持

---

# Phase 3 — entry / process semantics

- [ ] `fn main()` / `fn main() -> Unit`（scalar 戻り値は捨てて exit 0）
- [ ] args parity（Wasm と一致、RFC-008 を実装に合わせる）
- [ ] stdio / cwd / env fixtures
- [ ] `process.exit` / panic / signal 写像

## Phase 3 完了条件

- [ ] entry 検証 + args/stdio/cwd/env/exit fixtures PASS
- [ ] RFC-008 と実装一致

---

# Phase 4 — public runtime 安全既定

- [ ] public run 既定 GC ON
- [ ] arena は debug override のみ
- [ ] trap/panic user diagnostic
- [ ] 対応済み I/O の Result 契約
- [ ] ASan/UBSan/GC stress 継続 PASS

---

# Phase 5 — public fixture corpus

ディレクトリ: `tests/fixtures/native_cpp_public/`

- [ ] scalar / CFG / String / Vec / Struct / enum / Option / Result
- [ ] host: println / args / fs / exit
- [ ] GC retention / reclaim / panic / bounds
- [ ] HOF (`MIR_REF_FUNC` / `MIR_CALL_INDIRECT`) 原則対応、外すなら Known Limitations + blocker 記録
- [ ] PHI / de-SSA
- [ ] capability registry と public coverage receipt

---

# Phase 6 — public CLI E2E / parity / sanitizer

- [ ] wrapper 経由の run/compile/error diagnostics
- [ ] Wasm/native parity corpus
- [ ] ASan/UBSan on public corpus
- [ ] generated C `-Werror` / determinism

---

# Phase 7 — packaging / installed layout

- [ ] `<prefix>/lib/arukellt/native-cpp/{ark_native_runtime.c,h}`
- [ ] runtime discovery 順序
- [ ] installed-layout smoke（repo path 非依存）

---

# Phase 8 — CI / state / docs / close

- [ ] CI lanes（PR quick / Linux native / scheduled）
- [ ] `run_native_cpp_experimental` guarantee
- [ ] `docs/data/native-cpp-run-promotion-receipt.json`
- [ ] **最後の commit で** `run_supported=true`
- [ ] docs / generated contract / false-done gate
- [ ] issue #649 close
- [ ] 内部 executor strict + ADR-029 fixpoint 退行なし

---

# 推奨 PR 分割

1. ADR + emit `c` contract（`run_supported=false`）
2. host launcher
3. entry/runtime semantics
4. public language corpus
5. packaging/cache
6. promotion（ここで初めて `run_supported=true`）

---

# Final checklist（要約）

- [ ] ADR-050 / Linux+clang14 / private ABI / no FFI
- [ ] `run --target native-cpp` が native を起動
- [ ] `compile --emit c` + default `.c`
- [ ] public corpus + parity + sanitizer + installed-layout
- [ ] receipt + guarantee + `run_supported=true`
- [ ] #649 closed、executor/fixpoint 緑
