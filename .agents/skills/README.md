# Arukellt repository skills

現行の Agent Skills 仕様と Codex の repo-local discovery に合わせ、スキルは `.agents/skills/<name>/SKILL.md` に置く。本文は日本語を正本とし、`SKILL-ja.md` の二重管理は行わない。

## 残したスキル

| Skill | 用途 |
|---|---|
| `implementation-strategy` | 互換性・ADR・正本・検証を編集前に整理 |
| `architecture-decision` | ADR/RFC/plan/research の分類と ADR 作成 |
| `code-change-verification` | 変更範囲に応じた正規検証 |
| `docs-sync` | docs・構造化データ・生成物の同期 |
| `benchmark-change` | benchmark / baseline / perf gate |
| `issue-close-review` | false-done 防止と issue close 判定 |

## 統合・削除した旧スキル

- `arukellt-repo-context`: 常時必要な repo 規則なので `AGENTS.md` へ統合。
- `acceptance-slice-implementer` と全 `impl-*`: coding agent の通常能力と重複する persona/担当レーンを削除。必要な事前判断は `implementation-strategy`、完了検証は `code-change-verification` が担う。
- `design-language` / `design-selfhost-mir` / `design-stdlib`: 文書の種類ではなく意思決定ワークフローとして `architecture-decision` へ統合。
- `reviewer` / `verify`: review と verification の人工的な分離を廃止し、コード検証と issue close 判定へ分け直した。
- `impl-benchmark`: `benchmark-change` へ置換。
- `impl-selfhost-retirement`: ADR-029 後の移行期専用 persona であり、常設 skill から削除。

各 skill は相互に長い必須チェーンを作らず、必要な発火条件だけを `AGENTS.md` に置く。
