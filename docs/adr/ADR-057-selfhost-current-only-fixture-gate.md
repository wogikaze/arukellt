# ADR-057: セルフホスト fixture 検証を current-only 並列ゲートへ移行

ステータス: **ACCEPTED** — 通常の fixture ゲートは現行セルフホストを一度だけ使い、全件を並列 compile/validate する

決定日: 2026-09-16

関連: [ADR-029](ADR-029-selfhost-native-verification-contract.md)、[ADR-054](ADR-054-host-linker-and-rust-runtime-retirement.md)、PR #54

## 文脈

ADR-029 は、ピン留め selfhost wasm と現行 selfhost wasm の両方で各 `run:` fixture を
コンパイル・package・実行し、出力を比較する fixture-parity ゲートを定義した。
この比較は Rust compiler から selfhost へ移行する時期には有効だったが、Rust runtime と
host-linker を退役させた現在の通常開発では、各 fixture を二世代分生成して逐次実行する
コストが fixture 検証の律速になっている。

一方、fixture の compile と Wasm validation は全件で維持できる。
実行確認は、コミット済み `.expected` を持つ代表 fixture、各トップレベル領域の代表、
明示的な trap fixture を現行 compiler で実行すれば、通常の変更検出に必要な代表性を保てる。
全件の現行 runtime 実行が必要な場合は、同じ compiler 世代を使う明示的な opt-in を提供できる。

## 決定

1. `selfhost fixture-parity` の公開 CLI 名と manager の入口は維持する。
   ただし通常の意味は、ピン留め対現行の二重比較ではなく current-only fixture test とする。
2. ゲート開始時にピン留め wasm から現行 selfhost compiler を一度だけ準備し、各 `run:`
   fixture をその compiler で一度だけ compile する。
3. 生成された各 Wasm は `wasm-tools validate` で検査する。
   compile と validate は bounded worker pool で並列化し、共有 AST cache への同時書き込みは行わない。
4. 通常実行は各トップレベル領域から一つの smoke fixture を選び、`.expected` を優先する。
   明示的な trap fixture は常に含める。既存 `.expected` が表す意図的な process exit / assertion
   の出力は許可し、期待されていない trap または golden mismatch は失敗にする。
   native-cpp 専用の expected-negative / 数値 exit-status fixture は current P2 の runtime
   対象外として compile/validate のみ行い、専用 native-cpp gate で実行契約を検査する。
5. 全 fixture の現行 runtime 実行が必要な監査では、
   `scripts/run/selfhost-fixture-test.py --execute-all` を使う。
   これは current P2 runtime contract を持つ fixture を一つの current compiler だけで実行し、
   対象外の native-cpp fixture は dedicated gate に委譲する。ピン留め compiler の重複実行は戻さない。
6. ピン留め対現行の出力比較は通常ゲートから外し、
   `scripts/run/selfhost-fixture-test.py --reference` による明示的な bootstrap refresh audit
   として残す。
7. 公式 WASI P2 の component packaging と stock Wasmtime 実行を使う。
   host-linker、Rust compiler、リポジトリ固有の bridge は fixture ゲートへ戻さない。
   `:ro` fixture で bwrap が利用できない場合も writable fallback へ変更せず失敗させる。

## 帰結

- 通常ゲートは fixture ごとの pinned/current compile・package・run を繰り返さず、変更検証の
  compile/validate coverage を保ったまま実行時間と同時実行の余地を改善する。
- current-only には独立 compiler との挙動比較がない。正しさの根拠は `.expected`、explicit trap
  契約、Wasm validation、fixpoint、CLI/diagnostic の各 selfhost ゲートに分担させる。
- `fixture-parity` という既存名は互換性のために残るが、設計上の正確な呼称は fixture test である。
- pinned/current の差分を調査する手段は失われず、reference runner を必要な refresh audit で使える。

## 却下した代替案

- **従来の pinned/current 二重実行を通常ゲートに残す。** 独立比較は強いが、退役済み Rust
  runtime を前提にせずとも同じ compile/package/run を二度行うため、通常の fixture 回帰検証に
  不要な時間と資源を使う。
- **current compiler で全 fixture を逐次 compile・実行する。** 二重生成は避けられるが、
  fixture 数の増加に対して実行時間が線形に膨らみ、全件実行を必要としない通常の変更にも同じ
  コストを課す。
- **compile/validate の対象を smoke fixture だけに縮小する。** 実行は速くなるが、生成物の
  validation coverage を失うため採用しない。

## 再検討条件

独立した仕様由来 oracle を通常ゲートへ導入する場合、または fixture の runtime capability が
変わって代表 smoke では回帰検出を担保できなくなった場合に再検討する。
その際も、current-only の全件 compile/validate coverage と公式 WASI 境界を維持できるかを
比較して判断する。

## 検証

通常の fixture ゲート:

```bash
python3 scripts/manager.py selfhost fixture-parity
```

reference audit:

```bash
python3 scripts/run/selfhost-fixture-test.py --reference
```
