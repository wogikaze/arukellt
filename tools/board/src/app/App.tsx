import { useCallback, useEffect, useState } from "react";
import type { ReactNode } from "react";

import { useBoard } from "../data/BoardContext";
import { useCopy } from "../components/Toast";
import { useTheme } from "./ThemeContext";
import { Sidebar } from "./Sidebar";
import { CommandPalette } from "./CommandPalette";
import { PaneTree } from "../workspace/PaneTree";
import { useWorkspace } from "../workspace/WorkspaceContext";
import { encodeLayoutLink, parseDeepLink } from "../workspace/persistence";
import { useShortcuts } from "./shortcuts";

export function App(): ReactNode {
    const { data, error, isRefreshing, reload } = useBoard();
    const { state, dispatch, open } = useWorkspace();
    const { theme, toggleTheme } = useTheme();
    const copy = useCopy();
    const [isPaletteOpen, setPaletteOpen] = useState(false);
    const [isSidebarVisible, setSidebarVisible] = useState(true);

    useShortcuts({
        openPalette: () => setPaletteOpen(true),
        toggleSidebar: () => setSidebarVisible((visible) => !visible),
        dispatch,
        activePaneId: state.activePaneId,
    });

    // Deep links open once and are then cleared, so a later reload restores the
    // saved layout rather than re-opening the linked file.
    const applyDeepLink = useCallback(() => {
        const link = parseDeepLink(location.hash);
        if (!link || !data) return;
        if (link.kind === "board") open({ kind: "board" });
        if (link.kind === "graph") open({ kind: "graph", focus: link.value || null });
        if (link.kind === "search") open({ kind: "search", query: link.value });
        if (link.kind === "file") open({ kind: "file", path: link.value });
        if (link.kind === "issue") {
            const issue = data.issues.find((candidate) => candidate.id === link.value.replace(/^#/, ""));
            if (issue) open({ kind: "file", path: issue.path });
        }
        history.replaceState(null, "", location.pathname);
    }, [data, open]);

    useEffect(() => {
        applyDeepLink();
        window.addEventListener("hashchange", applyDeepLink);
        return () => window.removeEventListener("hashchange", applyDeepLink);
    }, [applyDeepLink]);

    if (error) {
        return (
            <div className="empty-state">
                <p className="empty-state__title">サーバーに接続できません</p>
                <p>{error}</p>
                <button type="button" className="toolbar-button" onClick={reload}>
                    再試行
                </button>
            </div>
        );
    }

    if (!data) {
        return (
            <div className="empty-state">
                <div className="spinner" />
                <p>リポジトリをスキャンしています…</p>
            </div>
        );
    }

    return (
        <div className="app">
            <header className="topbar">
                <button
                    type="button"
                    className="icon-button"
                    aria-pressed={isSidebarVisible}
                    title="サイドバーの表示切り替え (Ctrl+B)"
                    onClick={() => setSidebarVisible((visible) => !visible)}
                >
                    <SidebarIcon />
                </button>
                <span className="topbar__brand">Arukellt Board</span>
                <nav className="topbar__nav" aria-label="サイト内リンク">
                    <a href="/">Docs</a>
                    <a href="/playground/">Playground</a>
                </nav>
                <span className="topbar__repo text-mono" title={data.repoRoot}>
                    {data.repoName}
                    {data.branch ? <span className="topbar__branch">{data.branch}</span> : null}
                </span>

                <button type="button" className="topbar__palette" onClick={() => setPaletteOpen(true)}>
                    <SearchIcon />
                    <span>コマンド・issue・ADR を検索</span>
                    <kbd>Ctrl</kbd>
                    <kbd>K</kbd>
                </button>

                <div className="topbar__spacer" />

                <span className="topbar__generated" title={`最終スキャン: ${data.generatedAt}`}>
                    {new Date(data.generatedAt).toLocaleTimeString()}
                </span>
                <button
                    type="button"
                    className="icon-button"
                    title="リポジトリを再スキャン"
                    disabled={isRefreshing}
                    onClick={reload}
                >
                    {isRefreshing ? <span className="spinner" /> : <RefreshIcon />}
                </button>
                <button
                    type="button"
                    className="icon-button"
                    title="現在のレイアウトを共有リンクとしてコピー"
                    onClick={() => copy(encodeLayoutLink(state), "レイアウトリンク")}
                >
                    <LinkIcon />
                </button>
                <button
                    type="button"
                    className="icon-button"
                    title={theme === "dark" ? "ライトテーマに切り替え" : "ダークテーマに切り替え"}
                    onClick={toggleTheme}
                >
                    {theme === "dark" ? "☾" : "☀"}
                </button>
            </header>

            <div className="app__body">
                {isSidebarVisible ? <Sidebar /> : null}
                <main className="workspace">
                    <PaneTree node={state.root} />
                </main>
            </div>

            <CommandPalette isOpen={isPaletteOpen} onClose={() => setPaletteOpen(false)} />
        </div>
    );
}

function SidebarIcon(): ReactNode {
    return (
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.3" aria-hidden="true">
            <rect x="1.5" y="2.5" width="13" height="11" rx="1.5" />
            <line x1="6" y1="2.5" x2="6" y2="13.5" />
        </svg>
    );
}

function SearchIcon(): ReactNode {
    return (
        <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="1.4" aria-hidden="true">
            <circle cx="7" cy="7" r="4.5" />
            <line x1="10.5" y1="10.5" x2="14" y2="14" />
        </svg>
    );
}

function RefreshIcon(): ReactNode {
    return (
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.3" aria-hidden="true">
            <path d="M13.5 8a5.5 5.5 0 1 1-1.7-3.9" />
            <polyline points="13.5,1.5 13.5,4.6 10.4,4.6" />
        </svg>
    );
}

function LinkIcon(): ReactNode {
    return (
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.3" aria-hidden="true">
            <path d="M6.5 9.5a3 3 0 0 1 0-4.2l2-2a3 3 0 0 1 4.2 4.2l-1 1" />
            <path d="M9.5 6.5a3 3 0 0 1 0 4.2l-2 2a3 3 0 0 1-4.2-4.2l1-1" />
        </svg>
    );
}
