# ADR-056: CoreHIR 本体 forest の所有権移譲境界

ステータス: **ACCEPTED** — CoreHIR 本体は明示的な破壊的 ownership transfer で一度だけ移譲する

決定日: 2026-09-15

関連: [ADR-029](ADR-029-selfhost-native-verification-contract.md)、[ADR-053](ADR-053-selfhost-compiler-core-rewrite.md)

## 文脈

`CoreHirBodyTable` は、builder が構築し validator が検査した CoreHIR 本体を
freeze 境界まで運ぶ artifact である。
以前の `*_ref` accessor は table 内の `Vec` をそのまま返し、呼び出し側が参照を
保持したまま元 table の body storage を空にしていた。
これは deep copy を避ける一方で、detached snapshot を要求する boundary contract と
storage alias を禁止する checker に衝突する。

本体全体を deep copy すると、selfhost の長寿命 GC graph とピークメモリを増やす。
したがって必要なのは copy の追加ではなく、共有参照に見える API をなくし、所有権の
移動を API の名前と戻り値で表すことである。

## 決定

1. `CoreHirBodyTable` から本体を読むための `*_ref` accessor は設けない。
2. 本体 forest の移譲は `corehir_body_table_take_body_forest` に一本化する。
   戻り値は `CoreHirBodyForest` とし、`exprs`、function body roots、method body roots を
   同じ所有権単位で保持する。
3. この API は destructive である。呼び出し後、table の body vectors は空になり、
   以後その table から本体を読むことはできない。table に残るのは schema、契約情報、
   および scalar metadata である。
4. 移譲後の forest は、受け取った `CoreHirMirBodySource` または lazy lowering の
   owner が保持する。body table と forest の間に同時に有効な body storage alias を
   作らない。
5. 表示用など本当に独立した snapshot が必要な箇所では、明示的な detached copy API
   を使う。destructive transfer を read accessor の代わりに隠してはならない。
6. boundary checker はこの所有権契約を検査し、実装に合わせて弱めない。

## 帰結

- body forest は deep copy なしで次の lowering 段階へ移るため、hot path の追加メモリと
  GC graph を抑えられる。
- `take` の後に table body を参照するコードは契約違反として検出できる。
- caller は移譲後の forest の lifetime と cleanup を owner として引き受ける。
- builder、validator、freeze artifact の順序は維持され、table の metadata contract は
  body storage の移譲によって失われない。

## 却下した代替案

- **既存の `*_ref` 名を残して checker だけ弱める。** read accessor に見える名前と
  destructive な実態の不一致を温存し、alias を見落としやすくするため採用しない。
- **body forest 全体を毎回 deep copy する。** 所有権は明確になるが、selfhost のピーク
  RSS と GC graph を不必要に増やすため採用しない。
- **各 vector を別々の transfer API にする。** forest 全体の一貫した移譲を caller が
  組み立てる必要があり、部分移譲状態を作るため採用しない。

## 再検討条件

CoreHIR body artifact の schema または lowering の lifetime が変わり、body forest を
同一 ownership unit として移譲できなくなった場合に再検討する。
その場合も、alias を隠す read accessor ではなく、storage の共有またはコピーを明示する
新しい ADR と boundary contract を先に採択する。
