# ADR-087: 0.x 期の stable stdlib 削除例外（ADR-014 追補）

ステータス: **ACCEPTED** — 0.x に限り stable stdlib の削除窓を明示的例外で短縮し、1.0.0 は再証跡が揃うまで切らない

決定日: 2026-10-26

関連: [ADR-000](ADR-000-process.md)、[ADR-014](ADR-014-stability-labels.md)、[ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-046](ADR-046-free-function-eradication.md)、[#917](../../issues/open/917-stdlib-eradication-execution.md)、[#703](../../issues/open/703-monomorphic-api-bold-cutover.md)

## 文脈

[ADR-014](ADR-014-stability-labels.md) は公開 stdlib エントリに `stable` / `provisional` /
`experimental` / `deprecated` の 4 段階を適用し、`stable` の破壊的変更を「事前告知付きの
移行ガイド経由」に限定した。[ADR-046](ADR-046-free-function-eradication.md) D4 はこれを
削除実行に落とし、`stable` は少なくとも 1 リリースの deprecation（W0009）と migration
guide、削除時期の明示を要求する。`std/manifest.toml` の `[deprecation_policy]` は
`default_remove_in = "1.0.0"` と `minimum_complete_releases = 1` を持ち、現行バージョンは
0.1.0 である。

この組み合わせは、1.0.0 を切らない限り `stable` の削除を実質的に止める。ADR-046 の
根絶実行（[#917](../../issues/open/917-stdlib-eradication-execution.md)）は公開 free
surface を分類し、`stable` の削除対象 64 件と `stable` の free→method 置換 314 件を
残している。1.0.0 を今切ると 378 件の `stable` 公開関数が凍結され、残る 314 件の
free→method 置換の完了が 2.0.0 線まで押し出される。一方、1.0.0 を切るには
release-blocking な 3 項目（fixture harness、selfhost CLI parity、selfhost diag parity）の
再証跡が必要であり、現時点の release readiness は **NOT READY** である。

0.x は semver 上、minor で破壊的変更を許す。しかし ADR-014 の `stable` ラベルと
`remove_in = "1.0.0"` を暗黙に読み替えると、利用者への告知契約が失われる。例外は、
明示的な ADR として記録し、削除ごとの条件を検証できる形でなければならない。

決定時点のスナップショット（非規範。現在の件数は生成された
[eradication scorecard](../stdlib/eradication.md) と `std/manifest.toml` を正とする）:

| 区分 | 件数 | 内訳 |
|------|------|------|
| `stable` 公開関数 | 378 | 下記 64 + 314 |
| `stable` `must delete` | 64 | monomorphic seq/combinator 33、numeric conversions 17、vec intrinsic-compat 10、assertion 4 |
| `stable` `associated-or-method` | 314 | free→method 置換待ち |
| `provisional` `must delete` | 21 | HashMap builtins 20、prelude `filter_i32` 1 |
| `experimental` `must delete` | 2 | `std::wasm::memory_copy` / `memory_fill` |
| `deprecated` `must delete` | 41 | 既存の ADR-014 window |
| `associated-or-method` 合計 | 1129 | replacement-first |

## 決定

### D1: 1.0.0 を今は切らない

現時点で 1.0.0 タグを切らない。1.0.0 は release-blocking な 3 項目（fixture harness、
selfhost CLI parity、selfhost diag parity）の再証跡が揃うまで切らない。本 ADR は
1.0.0 の日付を約束しない。

### D2: 0.x 限定の明示的例外

`project.version` が 0.x である間（0.x 期）に限り、`stable` の公開 stdlib エントリを
ADR-014 の通常の deprecation window より早く削除できる。例外を適用する削除 wave は、
次のすべてを満たさなければならない。

1. **W0009**: 対象 API の利用に対して W0009 が実装され、発火する。
2. **Migration guide**: `docs/stdlib/migration-guidance.md` に対象 API の before/after、
   到達可能な replacement（機能自体を retire する場合はその旨）が記載されている。
3. **Changelog**: `docs/stdlib/changelog.md` に、日付、ADR-087 への参照、対象シンボル、
   削除理由、replacement が記録されている。

この例外は「stdlib 全体が 0.x だから一括で消してよい」という許可ではない。ADR-046 D4 の
「`provisional` は個別判断」「一括 bold cutover は認めない」は維持する。各 wave は対象
シンボルを明示し、上記 3 条件を満たす。例外の記録場所は本 ADR（ADR-014 追補）であり、
ADR-014 本体の決定は変更しない。

### D3: `remove_in` は上限として扱う

0.x 例外の下では、manifest の `remove_in` を「そのバージョンまでに削除する」上限として
扱う。`remove_in = "1.0.0"` の `stable` エントリは、D2 の条件を満たせば 0.x の minor
release で削除してよい。ただし `remove_in` より後に削除することを許すものではない。
`remove_in` の値を 0.x に書き換えて例外を表現しない。例外は本 ADR と changelog に記録する。

### D4: 1.0.0 以降は現行方針

1.0.0 以降は ADR-014 の現行方針をそのまま適用する。本 ADR の例外は 1.0.0 で失効し、
1.x に持ち越さない。`stable` の削除は通常の deprecation window（少なくとも 1 完了
リリース、`remove_in` の earliest-removal 意味）に従う。1.0.0 のタグ後に 0.x 例外を
理由に `stable` を削除してはならない。

### D5: 実行スコープ

本 ADR の採択後に進めるのは次に限る。

- `experimental` の `must delete` 2 件。ADR-014 により `experimental` は直接削除できる。
  migration note を添える。
- `provisional` の `must delete` 21 件。ADR-046 D4 に従い個別判断とし、一括 cutover は
  しない。
- `associated-or-method` 1129 件の replacement 追加。replacement は additive な API
  追加であり、`stable` の削除条件を必要としない。replacement を先に land し、free の
  削除はその後に行う。

`stable` の `must delete` 64 件はこの実行 wave に含めない。これらは D2 の条件を満たした
後の別 wave で削除する。特に 33 件の prelude monomorphic seq/combinator free は、W0009
実装（別 lane）が完了するまで削除しない。314 件の `stable` free→method 置換は
replacement の追加であり、この wave で進めてよい。対応する free の削除は D2 に従う。

## 帰結

- 1.0.0 の凍結を避けつつ、`stable` の根絶を 0.x 内で継続できる。378 件の `stable` を
  2.0.0 まで固定しない。
- 1.0.0 の release readiness を偽って主張しない。3 つの release-blocking check の
  再証跡が揃うまでタグを切らない。
- `stable` の削除コストは利用者に移る。0.x の minor で移行を強制されうるため、W0009、
  migration guide、changelog の 3 点が必須になる。
- ADR-014 の `stable` ラベルは 1.0.0 以降のために維持される。本 ADR はラベルの意味を
  恒久的に変えない。
- `remove_in` の意味は 0.x 例外の間だけ上限に変わる。1.x では earliest-removal に戻る。
- 0.x が長期化した場合、`stable` の互換性保証は実質的に弱くなる。1.0.0 の再検討時には
  この例外の終了を確認する。

## 却下した代替案

- **1.0.0 を今切る**: release-blocking 3 項目の再証跡がなく、378 件の `stable` を凍結し、
  314 件の free→method 置換を 2.0.0 線へ押し出す。採らない。
- **ADR-014 を変更せず、`stable` の削除を 1.0.0 まで待つ**: 1.0.0 を切らない判断と
  両立せず、根絶実行が停止する。採らない。
- **stdlib 全体を `provisional` とみなして一括 cutover する**: ADR-046 D4 が明示的に
  禁止している。採らない。
- **全 `stable` エントリの `remove_in` を 0.x に書き換える**: 個別の告知と削除根拠を
  失い、manifest の lifecycle 契約を破壊する。採らない。
- **`stable` 64 件を本 ADR で列挙して個別 ADR を 64 本作る**: 変更理由が同じ wave に
  対して過剰であり、ADR-000 の「決定だけを残す」に反する。本 ADR の例外 + changelog の
  wave 記録で代替する。

## 再検討条件

- 1.0.0 を切る提案が出たとき（本 ADR の例外を終了し、ADR-014 の通常方針へ戻す）。
- release-blocking 3 項目が再証跡付きで通ったとき（1.0.0 の準備へ移る）。
- `stable` の削除 wave が W0009 / migration guide / changelog のいずれかを欠いたとき
  （削除を止め、本 ADR の条件を再確認する）。
- 0.x 期が長期化し、`stable` の互換性保証が利用者に受け入れられないと判断されたとき。

## 参照

- [ADR-014](ADR-014-stability-labels.md)
- [ADR-046](ADR-046-free-function-eradication.md)
- [ADR-044](ADR-044-trait-method-syntax-adopted.md)
- [ADR-000](ADR-000-process.md)
- [#917](../../issues/open/917-stdlib-eradication-execution.md)
- [#703](../../issues/open/703-monomorphic-api-bold-cutover.md)
- [`std/manifest.toml`](../../std/manifest.toml) `[deprecation_policy]`
- [`docs/data/stdlib-api-eradication.toml`](../data/stdlib-api-eradication.toml)
- [eradication.md](../stdlib/eradication.md)
- [migration-guidance.md](../stdlib/migration-guidance.md)
- [changelog.md](../stdlib/changelog.md)
- [current-state.md](../current-state.md)
