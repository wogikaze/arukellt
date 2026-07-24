import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

import { useCopy, useEscapeKey } from "./Toast";

export interface CopyEntry {
    label: string;
    /** Resolved lazily so large payloads (mermaid, prompts) are built on demand. */
    value: () => string;
    hint?: string;
}

/** One-click copy button, used where a single payload is obvious. */
export function CopyButton({
    value,
    label,
    title,
    children,
    className = "copy-button",
}: {
    value: string | (() => string);
    label: string;
    title?: string;
    children?: ReactNode;
    className?: string;
}): ReactNode {
    const copy = useCopy();
    return (
        <button
            type="button"
            className={className}
            title={title ?? `${label} をコピー`}
            aria-label={title ?? `${label} をコピー`}
            onClick={(event) => {
                event.stopPropagation();
                copy(typeof value === "function" ? value() : value, label);
            }}
        >
            {children ?? <CopyIcon />}
        </button>
    );
}

/**
 * Dropdown of related copy payloads (relative path, absolute path, agent
 * prompt, …). Grouping them avoids a row of near-identical icons on every card.
 */
export function CopyMenu({ entries, compact = false }: { entries: CopyEntry[]; compact?: boolean }): ReactNode {
    const [isOpen, setOpen] = useState(false);
    const containerRef = useRef<HTMLDivElement>(null);
    const copy = useCopy();

    useEscapeKey(isOpen, () => setOpen(false));
    useEffect(() => {
        if (!isOpen) return;
        const onPointerDown = (event: PointerEvent): void => {
            if (!containerRef.current?.contains(event.target as Node)) setOpen(false);
        };
        window.addEventListener("pointerdown", onPointerDown);
        return () => window.removeEventListener("pointerdown", onPointerDown);
    }, [isOpen]);

    return (
        <div className="copy-menu" ref={containerRef}>
            <button
                type="button"
                className={compact ? "copy-button" : "toolbar-button"}
                aria-haspopup="menu"
                aria-expanded={isOpen}
                title="コピー候補を開く"
                onClick={(event) => {
                    event.stopPropagation();
                    setOpen((open) => !open);
                }}
            >
                <CopyIcon />
                {compact ? null : <span>Copy</span>}
            </button>
            {isOpen ? (
                <div className="copy-menu__list" role="menu">
                    {entries.map((entry) => (
                        <button
                            key={entry.label}
                            type="button"
                            role="menuitem"
                            className="copy-menu__item"
                            onClick={(event) => {
                                event.stopPropagation();
                                copy(entry.value(), entry.label);
                                setOpen(false);
                            }}
                        >
                            <span className="copy-menu__label">{entry.label}</span>
                            {entry.hint ? <span className="copy-menu__hint">{entry.hint}</span> : null}
                        </button>
                    ))}
                </div>
            ) : null}
        </div>
    );
}

export function CopyIcon(): ReactNode {
    return (
        <svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true" fill="currentColor">
            <path d="M5 1.75C5 .78 5.78 0 6.75 0h5.5C13.22 0 14 .78 14 1.75v7.5A1.75 1.75 0 0 1 12.25 11h-5.5A1.75 1.75 0 0 1 5 9.25Zm1.75-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h5.5a.25.25 0 0 0 .25-.25v-7.5a.25.25 0 0 0-.25-.25Z" />
            <path d="M2 5.75C2 4.78 2.78 4 3.75 4H4v1.5h-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h5.5a.25.25 0 0 0 .25-.25V13H11v.25A1.75 1.75 0 0 1 9.25 15h-5.5A1.75 1.75 0 0 1 2 13.25Z" />
        </svg>
    );
}
