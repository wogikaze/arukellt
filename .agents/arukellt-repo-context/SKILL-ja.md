---
name: arukellt-repo-context
description: >-
  Arukelltリポジトリで作業する際に、現在のソースオブトゥルースファイル、
  検証コントラクト、および変更を実装またはレビューする前のマークダウン読み込み
  ワークフローが必要な場合にこのスキルを使用します。
---

# arukellt-repo-context

リポジトリ固有の運用ルールが重要な場合、Arukellt作業の開始時にこのスキルを使用します。

## 主要なソースオブトゥルース

関連する場合は以下の順序で読んでください：

1. 現在のユーザーに見える動作: `docs/current-state.md`
2. 現在のオープンワークキュー: `issues/open/index.md`
3. 現在の依存関係順序: `issues/open/dependency-graph.md`
4. 完了した追跡作業: `issues/done/`
5. 設計決定/根拠: `docs/adr/`
6. 検証コントラクト: `scripts/manager.py`
7. 生成されたドキュメントの動作: `scripts/gen/generate-docs.py`

## マークダウンの読み込み

大きなマークダウンファイルの場合は、ファイル全体を読み込むよりも`markdive`を優先します。

```bash
npx markdive dive <file> --depth 2
npx markdive dive <file> --path <section-id> --depth 2
npx markdive read <file> --path <section-id>
```

まず`dive`を使用し、次に`--path`で絞り込み、最後に必要なセクションのみを`read`します。

## 検証

- クイックパス: `python scripts/manager.py verify quick`
- フルパス: `python scripts/manager.py verify`

動作が変更され、生成されたドキュメントまたは問題インデックスに影響がある場合は、以下を実行します：

```bash
python3 scripts/gen/generate-docs.py
python3 scripts/check/check-docs-consistency.py
python3 scripts/gen/generate-issue-index.py
```

## ツールに関する注意事項

- コード検索には`ig`を優先
- 生成されたドキュメントとマニフェスト backed stdlibリファレンスページは再生成する必要があり、手動でメンテナンスしないでください
