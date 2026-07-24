import type { ReactNode } from "react";

import type { BoardData, IssueRecord } from "../../shared/types";
import { CopyMenu } from "../components/CopyMenu";
import { issueHandoffPrompt } from "../data/agent-handoff";
import { statusColor, unresolvedDeps } from "../data/grouping";
import { useWorkspace } from "../workspace/WorkspaceContext";

/**
 * Board card.
 *
 * Everything on the card answers one of two questions an agent operator asks:
 * "can this be started now?" (deps, readiness, acceptance progress) and "how do
 * I hand it over?" (copy menu).
 */
export function IssueCard({
    issue,
    data,
    byId,
}: {
    issue: IssueRecord;
    data: BoardData;
    byId: Map<string, IssueRecord>;
}): ReactNode {
    const { open, dispatch } = useWorkspace();
    const blockers = unresolvedDeps(issue, byId);
    const progress = issue.acceptance.total
        ? Math.round((issue.acceptance.checked / issue.acceptance.total) * 100)
        : null;

    return (
        <article
            className="card"
            style={{ ["--status-color" as string]: statusColor(issue.status) }}
            tabIndex={0}
            role="button"
            aria-label={`#${issue.id} ${issue.title}`}
            onClick={(event) => open({ kind: "file", path: issue.path }, event)}
            onKeyDown={(event) => {
                if (event.key !== "Enter") return;
                open({ kind: "file", path: issue.path }, event);
            }}
        >
            <div className="card__head">
                <span className="card__id">#{issue.id}</span>
                {issue.priority === null ? null : <span className="chip">P{issue.priority}</span>}
                <div className="card__head-spacer" />
                <button
                    type="button"
                    className="copy-button"
                    title="この issue を中心とした依存グラフを開く"
                    aria-label="依存グラフを開く"
                    onClick={(event) => {
                        event.stopPropagation();
                        dispatch({ type: "open", view: { kind: "graph", focus: issue.id }, target: "active" });
                    }}
                >
                    <GraphIcon />
                </button>
                <CopyMenu
                    compact
                    entries={[
                        { label: "パス", value: () => issue.path, hint: issue.path },
                        { label: "絶対パス", value: () => `${data.repoRoot}/${issue.path}` },
                        { label: "参照 (#id + パス)", value: () => `#${issue.id} \`${issue.path}\`` },
                        {
                            label: "エージェント用プロンプト",
                            value: () => issueHandoffPrompt(issue, data),
                            hint: "識別子・依存・受入条件つき",
                        },
                    ]}
                />
            </div>

            <h3 className="card__title">{issue.title}</h3>
            {issue.summary ? <p className="card__summary">{issue.summary}</p> : null}

            <div className="card__meta">
                {issue.tracks.map((track) => (
                    <span key={track} className="chip">
                        {track}
                    </span>
                ))}
            </div>

            <div className="card__footer">
                {progress === null ? (
                    <span className="card__stat muted">受入条件なし</span>
                ) : (
                    <span className="card__progress" title={`Acceptance criteria ${issue.acceptance.checked}/${issue.acceptance.total}`}>
                        <span className="card__progress-bar">
                            <span className="card__progress-fill" style={{ width: `${progress}%` }} />
                        </span>
                        <span className="card__stat">
                            {issue.acceptance.checked}/{issue.acceptance.total}
                        </span>
                    </span>
                )}
                {blockers.length ? (
                    <span className="card__stat card__stat--blocked" title={blockers.map((dep) => `#${dep.id} ${dep.title}`).join("\n")}>
                        ⛔ {blockers.length} 件の未解決依存
                    </span>
                ) : (
                    <span className="card__stat card__stat--ready">着手可能</span>
                )}
                {issue.blocks.length ? (
                    <span className="card__stat muted" title={issue.blocks.map((id) => `#${id}`).join(", ")}>
                        {issue.blocks.length} 件をブロック
                    </span>
                ) : null}
            </div>
        </article>
    );
}

function GraphIcon(): ReactNode {
    return (
        <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="1.3" aria-hidden="true">
            <circle cx="3.5" cy="8" r="2" />
            <circle cx="12.5" cy="3.5" r="2" />
            <circle cx="12.5" cy="12.5" r="2" />
            <line x1="5.3" y1="7.1" x2="10.7" y2="4.3" />
            <line x1="5.3" y1="8.9" x2="10.7" y2="11.7" />
        </svg>
    );
}
