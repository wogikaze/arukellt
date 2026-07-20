---
name: code-change-verification
description: コード、テスト、例、ビルド、生成処理、実行契約を変更した後に、Arukelltの正規検証コマンドを選択して実行し結果を報告する。実装完了時やレビュー前に使う。docsだけの軽微変更はdocs-syncを優先する。
---

# Code change verification

検証コマンドは `docs/data/verification-commands.toml` を正とし、旧 alias や記憶上のコマンドを優先しない。

## 手順

1. `git diff --stat` と `git diff --name-only` で実際の変更範囲を確認する。
2. 対象 issue に `REQUIRED_VERIFICATION` があれば、その正規コマンドを省略しない。
3. 次の対応で最小十分な検証を選ぶ。

   | 変更 | 検証 |
   |---|---|
   | 通常のコード、テスト、例、build/check挙動（編集ループ） | `python3 scripts/manager.py verify lane` |
   | 上記 + 明確な domain gate | `verify lane --gate cli-parity` / `t3` / `fmt-parity` 等 |
   | merge 前・フェーズ完了・CI 相当 | `python3 scripts/manager.py verify quick` |
   | fixture、診断、言語意味論 | `verify lane` + `python3 scripts/manager.py verify fixtures`（merge 前は quick） |
   | selfhost compiler / bootstrap | `verify lane` +該当する `selfhost fixpoint` / `fixture-parity` / `diag-parity` / `parity` |
   | Component emit / WIT / composition | `verify lane --gate component-interop` または component emit gate |
   | docs生成元・分類・リンク | `python3 scripts/manager.py docs regenerate` + `docs check` |
   | benchmark、性能、サイズ | `$benchmark-change` の手順 +該当 perf/size gate |
   | release、広範な基盤変更 | `python3 scripts/manager.py verify full` または `gate local` |

   **禁止:** 並列レーンや 1 編集ごとの完了条件に `verify quick` を既定化しない。

4. active な Rust code を実際に変更し、repo/issue が要求するときだけ Cargo 系検証を追加する。`cargo test --workspace` を全変更の既定にしない。
5. 生成物が変わった場合は、生成元から再生成された差分であることを確認する。
6. 失敗時は最初の有意な失敗、再現コマンド、変更由来か既存失敗かを分けて報告する。未実行を PASS と書かない。
7. 最後に diff を再確認し、検証のための一時ファイルや意図しない baseline 更新を残さない。

## 報告形式

- 変更範囲
- 実行した正確なコマンドと PASS / FAIL
- 未実行の関連ゲートと理由
- 失敗・既知問題・未確認範囲
- 完了条件を満たす根拠
