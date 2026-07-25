# ADR-050: Experimental Public Native C99 Run（公開 experimental native 実行）

ステータス: **ACCEPTED** — `native-cpp` の公開 `arukellt run` を Linux x86-64 限定の experimental 契約として採択する

決定日: 2026-07-25

---

## 文脈

[ADR-049](ADR-049-native-c99-selfhost-executor.md) は `native-cpp` を **内部 selfhost
executor** として採択し、一般ユーザー向け native 製品を非目標とした。その後、内部
executor lane は experimental まで到達した（root clear、strict wall/RSS dual gate、
CI 契約）。一方で公開 CLI の

```text
arukellt run program.ark --target native-cpp -- arg1 arg2
```

は当時未提供だった（`run_supported=false`、`project_run` は Wasm 固定）。

公開 native 実行を再開するには、内部 executor 契約を壊さず、support commitment を
広げすぎない境界が必要である。`support_tier=supported` まで上げると Windows / macOS /
長期互換 / 配布 ABI まで期待されるため、本 ADR は **run 可否** と **support tier** を
分離する。公開 experimental run は採択済み（`run_supported=true`）。全 fixture 実用は
別計画 [native-cpp-general-backend-readiness.md](../plans/native-cpp-general-backend-readiness.md)
が追跡する。

## 決定

### 1. `run_supported=true` の意味

`run_supported=true` は次を意味する。

- 対応済み言語機能を使ったプログラムを、公開 CLI で native 実行できる。
- 経路は `Ark source → MIR → C99 → clang → native executable → 実行` である。
- 全 MIR opcode、全 CoreOp、SIMD、async、network、Component Model の完成を意味しない。
- 未対応機能は compile-time の target capability diagnostic で拒否する。ICE、stub、偽値、
  実行時 trap への遅延は禁止する。

`support_tier` は当面 `scaffold` のままとする。`supported` への昇格は別 ADR / 別作業である。

### 2. ADR-049 との責務境界

| 領域 | 所有者 |
|------|--------|
| 内部 selfhost executor lane、strict dual gate、executor promotion receipt | ADR-049 |
| 公開 `run` / `compile --emit c`、host launcher、installed runtime layout、public fixture corpus | ADR-050 |
| portable C99 生成と private runtime ABI の詳細 | RFC-008（両 ADR が共有） |
| 正規 Wasmtime selfhost fixpoint | ADR-029（置換しない） |

ADR-049 を SUPERSEDE しない。内部 executor は維持し、公開 run はその上に載る別契約である。
`native-llvm` には影響させない。

### 3. 対象環境と toolchain

- host: Linux x86-64、LP64、little endian、libc 必須
- C compiler: clang 14 以上必須
- gcc 互換を保証しない
- cross compile を保証しない
- Windows / macOS を保証しない

### 4. ABI / FFI

- generated C は compiler-private ABI である
- generated C の source 互換性を保証しない
- runtime ABI は compiler と同時配布される内部 ABI である（整数 ABI version を持つ）
- external C から Ark 関数を呼ぶ FFI を保証しない
- Ark から外部 C 関数を呼ぶ FFI を保証しない
- 公開安定 C ABI は提供しない（[ADR-006](ADR-006-abi-policy.md)）

### 5. host access

公開 `native-cpp` run は ambient host access を持つ。WASI sandbox 相当の分離は保証しない。
capability の公開範囲は registry と public fixture corpus が正本とする。

### 6. CLI 契約

公開 run:

```text
arukellt run <file.ark> --target native-cpp [compiler options] -- [program arguments]
```

- `--` より前は compiler option、後は native executable へそのまま渡す
- stdin / stdout / stderr / cwd / environment を継承する
- native executable の exit code を CLI exit code とする
- signal 終了の写像規則を実装計画で固定する
- compile / clang 失敗時は executable を起動しない

公開 compile:

```text
arukellt compile <file.ark> --target native-cpp --emit c -o program.c
```

- `--emit c` を正式 emit kind とする
- native-cpp の既定 emit kind は `c`
- native-cpp で `--emit wasm` / `wat` / `component` / `wit` / `all` を拒否する
- `-o` 省略時は `<input>.c`
- 実行可能ファイル生成（`--emit exe`）は初期版で追加しない。clang orchestration は
  host launcher が所有し、compiler 本体は C source 生成までを所有する（ADR-049 / RFC-008
  の分離を維持する）

### 7. 状態モデル

昇格後の推奨 target state:

| 軸 | 値 |
|----|----|
| `support_tier` | `scaffold` |
| `implementation_state` | `partial` |
| `contract_stability` | `experimental` |
| `run_supported` | `true`（Phase 完了後の最終 commit でのみ変更） |

