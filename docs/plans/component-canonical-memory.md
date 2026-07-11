# Component Canonical ABI 一時メモリ（現行実装）

ステータス: 実装計画 / 現行挙動メモ（決定記録ではない）  
関連 ADR: [ADR-008](../adr/ADR-008-component-wrapping.md)

ADR-008 は「canonical ABI 用の一時領域と再確保契約を in-tree 実装が管理する」までを
契約とする。具体的なページ数・offset・bump 戦略はここに置く。

---

## 現行実装（変わりうる）

- Linear memory **1 page** のうち offset **256–65535** を canonical ABI スクラッチに使用
- Per-call bump（呼び出し毎リセット）
- 大きな文字列・リストは現行上限に注意

## 既知の限界（契約化しない）

次は現行戦略の限界であり、恒久 ABI ではない:

- 約 64KiB を超える文字列や list
- nested / reentrant call
- async / future
- 複数の同時 lift/lower
- allocator との共存

再設計時は本ファイルと `docs/current-state.md` を更新し、ADR-008 本体は改訂不要とする
（原則が変わらない限り）。
