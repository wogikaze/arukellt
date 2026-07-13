---
name: implementation-strategy
description: 言語意味論、公開API、ABI、ターゲット、コンパイラ段階、stdlib移行に関わる変更の実装方針を決める。互換性境界、既存ADR、正本、試験方法を編集前に整理するときに使う。局所的なバグ修正や文言修正だけなら使わない。
---

# Implementation strategy

編集前に、変更を現行契約へ位置付ける。

## 手順

1. `docs/current-state.md` と関連する `docs/data/*.toml` で現行挙動を確認する。
2. `docs/adr/README.md` から関連 ADR を探し、本文のステータスを確認する。`ACCEPTED` だけを拘束力のある決定として扱う。
3. 変更を次のいずれかに分類する。
   - 実装修正: 既存の採択済み契約へ実装を合わせる。
   - 互換性変更: stable/provisional/experimental の扱いを ADR-014 で決める。
   - 未決定の設計: コード編集より先に `$architecture-decision` で判断を記録する。
   - 一時的な実装ギャップ: ADR ではなく issue、`docs/plans/`、`docs/current-state.md` に置く。
4. 正本となる変更先、最小の隣接範囲、回帰試験、docs/生成物への波及を列挙する。
5. 次の現行制約を必要な範囲で適用する。
   - compiler/LSP の正本は `src/compiler/`。退役済み Rust-era 経路を既定にしない。
   - `wasm32-gc` は primary、`wasm32` は supported。WASI は host profile。
   - 公開 API は trait / method / associated function。ユーザー可達 free function は追加しない。
   - Component Model の理想契約と living implementation を混同しない。
6. 実装手順は `docs/process/coding-conventions.md` に合わせる。層の所有、canonical identity、診断/ICE の分離、決定性、テスト配置を逸脱しない。
7. 完了条件を、観測可能な挙動と実行する検証コマンドで定義する。

## 出力

実装方針には次を含める。

- 現行契約と関連 ADR
- 変更の分類と互換性影響
- 編集する正本と変更しない領域
- 回帰試験と検証コマンド
- docs、生成物、migration note への波及
- 未解決の設計判断または blocker

一般的な「専門エージェントの役割分担」や固定パス一覧を再掲しない。実際の変更内容から必要範囲を決める。
