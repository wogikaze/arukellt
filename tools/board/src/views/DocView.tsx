import { useCallback, useEffect, useState } from "react";
import type { ReactNode } from "react";

import type { DocRecord, FileContent, IssueRecord } from "../../shared/types";
import { fetchFile } from "../api/client";
import { CopyMenu } from "../components/CopyMenu";
import { Markdown } from "../components/Markdown";
import type { Heading } from "../components/markdown";
import { useCopy } from "../components/Toast";
import { useBoard } from "../data/BoardContext";
import { docHandoffPrompt, issueHandoffPrompt } from "../data/agent-handoff";
import { statusColor } from "../data/grouping";
import { docKindLabel, docStatusColor } from "../workspace/tab-labels";
import { useWorkspace } from "../workspace/WorkspaceContext";

export function DocView({ path }: { path: string }): ReactNode {
    const { data, issuesById } = useBoard();
    const { open } = useWorkspace();
    const copy = useCopy();
    const [file, setFile] = useState<FileContent | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [headings, setHeadings] = useState<Heading[]>([]);

    useEffect(() => {
        const controller = new AbortController();
        setFile(null);
        setError(null);
        setHeadings([]);
        fetchFile(path, controller.signal)
            .then(setFile)
            .catch((cause: unknown) => {
                if (controller.signal.aborted) return;
                setError(cause instanceof Error ? cause.message : String(cause));
            });
        return () => controller.abort();
    }, [path]);

    const onOpenPath = useCallback(
        (target: string, event: React.MouseEvent) => open({ kind: "file", path: target }, event),
        [open],
    );
    const onHeadings = useCallback((next: Heading[]) => setHeadings(next), []);

    if (error) {
        return (
            <div className="empty-state">
                <p className="empty-state__title">読み込めませんでした</p>
                <p className="text-mono">{path}</p>
                <p>{error}</p>
            </div>
        );
    }
    if (!file || !data) {
        return (
            <div className="empty-state">
                <div className="spinner" />
            </div>
        );
    }

    const copyEntries = [
        { label: "パス", value: () => file.path, hint: file.path },
        { label: "絶対パス", value: () => file.absolutePath },
        { label: "本文 (Markdown)", value: () => file.text },
        ...(file.issue ? [{ label: "エージェント用プロンプト", value: () => issueHandoffPrompt(file.issue as IssueRecord, data) }] : []),
        ...(file.doc ? [{ label: "エージェント用コンテキスト", value: () => docHandoffPrompt(file.doc as DocRecord, data) }] : []),
    ];

    return (
        <div className="doc">
            <header className="doc__header">
                <div className="doc__identity">
                    {file.issue ? (
                        <span className="badge" style={{ ["--status-color" as string]: statusColor(file.issue.status) }}>
                            #{file.issue.id} {file.issue.status}
                        </span>
                    ) : null}
                    {file.doc ? (
                        <>
                            <span className="chip chip--accent">{docKindLabel(file.doc.kind)}</span>
                            <span className="badge" style={{ ["--status-color" as string]: docStatusColor(file.doc.status) }}>
                                {file.doc.status}
                            </span>
                        </>
                    ) : null}
                    <button
                        type="button"
                        className="doc__path text-mono"
                        title="クリックでパスをコピー"
                        onClick={() => copy(file.path, "パス")}
                    >
                        {file.path}
                    </button>
                </div>
                <div className="doc__actions">
                    {file.issue ? (
                        <button
                            type="button"
                            className="toolbar-button"
                            onClick={(event) =>
                                open({ kind: "graph", focus: (file.issue as IssueRecord).id }, event)
                            }
                        >
                            依存グラフ
                        </button>
                    ) : null}
                    <CopyMenu entries={copyEntries} />
                </div>
            </header>

            {file.issue ? <IssueMeta issue={file.issue} byId={issuesById} /> : null}
            {file.doc && (file.doc.relatedDocs.length || file.doc.relatedIssues.length) ? (
                <DocRelations doc={file.doc} />
            ) : null}

            <div className="doc__body">
                <div className="doc__content">
                    <Markdown text={file.text} sourcePath={file.path} onOpenPath={onOpenPath} onHeadings={onHeadings} />
                </div>
                {headings.length > 2 ? <Outline headings={headings} /> : null}
            </div>
        </div>
    );
}

