import type { BoardData, DocRecord, IssueRecord } from "../../shared/types";

/**
 * Text blocks meant to be pasted straight into a coding agent.
 *
 * Everything here is derived from files the agent can also read, so the block
 * stays short: it establishes identity and paths, and points at the canonical
 * sources instead of duplicating their content.
 */

function absolutePath(data: BoardData, relPath: string): string {
    return `${data.repoRoot}/${relPath}`;
}

function depLine(ids: string[], byId: Map<string, IssueRecord>): string {
    if (!ids.length) return "none";
    return ids
        .map((id) => {
            const dep = byId.get(id);
            return dep ? `#${id} (${dep.status})` : `#${id} (unknown)`;
        })
        .join(", ");
}

export function issueHandoffPrompt(issue: IssueRecord, data: BoardData): string {
    const byId = new Map(data.issues.map((candidate) => [candidate.id, candidate]));
    const lines = [
        `# Task: #${issue.id} — ${issue.title}`,
        "",
        `- Issue file: \`${issue.path}\``,
        `- Absolute path: \`${absolutePath(data, issue.path)}\``,
        `- Status: ${issue.status} | Track: ${issue.tracks.join(", ")}` +
            (issue.priority === null ? "" : ` | Priority: ${issue.priority}`),
        `- Orchestration class: ${issue.orchestrationClass}`,
        `- Depends on: ${depLine(issue.dependsOn, byId)}`,
        `- Blocks: ${depLine(issue.blocks, byId)}`,
        `- Acceptance criteria: ${issue.acceptance.checked}/${issue.acceptance.total} checked`,
        "",
        "## Summary",
        "",
        issue.summary || "(issue ファイル本文を参照)",
        "",
        "## Instructions",
        "",
        "1. まず issue ファイル全体を読み、Acceptance Criteria と検証方針を確認する。",
        "2. `AGENTS.md` のコード品質規約と必須ワークフローに従う。",
        "3. 未完了の依存 issue がある場合は、着手前にその影響を判断する。",
        "4. 完了主張の前に issue 記載の検証コマンドを実行し、結果を報告する。",
    ];
    return lines.join("\n");
}

export function docHandoffPrompt(doc: DocRecord, data: BoardData): string {
    const lines = [
        `# Context: ${doc.id} — ${doc.title}`,
        "",
        `- Document: \`${doc.path}\``,
        `- Absolute path: \`${absolutePath(data, doc.path)}\``,
        `- Kind: ${doc.kind} | Status: ${doc.status}`,
        doc.statusText ? `- Status note: ${doc.statusText}` : "",
        doc.relatedDocs.length ? `- Related documents: ${doc.relatedDocs.join(", ")}` : "",
        doc.relatedIssues.length ? `- Referenced issues: ${doc.relatedIssues.map((id) => `#${id}`).join(", ")}` : "",
        "",
        doc.summary,
        "",
        doc.kind === "adr"
            ? "拘束力があるのは ACCEPTED の ADR のみ。PROPOSED は未採択、SUPERSEDED は履歴として扱う。"
            : "この文書は決定記録ではない。拘束力のある判断は docs/adr/ を参照する。",
    ];
    return lines.filter((line) => line !== "").join("\n");
}

/**
 * Node fills are emitted into the diagram source because mermaid resolves
 * `classDef` at render time and cannot read CSS custom properties, so the
 * palette has to follow the active theme explicitly.
 */
const MERMAID_PALETTES = {
    dark: [
        "  classDef open fill:#1b3a5c,stroke:#58a6ff,color:#e3e8f0;",
        "  classDef blocked fill:#4a3410,stroke:#e3a008,color:#e3e8f0;",
        "  classDef done fill:#173d20,stroke:#3fb950,color:#e3e8f0;",
        "  classDef reject fill:#2b2f36,stroke:#8b949e,color:#97a3b6;",
        "  classDef external fill:transparent,stroke:#6b7688,color:#97a3b6,stroke-dasharray:4 3;",
    ],
    light: [
        "  classDef open fill:#ddeaff,stroke:#1f6feb,color:#1c2230;",
        "  classDef blocked fill:#fdf1d0,stroke:#bf8700,color:#1c2230;",
        "  classDef done fill:#d8f2df,stroke:#1a7f37,color:#1c2230;",
        "  classDef reject fill:#eceff3,stroke:#6e7781,color:#5a6472;",
        "  classDef external fill:transparent,stroke:#8894a5,color:#5a6472,stroke-dasharray:4 3;",
    ],
} as const;

/** Mermaid flowchart of an issue's dependency neighbourhood or a whole lane set. */
export function dependencyMermaid(
    issues: IssueRecord[],
    byId: Map<string, IssueRecord>,
    theme: keyof typeof MERMAID_PALETTES = "dark",
): string {
    const visible = new Set(issues.map((issue) => issue.id));
    const lines = ["flowchart LR"];

    for (const issue of issues) {
        const label = issue.title.length > 44 ? `${issue.title.slice(0, 42)}…` : issue.title;
        // Quotes guard against titles containing mermaid punctuation.
        lines.push(`  n${issue.id}["#${issue.id} ${label.replace(/"/g, "'")}"]:::${issue.status}`);
    }

    for (const issue of issues) {
        for (const depId of issue.dependsOn) {
            if (!visible.has(depId)) continue;
            lines.push(`  n${depId} --> n${issue.id}`);
        }
    }

    // Dependencies outside the visible set are shown as stubs so a filtered
    // graph never silently hides a blocker.
    const stubs = new Set<string>();
    for (const issue of issues) {
        for (const depId of issue.dependsOn) {
            if (visible.has(depId) || stubs.has(depId)) continue;
            stubs.add(depId);
            const dep = byId.get(depId);
            lines.push(`  n${depId}(["#${depId} ${dep ? dep.status : "unknown"}"]):::external`);
            lines.push(`  n${depId} --> n${issue.id}`);
        }
    }

    lines.push(...MERMAID_PALETTES[theme]);
    return lines.join("\n");
}

/** Issues that participate in at least one dependency edge inside `issues`. */
export function connectedOnly(issues: IssueRecord[]): IssueRecord[] {
    const present = new Set(issues.map((issue) => issue.id));
    const linked = new Set<string>();
    for (const issue of issues) {
        for (const depId of issue.dependsOn) {
            if (!present.has(depId)) continue;
            linked.add(issue.id);
            linked.add(depId);
        }
    }
    return issues.filter((issue) => linked.has(issue.id));
}

/** Neighbourhood of one issue: upstream dependencies and downstream blocked work. */
export function neighbourhood(rootId: string, byId: Map<string, IssueRecord>, depth: number): IssueRecord[] {
    const collected = new Map<string, IssueRecord>();
    const root = byId.get(rootId);
    if (!root) return [];

    const visit = (issue: IssueRecord, remaining: number): void => {
        if (collected.has(issue.id)) return;
        collected.set(issue.id, issue);
        if (remaining === 0) return;
        for (const id of [...issue.dependsOn, ...issue.blocks]) {
            const next = byId.get(id);
            if (next) visit(next, remaining - 1);
        }
    };

    visit(root, depth);
    return [...collected.values()];
}
