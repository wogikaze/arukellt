import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

import type { SearchResponse } from "../../shared/types";
import { fetchSearch } from "../api/client";
import { docStatusColor } from "../workspace/tab-labels";
import { statusColor } from "../data/grouping";
import { useWorkspace } from "../workspace/WorkspaceContext";
import type { IssueStatus } from "../../shared/types";

const DEBOUNCE_MS = 180;

/**
 * Mark occurrences of the query inside an excerpt. Split on the lowercased
 * text so the original casing survives, which matters for identifiers.
 */
function Highlighted({ text, needle }: { text: string; needle: string }): ReactNode {
    const target = needle.trim().toLowerCase();
    if (!target) return text;

    const parts: ReactNode[] = [];
    const haystack = text.toLowerCase();
    let cursor = 0;
    while (cursor < text.length) {
        const at = haystack.indexOf(target, cursor);
        if (at === -1) break;
        if (at > cursor) parts.push(text.slice(cursor, at));
        parts.push(
            <mark key={at} className="search__mark">
                {text.slice(at, at + target.length)}
            </mark>,
        );
        cursor = at + target.length;
    }
    parts.push(text.slice(cursor));
    return parts;
}

export function SearchView({ paneId, tabId, query }: { paneId: string; tabId: string; query: string }): ReactNode {
    const { dispatch, open } = useWorkspace();
    const [result, setResult] = useState<SearchResponse | null>(null);
    const [isSearching, setSearching] = useState(false);
    const inputRef = useRef<HTMLInputElement>(null);

    useEffect(() => {
        inputRef.current?.focus();
    }, []);

    useEffect(() => {
        if (query.trim().length < 2) {
            setResult(null);
            return;
        }
        const controller = new AbortController();
        setSearching(true);
        const timer = setTimeout(() => {
            fetchSearch(query, controller.signal)
                .then(setResult)
                .catch(() => undefined)
                .finally(() => {
                    if (!controller.signal.aborted) setSearching(false);
                });
        }, DEBOUNCE_MS);
        return () => {
            clearTimeout(timer);
            controller.abort();
        };
    }, [query]);

    return (
        <div className="search">
            <div className="search__bar">
                <input
                    ref={inputRef}
                    type="search"
                    className="search__input"
                    placeholder="issue と ADR/RFC/plan の全文検索（2 文字以上）"
                    value={query}
                    onChange={(event) =>
                        dispatch({ type: "replace-view", paneId, tabId, view: { kind: "search", query: event.target.value } })
                    }
                />
                {isSearching ? <div className="spinner" /> : null}
                {result ? <span className="board__count">{result.total} 件</span> : null}
            </div>

            {!result ? (
                <div className="empty-state">
                    <p className="empty-state__title">検索語を入力してください</p>
                    <p>識別子でそのまま探せます: `wasm32-gc`、`ADR-013`、`714`</p>
                </div>
            ) : (
                <div className="search__results">
                    {result.hits.map((hit) => (
                        <button
                            key={hit.path}
                            type="button"
                            className="search__hit"
                            style={{
                                ["--status-color" as string]:
                                    hit.kind === "issue"
                                        ? statusColor(hit.status as IssueStatus)
                                        : docStatusColor(hit.status),
                            }}
                            onClick={(event) => open({ kind: "file", path: hit.path }, event)}
                        >
                            <span className="search__hit-head">
                                <span className="status-dot" />
                                <span className="search__hit-title">{hit.title}</span>
                                <span className="search__hit-path text-mono">{hit.path}</span>
                            </span>
                            {hit.excerpts.map((excerpt) => (
                                <span key={excerpt.line} className="search__excerpt">
                                    <span className="search__excerpt-line">{excerpt.line}</span>
                                    <span className="search__excerpt-text">
                                        <Highlighted text={excerpt.text} needle={result.query} />
                                    </span>
                                </span>
                            ))}
                        </button>
                    ))}
                    {result.hits.length === 0 ? (
                        <div className="empty-state">
                            <p className="empty-state__title">一致するファイルがありません</p>
                        </div>
                    ) : null}
                </div>
            )}
        </div>
    );
}
