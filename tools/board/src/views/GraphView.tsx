import { useMemo } from "react";
import type { ReactNode } from "react";

import type { IssueRecord } from "../../shared/types";
import { CopyMenu } from "../components/CopyMenu";
import { Mermaid } from "../components/Mermaid";
import { useBoard } from "../data/BoardContext";
import { connectedOnly, dependencyMermaid, neighbourhood } from "../data/agent-handoff";
import { useTheme } from "../app/ThemeContext";
import { useWorkspace } from "../workspace/WorkspaceContext";

/** Beyond three hops the graph stops being readable for this repository's fan-out. */
const DEPTH_OPTIONS = [1, 2, 3];

/**
 * Dependency graph.
 *
 * Two modes: focused on one issue's neighbourhood, or the whole actionable set
 * (open + blocked). Two reductions keep the whole-repo view legible — `done`
 * history is excluded, and issues with no dependency edge are hidden by default
 * because they would otherwise be ~60 unconnected boxes stacked in a column.
 */
export function GraphView({
    paneId,
    tabId,
    focus,
    depth,
    showIsolated,
}: {
    paneId: string;
    tabId: string;
    focus: string | null;
    depth: number;
    showIsolated: boolean;
}): ReactNode {
    const { data, issuesById } = useBoard();
    const { dispatch, open } = useWorkspace();
    const { theme } = useTheme();

    const scope: IssueRecord[] = useMemo(() => {
        if (!data) return [];
        if (focus) return neighbourhood(focus, issuesById, depth);
        const actionable = data.issues.filter((issue) => issue.status === "open" || issue.status === "blocked");
        return showIsolated ? actionable : connectedOnly(actionable);
    }, [data, focus, depth, showIsolated, issuesById]);

    const source = useMemo(() => dependencyMermaid(scope, issuesById, theme), [scope, issuesById, theme]);
    const focusIssue = focus ? issuesById.get(focus) : undefined;

    const setView = (next: { focus?: string | null; depth?: number; showIsolated?: boolean }): void => {
        dispatch({
            type: "replace-view",
            paneId,
            tabId,
            view: {
                kind: "graph",
                focus: next.focus === undefined ? focus : next.focus,
                depth: next.depth ?? depth,
                showIsolated: next.showIsolated ?? showIsolated,
            },
        });
    };

    if (!data) return null;

    return (
        <div className="graph">
            <div className="board__toolbar">
                <div className="segmented" role="group" aria-label="グラフの範囲">
                    <button
                        type="button"
                        className={focus ? "segmented__item" : "segmented__item segmented__item--active"}
                        onClick={() => setView({ focus: null })}
                    >
                        未完了すべて
                    </button>
                    <button
                        type="button"
                        className={focus ? "segmented__item segmented__item--active" : "segmented__item"}
                        disabled={!focus}
                        title={focus ? `#${focus} を中心に表示中` : "カードのグラフボタンから issue を指定"}
                    >
                        {focus ? `#${focus} の周辺` : "issue 指定なし"}
                    </button>
                </div>

                {focus ? (
                    <div className="segmented" role="group" aria-label="探索の深さ">
                        {DEPTH_OPTIONS.map((option) => (
                            <button
                                key={option}
                                type="button"
                                className={option === depth ? "segmented__item segmented__item--active" : "segmented__item"}
                                title={`依存を ${option} ホップまで辿る`}
                                onClick={() => setView({ depth: option })}
                            >
                                {option} hop
                            </button>
                        ))}
                    </div>
                ) : (
                    <button
                        type="button"
                        className={showIsolated ? "toolbar-button toolbar-button--active" : "toolbar-button"}
                        aria-pressed={showIsolated}
                        title="依存関係を持たない issue も表示する"
                        onClick={() => setView({ showIsolated: !showIsolated })}
                    >
                        孤立ノードも表示
                    </button>
                )}

                <div className="board__toolbar-spacer" />
                <span className="board__count">{scope.length} node</span>
                <CopyMenu
                    entries={[
                        { label: "mermaid ソース", value: () => source, hint: "``` mermaid に貼れる形式" },
                        {
                            label: "ノードのパス一覧",
                            value: () => scope.map((issue) => issue.path).join("\n"),
                        },
                        {
                            label: "依存関係の要約",
                            value: () =>
                                scope
                                    .map(
                                        (issue) =>
                                            `#${issue.id} (${issue.status}) depends on: ` +
                                            (issue.dependsOn.length ? issue.dependsOn.map((id) => `#${id}`).join(", ") : "none"),
                                    )
                                    .join("\n"),
                        },
                    ]}
                />
            </div>

            {focusIssue ? (
                <p className="graph__caption">
                    <strong>#{focusIssue.id}</strong> {focusIssue.title} — ノードをクリックすると issue を開きます。
                </p>
            ) : (
                <p className="graph__caption">
                    open / blocked のうち依存関係を持つ issue を表示しています。破線は表示範囲外の issue です。
                </p>
            )}

            {scope.length === 0 ? (
                <div className="empty-state">
                    <p className="empty-state__title">対象の issue がありません</p>
                </div>
            ) : (
                <Mermaid
                    source={source}
                    onNodeClick={(id) => {
                        const issue = issuesById.get(id);
                        if (issue) open({ kind: "file", path: issue.path });
                    }}
                />
            )}
        </div>
    );
}
