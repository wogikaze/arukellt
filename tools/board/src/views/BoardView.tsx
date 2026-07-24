import { useMemo } from "react";
import type { ReactNode } from "react";

import type { IssueStatus } from "../../shared/types";
import { ISSUE_STATUSES } from "../../shared/types";
import { useBoard } from "../data/BoardContext";
import { dependencyMermaid } from "../data/agent-handoff";
import type { BoardAxis, BoardFilters } from "../data/grouping";
import { applyFilters, BOARD_AXES, buildLanes, DEFAULT_FILTERS } from "../data/grouping";
import { CopyMenu } from "../components/CopyMenu";
import { MultiSelect } from "../components/MultiSelect";
import { useTheme } from "../app/ThemeContext";
import { useWorkspace } from "../workspace/WorkspaceContext";
import { IssueCard } from "./IssueCard";

export function BoardView({
    paneId,
    tabId,
    axis,
    filters,
}: {
    paneId: string;
    tabId: string;
    axis: BoardAxis;
    filters: BoardFilters;
}): ReactNode {
    const { data, issuesById } = useBoard();
    const { dispatch } = useWorkspace();
    const { theme } = useTheme();

    const update = (patch: { axis?: BoardAxis; filters?: Partial<BoardFilters> }): void => {
        dispatch({
            type: "replace-view",
            paneId,
            tabId,
            view: { kind: "board", axis: patch.axis ?? axis, filters: { ...filters, ...patch.filters } },
        });
    };

    const visible = useMemo(
        () => applyFilters(data?.issues ?? [], filters, issuesById),
        [data, filters, issuesById],
    );
    const lanes = useMemo(
        () => buildLanes(visible, axis, issuesById, filters.statuses),
        [visible, axis, issuesById, filters.statuses],
    );

    if (!data) return null;

    const uniqueCount = new Set(visible.map((issue) => issue.id)).size;

    return (
        <div className="board">
            <div className="board__toolbar">
                <div className="segmented" role="group" aria-label="レーンの軸">
                    {BOARD_AXES.map((option) => (
                        <button
                            key={option.id}
                            type="button"
                            className={option.id === axis ? "segmented__item segmented__item--active" : "segmented__item"}
                            title={option.hint}
                            aria-pressed={option.id === axis}
                            onClick={() => update({ axis: option.id })}
                        >
                            {option.label}
                        </button>
                    ))}
                </div>

                <div className="board__toolbar-group">
                    {ISSUE_STATUSES.map((status) => {
                        const enabled = filters.statuses.includes(status);
                        return (
                            <button
                                key={status}
                                type="button"
                                className={enabled ? `status-toggle status-toggle--${status}` : "status-toggle"}
                                aria-pressed={enabled}
                                title={`issues/${status} を表示切り替え`}
                                onClick={() =>
                                    update({
                                        filters: {
                                            statuses: enabled
                                                ? filters.statuses.filter((value) => value !== status)
                                                : ([...filters.statuses, status] as IssueStatus[]),
                                        },
                                    })
                                }
                            >
                                {status}
                                <span className="status-toggle__count">{data.stats.issuesByStatus[status]}</span>
                            </button>
                        );
                    })}
                </div>

                <MultiSelect
                    label="Track"
                    options={data.tracks}
                    selected={filters.tracks}
                    onChange={(tracks) => update({ filters: { tracks } })}
                />
                <MultiSelect
                    label="Orchestration"
                    options={data.orchestrationClasses}
                    selected={filters.orchestrationClasses}
                    onChange={(orchestrationClasses) => update({ filters: { orchestrationClasses } })}
                />

                <button
                    type="button"
                    className={filters.onlyReady ? "toolbar-button toolbar-button--active" : "toolbar-button"}
                    aria-pressed={filters.onlyReady}
                    title="依存がすべて解決済みの open issue のみ表示"
                    onClick={() => update({ filters: { onlyReady: !filters.onlyReady } })}
                >
                    Ready only
                </button>

                <input
                    type="search"
                    className="board__filter-input"
                    placeholder="カードを絞り込み (#714 / 文字列)"
                    value={filters.query}
                    onChange={(event) => update({ filters: { query: event.target.value } })}
                />

                <div className="board__toolbar-spacer" />

                <span className="board__count" title="フィルタ後の issue 件数">
                    {uniqueCount} issue / {lanes.length} lane
                </span>

                <button
                    type="button"
                    className="toolbar-button"
                    title="表示中の issue で依存グラフを開く"
                    onClick={() => dispatch({ type: "open", view: { kind: "graph", focus: null }, target: "split-right" })}
                >
                    Graph →
                </button>

                <CopyMenu
                    entries={[
                        {
                            label: "表示中の issue 一覧",
                            value: () =>
                                lanes
                                    .filter((lane) => lane.issues.length)
                                    .map(
                                        (lane) =>
                                            `## ${lane.label}\n` +
                                            lane.issues.map((issue) => `- #${issue.id} ${issue.title} — \`${issue.path}\``).join("\n"),
                                    )
                                    .join("\n\n"),
                            hint: "Markdown リスト",
                        },
                        {
                            label: "表示中の依存グラフ (mermaid)",
                            value: () => dependencyMermaid(visible, issuesById, theme),
                        },
                        {
                            label: "表示中のパス一覧",
                            value: () => [...new Set(visible.map((issue) => issue.path))].join("\n"),
                        },
                    ]}
                />

                {isFiltered(filters) ? (
                    <button
                        type="button"
                        className="toolbar-button"
                        title="フィルタを初期状態に戻す"
                        onClick={() => update({ filters: DEFAULT_FILTERS })}
                    >
                        Reset
                    </button>
                ) : null}
            </div>

            {lanes.length === 0 ? (
                <div className="empty-state">
                    <p className="empty-state__title">条件に一致する issue がありません</p>
                    <p>ステータスの絞り込みか検索語を見直してください。</p>
                </div>
            ) : (
                <div className="board__lanes">
                    {lanes.map((lane) => (
                        <section key={lane.id} className="lane" style={{ ["--lane-color" as string]: lane.color }}>
                            <header className="lane__header" title={lane.hint}>
                                <span className="lane__title">{lane.label}</span>
                                <span className="lane__count">{lane.issues.length}</span>
                            </header>
                            <div className="lane__cards">
                                {lane.issues.map((issue) => (
                                    <IssueCard key={`${lane.id}:${issue.id}`} issue={issue} data={data} byId={issuesById} />
                                ))}
                                {lane.issues.length === 0 ? <p className="lane__empty">—</p> : null}
                            </div>
                        </section>
                    ))}
                </div>
            )}
        </div>
    );
}

function isFiltered(filters: BoardFilters): boolean {
    return (
        filters.query !== "" ||
        filters.onlyReady ||
        filters.tracks.length > 0 ||
        filters.orchestrationClasses.length > 0 ||
        filters.statuses.length !== DEFAULT_FILTERS.statuses.length ||
        !filters.statuses.every((status) => DEFAULT_FILTERS.statuses.includes(status))
    );
}
