# native-cpp general backend readiness

ステータス: 実行計画（決定記録ではない）
関連 ADR: [ADR-050](../adr/ADR-050-experimental-public-native-c99-run.md)、[ADR-049](../adr/ADR-049-native-c99-selfhost-executor.md)
公開 run 昇格（完了）: [native-cpp-public-run-promotion.md](native-cpp-public-run-promotion.md)
作成日: 2026-07-25

---

## 1. 評価（暫定 → v2 で置換）

**公開 native run は動作し、保証された subset（`native_cpp_public` 16）では実用可能。全 fixture 木の v1 計測は compile 56% / run 43%（`fn main(` 絞り込み・exit 0 基準）で、ICE 88・crash 83 が残るため一般用途の native backend には未到達。v1 数字は暫定・未分類であり、正式指標は measure v2 取得後に置き換える。**

| 用途 | 判定 |
|------|------|
| `native_cpp_public` で保証された 16 機能 | 実用可能 |
| 対応機能が既知の自作プログラム | 条件付きで実用可能 |
| 全 fixture に近い一般的な Ark コード | 未実用 |
| 未対応機能の安全な拒否 | おおむね成立（safe reject 条件で精査） |
| compiler robustness | ICE が残る間は不足 |
| runtime robustness | unexpected crash 精査前は不足 |
| production | 不可 |

## 2. 正式指標（measure v2）

| 指標 | 定義 |
|------|------|
| positive compile pass | compile 成功が期待される positive のうち compile 成功 |
| positive semantic run pass | positive のうち stdout/stderr/exit/signal が期待値と一致 |
| compiled-positive semantic run pass | compile 成功 positive のうち実行結果が期待値と一致 |
| expected-negative diagnostic pass | compile 失敗だけでなく diagnostic 内容も期待値と一致 |
| unexpected ICE | 入力種別を問わず compiler ICE（最終 gate: `ice_total == 0`） |
| unexpected crash | 期待結果と一致しない signal 終了・runtime 異常 |

母集団は manifest 全体から分類する（`fn main(` だけで全件を絞らない）。

- `positive_run`: kind ∈ {`run`,`module-run`,`t3-run`} かつ native-cpp entry point あり
- `expected_negative`: kind ∈ {`compile-error`,`diag`,`module-diag`,`component-world-error`}（main 不問）
- `other`: その他・未登録・対象外

## 3. 目標値

- [x] unexpected ICE（全母集団）: `0`
- [x] unexpected crash: `0`
- [x] expected-negative diagnostic match: `100%` in-scope（2026-07-26: 107/107; 14 fixtures documented in `docs/data/native-cpp-expected-negative-limitations.toml`; including limitations 107/121 ≈ 88.4%）
- [x] positive compile: `≥80%`（2026-07-26: 1349/1674 = 80.59%）
- [x] compiled-positive semantic run: `≥95%`（96.44%）
- [x] public corpus: `100%` 維持（16/16）
- [x] Wasm/native parity: `100%` 維持（hostable subset; HOF は native public corpus で担保）

## 4. 実行順

- [x] 評価文言を暫定値として同期（Phase 0）
- [x] manifest 起点の measure v2
- [x] expected outcome との semantic 判定
- [x] v2 baseline と ratchet 導入（canonical receipt + `docs/data/native-cpp-fixture-coverage-baseline.json`）
- [x] unexpected crash cluster を 0 へ
- [x] ICE を全母集団で 0 へ
- [x] capability reject 上位を実装（HashMap / Range / assert / sort 等; 残差あり）
- [x] positive compile 80% / compiled-positive run 95%
- [x] docs と release guarantee の評価文言を昇格（general readiness COMPLETE; still not production-ready / support_tier remains scaffold）

## 5. Ratchet（悪化禁止）

v2 baseline 固定直後:

```text
unexpected ICE <= baseline ICE
unexpected crash <= baseline crash
positive compile rate >= baseline rate
compiled-positive semantic run rate >= baseline rate
```

0 になった項目から `<= 0` へ固定する。目標値 hard gate は段階的。

## 6. コマンド

```text
ARUKELLT_BUILD_DIR=$PWD/.build-native-recovery \
  python3 scripts/check/measure-native-cpp-fixture-coverage.py --write

python3 scripts/check/check-native-cpp-fixture-readiness-ratchet.py
```

証拠:

- v1 暫定: `docs/data/native-cpp-fixture-coverage-receipt.json`（schema v1）
- v2 正式: 同パスを schema v2 で上書きし、baseline は
  `docs/data/native-cpp-fixture-coverage-baseline.json`
