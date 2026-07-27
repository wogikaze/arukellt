import { useCallback, useEffect, useState } from "react";
import type { ReactNode } from "react";

import type { DocRecord, FileContent, IssueRecord } from "../../shared/types";
import { fetchFile } from "../api/client";
import { CopyMenu } from "../components/CopyMenu";
import { Markdown } from "../components/Markdown";
import { decodeHtmlEntities, type Heading } from "../components/markdown";
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

    const { frontmatter, body } = parseFrontmatter(file.text);

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

            {Object.keys(frontmatter).length ? <FrontmatterPanel frontmatter={frontmatter} byId={issuesById} /> : null}
            {file.issue ? <IssueMeta issue={file.issue} byId={issuesById} /> : null}
            {file.doc && (file.doc.relatedDocs.length || file.doc.relatedIssues.length) ? (
                <DocRelations doc={file.doc} />
            ) : null}

            <div className="doc__body">
                <div className="doc__content">
                    <Markdown text={body} sourcePath={file.path} onOpenPath={onOpenPath} onHeadings={onHeadings} />
                </div>
                {headings.length > 2 ? <Outline headings={headings} /> : null}
            </div>
        </div>
    );
}

interface Frontmatter {
    frontmatter: Record<string, string>;
    body: string;
}

function parseFrontmatter(text: string): Frontmatter {
    if (!text.startsWith("---\n")) return { frontmatter: {}, body: text };
    const end = text.indexOf("\n---", 4);
    if (end === -1) return { frontmatter: {}, body: text };

    const block = text.slice(4, end);
    const body = text.slice(end + 4).replace(/^\r?\n/, "");
    const frontmatter: Record<string, string> = {};
    let lastKey = "";
    for (const line of block.split("\n")) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith("#")) continue;
        if (/^\s/.test(line) && lastKey) {
            frontmatter[lastKey] = `${frontmatter[lastKey]} ${trimmed}`.trim();
            continue;
        }
        const colon = line.indexOf(":");
        if (colon === -1) continue;
        lastKey = line.slice(0, colon).trim();
        const value = line.slice(colon + 1).trim();
        frontmatter[lastKey] = decodeHtmlEntities(value.replace(/^['"](.*)['"]$/s, "$1"));
    }
    return { frontmatter, body };
}

const REF_ID = /#?(\d{1,4})([a-z])?\b/g;
const ADR_OR_RFC = /^(adr|rfc)[-\s]?(\d+)/i;

function splitRefTokens(raw: string): string[] {
    return raw
        .split(/[,、/]|\s+and\s+/)
        .map((token) => token.trim())
        .filter(Boolean);
}

function FrontmatterPanel({
    frontmatter,
    byId,
}: {
    frontmatter: Record<string, string>;
    byId: Map<string, IssueRecord>;
}): ReactNode {
    const { open } = useWorkspace();

    const renderChips = (raw: string): ReactNode => {
        const tokens = splitRefTokens(raw);
        if (!tokens.length) return <span className="muted">なし</span>;
        return tokens.map((token) => {
            const adrRfc = ADR_OR_RFC.exec(token);
            if (adrRfc) {
                const kind = adrRfc[1].toUpperCase();
                const id = adrRfc[2];
                return (
                    <span key={token} className="chip">
                        {kind}-{id}
                    </span>
                );
            }
            REF_ID.lastIndex = 0;
            const match = REF_ID.exec(token);
            if (!match) return <span key={token} className="chip">{token}</span>;
            const id = match[1] + (match[2] ?? "");
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

    const renderValue = (key: string, raw: string): ReactNode => {
        const lower = key.toLowerCase();
        if (lower === "track" || lower === "tracks") {
            const tracks = raw
                .split(/[,、/]|\s+and\s+/)
                .map((t) => t.trim().toLowerCase())
                .filter(Boolean);
            return tracks.length
                ? tracks.map((track) => (
                      <span key={track} className="chip">
                          {track}
                      </span>
                  ))
                : null;
        }
        if (["depends on", "depends_on", "depends", "blocked by", "blocked_by", "blocks", "related"].includes(lower)) {
            return <span className="meta-grid__refs">{renderChips(raw)}</span>;
        }
        if (lower === "priority") return <span className="chip">P{raw.replace(/\D/g, "") || raw}</span>;
        if (lower === "status") {
            const color = statusColor(raw as IssueRecord["status"]);
            return (
                <span className="badge" style={{ ["--status-color" as string]: color }}>
                    {raw}
                </span>
            );
        }
        if (["created", "updated", "decided on", "decided_on", "date"].includes(lower)) {
            return <span className="text-mono">{raw}</span>;
        }
        return <span>{raw}</span>;
    };

    const keys = Object.keys(frontmatter);
    if (!keys.length) return null;

    return (
        <dl className="meta-grid frontmatter-panel">
            {keys.map((key) => (
                <div key={key} className="meta-grid__cell">
                    <dt>{key}</dt>
                    <dd>{renderValue(key, frontmatter[key])}</dd>
                </div>
            ))}
        </dl>
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
