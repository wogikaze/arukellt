import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";

import { useEscapeKey } from "./Toast";

/**
 * Filter dropdown for long value lists (69 tracks, 24 orchestration classes).
 * A plain `<select multiple>` cannot be searched, which is the deciding factor.
 */
export function MultiSelect({
    label,
    options,
    selected,
    onChange,
}: {
    label: string;
    options: string[];
    selected: string[];
    onChange: (next: string[]) => void;
}): ReactNode {
    const [isOpen, setOpen] = useState(false);
    const [query, setQuery] = useState("");
    const containerRef = useRef<HTMLDivElement>(null);
    const inputRef = useRef<HTMLInputElement>(null);

    useEscapeKey(isOpen, () => setOpen(false));
    useEffect(() => {
        if (!isOpen) return;
        inputRef.current?.focus();
        const onPointerDown = (event: PointerEvent): void => {
            if (!containerRef.current?.contains(event.target as Node)) setOpen(false);
        };
        window.addEventListener("pointerdown", onPointerDown);
        return () => window.removeEventListener("pointerdown", onPointerDown);
    }, [isOpen]);

    const matches = useMemo(() => {
        const needle = query.trim().toLowerCase();
        return needle ? options.filter((option) => option.toLowerCase().includes(needle)) : options;
    }, [options, query]);

    const toggle = (option: string): void => {
        onChange(selected.includes(option) ? selected.filter((value) => value !== option) : [...selected, option]);
    };

    return (
        <div className="multiselect" ref={containerRef}>
            <button
                type="button"
                className={selected.length ? "toolbar-button toolbar-button--active" : "toolbar-button"}
                aria-haspopup="listbox"
                aria-expanded={isOpen}
                onClick={() => setOpen((open) => !open)}
            >
                {label}
                {selected.length ? <span className="toolbar-button__count">{selected.length}</span> : null}
                <span className="toolbar-button__caret">▾</span>
            </button>
            {isOpen ? (
                <div className="multiselect__panel">
                    <input
                        ref={inputRef}
                        type="search"
                        className="multiselect__search"
                        placeholder={`${label} を検索`}
                        value={query}
                        onChange={(event) => setQuery(event.target.value)}
                    />
                    <div className="multiselect__options" role="listbox" aria-multiselectable="true">
                        {matches.map((option) => (
                            <label key={option} className="multiselect__option">
                                <input
                                    type="checkbox"
                                    checked={selected.includes(option)}
                                    onChange={() => toggle(option)}
                                />
                                <span>{option}</span>
                            </label>
                        ))}
                        {matches.length === 0 ? <p className="multiselect__empty">該当なし</p> : null}
                    </div>
                    {selected.length ? (
                        <button type="button" className="multiselect__clear" onClick={() => onChange([])}>
                            選択を解除
                        </button>
                    ) : null}
                </div>
            ) : null}
        </div>
    );
}
