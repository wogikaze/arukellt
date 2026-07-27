import type { Mermaid } from "mermaid";

/**
 * Lazy mermaid loader.
 *
 * mermaid is by far the largest dependency here and most sessions never open a
 * graph, so it is imported on first use and re-initialised when the theme
 * changes (its colours are baked in at render time, not read from CSS).
 */

let modulePromise: Promise<Mermaid> | null = null;
let initialisedTheme: string | null = null;
let renderCounter = 0;

const DARK_VARIABLES = {
    background: "#11151d",
    primaryColor: "#1b3a5c",
    primaryTextColor: "#e3e8f0",
    primaryBorderColor: "#58a6ff",
    lineColor: "#6b7688",
    secondaryColor: "#171c26",
    tertiaryColor: "#0d1017",
    fontSize: "13px",
};

const LIGHT_VARIABLES = {
    background: "#ffffff",
    primaryColor: "#ddeaff",
    primaryTextColor: "#1c2230",
    primaryBorderColor: "#1f6feb",
    lineColor: "#8894a5",
    secondaryColor: "#f4f6fa",
    tertiaryColor: "#eef1f6",
    fontSize: "13px",
};

async function loadMermaid(theme: string): Promise<Mermaid> {
    if (!modulePromise) modulePromise = import("mermaid").then((module) => module.default);
    const mermaid = await modulePromise;
    if (initialisedTheme !== theme) {
        mermaid.initialize({
            startOnLoad: false,
            securityLevel: "strict",
            theme: "base",
            themeVariables: theme === "light" ? LIGHT_VARIABLES : DARK_VARIABLES,
            fontFamily: getComputedStyle(document.body).fontFamily,
            flowchart: { curve: "basis", nodeSpacing: 32, rankSpacing: 46, useMaxWidth: false },
        });
        initialisedTheme = theme;
    }
    return mermaid;
}

export interface MermaidRender {
    svg: string;
    error: string | null;
}

export async function renderMermaid(source: string, theme: string): Promise<MermaidRender> {
    try {
        const mermaid = await loadMermaid(theme);
        renderCounter += 1;
        const { svg } = await mermaid.render(`mermaid-${renderCounter}`, source);
        return { svg, error: null };
    } catch (cause) {
        // A malformed diagram in one document must not blank the whole pane.
        return { svg: "", error: cause instanceof Error ? cause.message : String(cause) };
    }
}