内部 executor lane state と公開 run state を machine-readable に分離して維持する。
`run_supported=true` への変更は、実装計画の最終 promotion checklist が満たされた
最後の commit だけで行う。フラグだけの先行変更を禁止する。

### 8. 検証と packaging

- 公開 CLI end-to-end、public fixture corpus、Wasm/native parity、ASan/UBSan、
  installed-layout smoke、clang 14 と CI clang を要求する
- release guarantee `run_native_cpp_experimental` と
  `docs/data/native-cpp-run-promotion-receipt.json` を要求する
- 正規 selfhost fixpoint（ADR-029）と内部 executor strict lane（ADR-049）を退行させない

**public corpus 完成 ≠ 全 fixture 実用。** 保証範囲は `native_cpp_public` と
capability registry である。全 manifest 向け readiness は measure v2 の正式指標を使う
（v1 の compile 56% / run 43% は `fn main(` 絞り + exit-0 の暫定・未分類）。

| 正式指標 | 定義 |
|----------|------|
| positive compile pass | compile 成功が期待される positive のうち compile 成功 |
| positive semantic run pass | positive のうち stdout/stderr/exit/signal が期待値と一致 |
| compiled-positive semantic run pass | compile 成功 positive のうち実行結果が期待値と一致 |
| expected-negative diagnostic pass | compile 失敗だけでなく diagnostic 内容も期待値と一致 |
| unexpected ICE | 入力種別を問わず compiler ICE（最終 gate は `ice_total == 0`） |
| unexpected crash | 期待結果と一致しない signal 終了・runtime 異常 |
| public corpus / Wasm-native parity | 既存 gate を 100% 維持 |

証拠: `docs/data/native-cpp-fixture-coverage-receipt.json`（v2 以降）、
計画: [native-cpp-general-backend-readiness.md](../plans/native-cpp-general-backend-readiness.md)。

## 帰結

- 公開 experimental native run の設計境界が固定される
- 実装順序と PR 分割は
  [`docs/plans/native-cpp-public-run-promotion.md`](../plans/native-cpp-public-run-promotion.md)
  が所有する。一般 backend readiness は
  [`docs/plans/native-cpp-general-backend-readiness.md`](../plans/native-cpp-general-backend-readiness.md)
  が所有する
- issue [#649](../../issues/done/649-t4-native-full-lowering.md) が公開 run 契約の作業追跡を所有した（完了）
- `support_tier=supported`、配布 ABI、外部 FFI、非 Linux host は未採択のまま残る

## Known Limitations（experimental 公開 run）

- **Capture 付き closure:** `MIR_REF_FUNC` / `MIR_CALL_INDIRECT` の zero-capture
  （named `fn(...)` / typed funcref）のみ対応する。environment を持つ capture closure は
  未対応とし、対応前は compile-time 拒否または別 issue で追跡する（#649 サブ項目）。
- **未注釈の局所 funcref:** `let f = add_one; f(x)` のように型が `fn(...)` として
  残らない局所束縛は、現状 MIR が direct `CALL` になり得る。公開 corpus は
  `fn apply(f: fn(i32) -> i32, ...)` 形の zero-capture を正とする。
- **PHI:** `block.phis` は predecessor edge の parallel copy で lower する。現行の
  structured if/else 合流は共有 local 代入が主流であり、PHI 自体は稀である。

## 代替案

1. **ADR-049 を直接拡張する** — 却下。内部 executor と公開 run の完了条件・CI・receipt が混ざる。
2. **最初から `support_tier=supported`** — 却下。ホスト面と互換期待が過大になる。
3. **compiler Wasm 内で clang を起動する** — 却下。selfhost は Wasm 内で動き、process
   orchestration は host launcher が所有すべきである。
4. **`--emit exe` を compiler に持たせる** — 初期版では却下。C 生成と host link の境界を保つ。

## 再検討条件

- Windows / macOS / cross compile を正式対応する
- 公開安定 C ABI または外部 FFI を提供する
- `support_tier=supported` へ上げる
- WASI 相当の sandbox を native に導入する
- `--emit exe` を compiler 契約へ取り込む

## 関連

- [ADR-006: 公開 ABI 境界の分類](ADR-006-abi-policy.md)
- [ADR-007: コンパイルターゲット整理](ADR-007-targets.md)
- [ADR-014: 安定性ラベル](ADR-014-stability-labels.md)
- [ADR-029: セルフホストネイティブ検証契約](ADR-029-selfhost-native-verification-contract.md)
- [ADR-049: Native C99 Selfhost Executor](ADR-049-native-c99-selfhost-executor.md)
- [RFC-008: native-cpp C99 backend と runtime ABI](../rfcs/008-native-cpp-c99-backend-runtime-abi.md)
- [native-cpp public run promotion plan](../plans/native-cpp-public-run-promotion.md)
- [native-cpp capability registry](../../data/native-cpp-capabilities.toml)
