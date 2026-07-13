---
name: issue-close-review
description: Arukellt issueをopenからdoneへ移す前に、差分、受入条件、検証結果、依存関係、生成物、SKIPを照合してfalse-doneを防ぐ。close判定、完了報告の監査、issue状態変更に使う。通常のコードレビューだけには使わない。
---

# Issue close review

`docs/process/false-done-prevention.md` を基準に、文章ではなく repository evidence で判定する。

## 手順

1. 対象 issue、関連 commit/diff、完了報告、依存 issue を読む。
2. 各 acceptance / DONE_WHEN を、変更ファイル、テスト、生成物、実行結果へ一対一で対応付ける。
3. 次を確認する。
   - 失敗、SKIP、allow-list 追加で未実装を隠していない。
   - current-state、ADR、issue status、ディレクトリが矛盾していない。
   - 生成物は正本から再生成されている。
   - 必須 verification が正規コマンドで成功している。
   - upstream dependency が実際に満たされている。
   - 無関係な変更や未追跡の follow-up が混入していない。
4. 検証記録が古い、対象 commit と違う、または不足している場合は該当コマンドを再実行する。実行不能なら承認しない。
5. 判定は `APPROVE`、`REQUEST_CHANGES`、`BLOCKED` のいずれかにする。曖昧なまま close しない。
6. issue の移動や status 更新は、ユーザーまたは work order が明示的に許可した場合だけ行う。移動後は issue index を generator で再生成し、docs/queue consistency を確認する。

## 報告

- 判定と対象 commit
- acceptance ごとの evidence
- 実行・確認した verification
- blocking finding（重要度順）
- close 可否と残作業

自分が実装した差分でも同じ基準で自己レビューするが、独立レビューが要求される変更を自己承認で代替しない。
