# Arukellt Board

Arukellt Board は、issue / ADR / ドキュメントを横断して追跡できる読み取り専用のカンバン SPA です。複数軸切り替え、全文検索、Mermaid 依存グラフ、エージェント向けコピー、n ペイン分割タブを備えています。

## 起動

```bash
# プロダクションビルドを serve（ポート 8765、自動でブラウザを開く）
scripts/run/serve-docs.sh

# Vite dev サーバー（ホットリロード）
scripts/run/serve-docs.sh --dev

# ポートを変更 / ブラウザを開かない
scripts/run/serve-docs.sh -p 9000 --no-open
```

`tools/doc-viewer/` は `tools/board/` に置き換わりました。

## 主な機能

- **カンバン**: 軸を status / orchestration class / track / priority で切り替え、status フィルタと Ready only で絞り込みます。
- **ペインとタブ**: 複数ペインを水平・垂直に分割し、タブをドラッグで移動できます。
- **ドキュメント表示**: Markdown をレンダリングし、Mermaid ダイアグラム、見出しアンカー、内部リンクに対応します。
- **依存グラフ**: 表示中の issue または特定 issue を中心とした Mermaid フローチャートを表示します。
- **コマンドパレット**: `Ctrl+K` で issue / ADR / ドキュメントを即座に開けます。
- **エージェント向けコピー**: issue カードやメニューからパス、絶対パス、参照、ハンドオフ用プロンプトをクリップボードにコピーします。
- **テーマ**: ダーク / ライトを切り替え、localStorage に保存します。

## 技術スタック

- Vite + React + TypeScript
- `marked`（Markdown）、`mermaid`（ダイアグラム）
- Node 製の読み取り専用 API サーバー（`tools/board/server/main.ts`）

## 検証

```bash
cd tools/board
npm run typecheck
npm run build
npm run serve
```

E2E スクリーンショット検証は `tools/board` を起動した上で `/tmp/board-verify/verify.mjs`（playwright-core）を実行します。