function IssueMeta({ issue, byId }: { issue: IssueRecord; byId: Map<string, IssueRecord> }): ReactNode {
    const { open } = useWorkspace();
    const progress = issue.acceptance.total
        ? Math.round((issue.acceptance.checked / issue.acceptance.total) * 100)
        : 0;

    const renderRefs = (ids: string[]): ReactNode => {
        if (!ids.length) return <span className="muted">なし</span>;
        return ids.map((id) => {
            const target = byId.get(id);
            return (
                <button
                    key={id}
                    type="button"
                    className="ref-chip"
                    style={{ ["--status-color" as string]: target ? statusColor(target.status) : "var(--fg-subtle)" }}
                    title={target ? `${target.title} (${target.status})` : "リポジトリ内に該当 issue なし"}
                    disabled={!target}
                    onClick={(event) => target && open({ kind: "file", path: target.path }, event)}
                >
                    <span className="status-dot" />#{id}
                </button>
            );
        });
    };

    return (
        <dl className="meta-grid">
            <div className="meta-grid__cell">
                <dt>Track</dt>
                <dd>
                    {issue.tracks.map((track) => (
                        <span key={track} className="chip">
                            {track}
                        </span>
                    ))}
                </dd>
            </div>
            <div className="meta-grid__cell">
                <dt>Orchestration</dt>
                <dd>
                    <span className="chip">{issue.orchestrationClass}</span>
                    {issue.orchestrationUpstream ? <span className="muted"> ← {issue.orchestrationUpstream}</span> : null}
                </dd>
            </div>
            <div className="meta-grid__cell">
                <dt>Depends on</dt>
                <dd className="meta-grid__refs">{renderRefs(issue.dependsOn)}</dd>
            </div>
            <div className="meta-grid__cell">
                <dt>Blocks</dt>
                <dd className="meta-grid__refs">{renderRefs(issue.blocks)}</dd>
            </div>
            <div className="meta-grid__cell">
                <dt>Acceptance</dt>
                <dd>
                    <span className="card__progress">
                        <span className="card__progress-bar">
                            <span className="card__progress-fill" style={{ width: `${progress}%` }} />
                        </span>
                        <span className="card__stat">
                            {issue.acceptance.checked}/{issue.acceptance.total}
                        </span>
                    </span>
                </dd>
            </div>
            <div className="meta-grid__cell">
                <dt>Updated</dt>
                <dd className="text-mono">{issue.updated || "—"}</dd>
            </div>
        </dl>
    );
}

function DocRelations({ doc }: { doc: DocRecord }): ReactNode {
    const { data } = useBoard();
    const { open } = useWorkspace();

    const openDoc = (id: string, event: React.MouseEvent): void => {
        const target = data?.docs.find((candidate) => candidate.id === id);
        if (target) open({ kind: "file", path: target.path }, event);
    };
    const openIssue = (id: string, event: React.MouseEvent): void => {
        const target = data?.issues.find((candidate) => candidate.id === id);
        if (target) open({ kind: "file", path: target.path }, event);
    };

    return (
        <dl className="meta-grid">
            {doc.relatedDocs.length ? (
                <div className="meta-grid__cell">
                    <dt>関連ドキュメント</dt>
                    <dd className="meta-grid__refs">
                        {doc.relatedDocs.map((id) => (
                            <button key={id} type="button" className="ref-chip" onClick={(event) => openDoc(id, event)}>
                                {id}
                            </button>
                        ))}
                    </dd>
                </div>
            ) : null}
            {doc.relatedIssues.length ? (
                <div className="meta-grid__cell">
                    <dt>参照 issue</dt>
                    <dd className="meta-grid__refs">
                        {doc.relatedIssues.slice(0, 24).map((id) => (
                            <button key={id} type="button" className="ref-chip" onClick={(event) => openIssue(id, event)}>
                                #{id}
                            </button>
                        ))}
                    </dd>
                </div>
            ) : null}
        </dl>
    );
}

function Outline({ headings }: { headings: Heading[] }): ReactNode {
    return (
        <nav className="outline" aria-label="見出し">
            <p className="outline__title">目次</p>
            {headings.map((heading) => (
                <a
                    key={heading.id}
                    className={`outline__item outline__item--h${heading.level}`}
                    href={`#${heading.id}`}
                    onClick={(event) => {
                        event.preventDefault();
                        document.getElementById(heading.id)?.scrollIntoView({ behavior: "smooth", block: "start" });
                    }}
                >
                    {heading.text}
                </a>
            ))}
        </nav>
    );
}
