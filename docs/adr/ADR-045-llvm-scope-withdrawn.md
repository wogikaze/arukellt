# ADR-045: 旧 LLVM 役割方針を撤回し、再開まで保留する

ステータス: **SUPERSEDED** — native-cpp の限定判断を ADR-049 が後継する

決定日: 2026-07-11  
廃止: [ADR-005-llvm-scope.md](ADR-005-llvm-scope.md)
後継: [ADR-049-native-c99-selfhost-executor.md](ADR-049-native-c99-selfhost-executor.md)

---

## 文脈

2026-03-24 の ADR-005 は「LLVM IR バックエンドは Wasm 意味論に従属」を採択したが、
同時に `extern "C"` を LLVM 限定とするなど、コア意味論とホスト結合の境界が混在し
矛盾していた。主 Wasm バックエンド自体も未完成であり、native / LLVM の役割を
固定する材料が足りない。

「保留する」こと自体を採択済み判断として記録した（同一ファイルを `DEFERRED` に
書き換えて履歴を消さない）。2026-07-22 に、限定された `native-cpp` セルフホスト
executor の判断を ADR-049 が後継した。以下は保留を採択した時点の歴史的決定である。

---

## 決定

1. **旧 ADR-005 の採択内容は撤回する。** LLVM の役割・意味論従属・最適化方針・
   `extern "C"` のターゲット限定は**未決定**とする。
2. native / LLVM 向けの新規言語機能・ABI・最適化方針は、再開まで設計決定として
   固定しない（scaffold 実装の実験は妨げない）。
3. **再開条件**（すべて満たしたとき `PROPOSED` で再検討）:
   - 主 Wasm ターゲット（`wasm32-gc`）の emit・実行経路が現行 verify ゲートで安定する
   - 言語コア意味論の正本が Wasm 側で文書化され、native との差分を議論できる
   - native / LLVM の必要性を issue または RFC で具体的に再提起する
4. 再開時に必ず決着させる論点:
   - コア言語意味論とホスト結合（ABI 投影）の境界
   - `extern "C"` / Layer 3 FFI をターゲット限定構文とするか、共有 MIR 概念とするか
   - LLVM を「従属再現」に留めるか、独立最適化を許すか

---

## 帰結

- ADR-005 は本 ADR により `SUPERSEDED` とする（旧本文は履歴として残す）。
- ADR-006 の Layer 3 は予約領域に弱め、具体 ABI は本 ADR 再開まで固定しない。

## 関連

- [ADR-005](ADR-005-llvm-scope.md)（廃止記録 + 旧本文）
- [ADR-006](ADR-006-abi-policy.md)
- [ADR-007](ADR-007-targets.md)
- [ADR-049](ADR-049-native-c99-selfhost-executor.md)（後継）
