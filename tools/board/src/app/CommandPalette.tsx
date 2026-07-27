import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";

import { useBoard } from "../data/BoardContext";
import { useEscapeKey } from "../components/Toast";
import { docKindLabel, docStatusColor } from "../workspace/tab-labels";
import { statusColor } from "../data/grouping";
import { useWorkspace } from "../workspace/WorkspaceContext";
import type { TabView } from "../workspace/types";

interface Command {
    id: string;
    label: string;
    detail: string;
    color: string;
    /** Lowercased haystack; precomputed because the list has ~900 entries. */
    haystack: string;
    run: (event: { metaKey: boolean; ctrlKey: boolean; shiftKey: boolean }) => void;
}

const MAX_RESULTS = 40;

/**
 * Ctrl/Cmd+K palette over actions, issues and documents.
 *
 * Ranking is prefix/word-boundary aware rather than plain substring so that
 * typing `714` or `adr-13` lands on the intended record instead of the first
 * file that merely mentions it.
 */
export function CommandPalette({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }): ReactNode {
    const { data, reload } = useBoard();
    const { state, dispatch, open } = useWorkspace();
    const [query, setQuery] = useState("");
    const [cursor, setCursor] = useState(0);
    const inputRef = useRef<HTMLInputElement>(null);
    const listRef = useRef<HTMLDivElement>(null);

    useEscapeKey(isOpen, onClose);
    useEffect(() => {
        if (!isOpen) return;
        setQuery("");
        setCursor(0);
        inputRef.current?.focus();
    }, [isOpen]);

    const commands = useMemo<Command[]>(() => {
        const openView = (view: TabView) => (event: { metaKey: boolean; ctrlKey: boolean; shiftKey: boolean }) =>
            open(view, event);

        const actions: Command[] = [
            { id: "act:board", label: "Board を開く", detail: "カンバン", color: "var(--accent)", run: openView({ kind: "board" }) },
            { id: "act:graph", label: "依存グラフを開く", detail: "mermaid", color: "var(--doc-superseded)", run: openView({ kind: "graph", focus: null }) },
            { id: "act:search", label: "全文検索を開く", detail: "issues + docs", color: "var(--status-ready)", run: openView({ kind: "search", query: "" }) },
            {
                id: "act:split-right",
                label: "アクティブペインを右に分割",
                detail: "Ctrl+\\",
                color: "var(--fg-muted)",
                run: () => dispatch({ type: "split-pane", paneId: state.activePaneId, axis: "row" }),
            },
            { id: "act:reload", label: "リポジトリを再スキャン", detail: "ファイル変更を取り込む", color: "var(--fg-muted)", run: () => reload() },
            { id: "act:reset", label: "レイアウトを初期化", detail: "全ペインを閉じる", color: "var(--doc-rejected)", run: () => dispatch({ type: "reset" }) },
        ].map((command) => ({ ...command, haystack: `${command.label} ${command.detail}`.toLowerCase() }));

        const issues: Command[] = (data?.issues ?? []).map((issue) => ({
            id: `issue:${issue.path}`,
            label: `#${issue.id} ${issue.title}`,
            detail: `${issue.status} · ${issue.tracks.join(", ")}`,
            color: statusColor(issue.status),
            haystack: `#${issue.id} ${issue.id} ${issue.title} ${issue.tracks.join(" ")}`.toLowerCase(),
            run: openView({ kind: "file", path: issue.path }),
        }));

        const docs: Command[] = (data?.docs ?? []).map((doc) => ({
            id: `doc:${doc.path}`,
            label: doc.title,
            detail: `${docKindLabel(doc.kind)} · ${doc.status}`,
            color: docStatusColor(doc.status),
            haystack: `${doc.id} ${doc.title} ${doc.kind}`.toLowerCase(),
            run: openView({ kind: "file", path: doc.path }),
        }));

        return [...actions, ...docs, ...issues];
    }, [data, dispatch, open, reload, state.activePaneId]);

    const results = useMemo(() => {
        const needle = query.trim().toLowerCase();
        if (!needle) return commands.slice(0, MAX_RESULTS);
        const scored: { command: Command; score: number }[] = [];
        for (const command of commands) {
            const at = command.haystack.indexOf(needle);
            if (at === -1) continue;
            const isWordStart = at === 0 || /[\s#/·,-]/.test(command.haystack[at - 1]);
            scored.push({ command, score: (at === 0 ? 100 : 0) + (isWordStart ? 30 : 0) - Math.min(at, 25) });
        }
        scored.sort((a, b) => b.score - a.score);
        return scored.slice(0, MAX_RESULTS).map((entry) => entry.command);
    }, [commands, query]);

    useEffect(() => {
        setCursor(0);
    }, [query]);

    useEffect(() => {
        listRef.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" });
    }, [cursor, results]);

    if (!isOpen) return null;

    const runAt = (index: number, event: { metaKey: boolean; ctrlKey: boolean; shiftKey: boolean }): void => {
        const command = results[index];
        if (!command) return;
        command.run(event);
        onClose();
    };

    return (
        <div className="palette-backdrop" onPointerDown={onClose}>
            <div className="palette" role="dialog" aria-modal="true" aria-label="コマンドパレット" onPointerDown={(event) => event.stopPropagation()}>
                <input
                    ref={inputRef}
                    type="text"
                    className="palette__input"
                    placeholder="コマンド、issue 番号、ADR タイトル…"
                    value={query}
                    onChange={(event) => setQuery(event.target.value)}
                    onKeyDown={(event) => {
                        if (event.key === "ArrowDown") {
                            event.preventDefault();
                            setCursor((index) => Math.min(index + 1, results.length - 1));
                        } else if (event.key === "ArrowUp") {
                            event.preventDefault();
                            setCursor((index) => Math.max(index - 1, 0));
                        } else if (event.key === "Enter") {
                            event.preventDefault();
                            runAt(cursor, event);
                        }
                    }}
                />
                <div className="palette__list" ref={listRef} role="listbox">
                    {results.map((command, index) => (
                        <button
                            key={command.id}
                            type="button"
                            role="option"
                            aria-selected={index === cursor}
                            className={index === cursor ? "palette__item palette__item--active" : "palette__item"}
                            style={{ ["--status-color" as string]: command.color }}
                            onPointerEnter={() => setCursor(index)}
                            onClick={(event) => runAt(index, event)}
                        >
                            <span className="status-dot" />
                            <span className="palette__label">{command.label}</span>
                            <span className="palette__detail">{command.detail}</span>
                        </button>
                    ))}
                    {results.length === 0 ? <p className="palette__empty">該当なし</p> : null}
                </div>
                <footer className="palette__footer">
                    <span>
                        <kbd>↑</kbd>
                        <kbd>↓</kbd> 選択
                    </span>
                    <span>
                        <kbd>Enter</kbd> 開く
                    </span>
                    <span>
                        <kbd>Ctrl</kbd>+<kbd>Enter</kbd> 右に分割して開く
                    </span>
                    <span>
                        <kbd>Esc</kbd> 閉じる
                    </span>
                </footer>
            </div>
        </div>
    );
}
