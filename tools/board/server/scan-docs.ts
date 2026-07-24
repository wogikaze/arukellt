import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

import type { DocKind, DocRecord, DocStatus, TreeNode } from "../shared/types";
import { REPO_ROOT } from "./repo";
import {
    firstHeading,
    inlineMeta,
    pick,
    splitFrontmatter,
    stripMarkdown,
    summaryParagraph,
} from "./markdown-meta";

/** Directories scanned as structured decision/knowledge documents. */
const DOC_SOURCES: { kind: DocKind; dir: string }[] = [
    { kind: "adr", dir: "docs/adr" },
    { kind: "rfc", dir: "docs/rfcs" },
    { kind: "plan", dir: "docs/plans" },
    { kind: "research", dir: "docs/research" },
    { kind: "process", dir: "docs/process" },
    { kind: "state", dir: "docs/state" },
];

const DOC_ID = /^((?:ADR|RFC)-\d+|\d{3})/i;
const ISSUE_REF = /#(\d{1,4})\b/g;
const DOC_REF = /\b(ADR|RFC)-(\d{3})\b/g;

/** Extensions surfaced in the docs tree; everything else is build output or assets. */
const TREE_EXTENSIONS = new Set([".md", ".html", ".toml"]);
const TREE_HIDDEN = new Set(["_sidebar.md", "_coverpage.md", ".nojekyll", "node_modules", "dist"]);

/**
 * Decision documents declare status on a `ステータス:` / `Status:` line near the
 * top, with rationale appended after an em dash. Order matters: `SUPERSEDED`
 * lines frequently name the superseding ADR, which itself says `ACCEPTED`.
 */
const STATUS_KEYWORDS: [DocStatus, RegExp][] = [
    ["SUPERSEDED", /\bSUPERSEDED\b|置換|廃止/i],
    ["REJECTED", /\bREJECTED\b|却下/i],
    ["ACCEPTED", /\bACCEPTED\b|採択/i],
    ["PROPOSED", /\bPROPOSED\b|提案/i],
    ["DRAFT", /\bDRAFT\b|草案|運用メモ/i],
];

function statusLine(body: string): string {
    for (const line of body.split("\n").slice(0, 30)) {
        const match = /^\s*(?:>\s*)?(?:\*\*)?(?:ステータス|Status|状態)(?:\*\*)?\s*[:：]\s*(.+?)\s*$/.exec(line);
        // ADR-000 documents the lifecycle itself and starts with a link line;
        // a status line that is only a link carries no state.
        if (match && !/^\[/.test(match[1].trim())) return match[1].trim();
    }
    return "";
}

function classifyStatus(text: string): DocStatus {
    if (!text) return "UNKNOWN";
    for (const [status, pattern] of STATUS_KEYWORDS) {
        if (pattern.test(text)) return status;
    }
    return "UNKNOWN";
}

function uniqueMatches(text: string, pattern: RegExp, format: (m: RegExpExecArray) => string): string[] {
    const found = new Set<string>();
    for (const match of text.matchAll(pattern)) found.add(format(match as RegExpExecArray));
    return [...found];
}

function parseDocFile(kind: DocKind, relPath: string, name: string): DocRecord {
    const text = readFileSync(join(REPO_ROOT, relPath), "utf8");
    const { meta: frontmatter, body } = splitFrontmatter(text);
    const meta = { ...inlineMeta(body), ...frontmatter };

    const rawStatus = statusLine(body) || pick(meta, "Status", "ステータス");
    const idMatch = DOC_ID.exec(name);

    return {
        id: idMatch ? idMatch[1].toUpperCase() : name.replace(/\.md$/, ""),
        kind,
        title: firstHeading(body) || name.replace(/\.md$/, ""),
        path: relPath,
        status: classifyStatus(rawStatus),
        statusText: stripMarkdown(rawStatus),
        decidedOn: pick(meta, "決定日", "Decided", "Date", "日付"),
        relatedIssues: uniqueMatches(body, ISSUE_REF, (m) => String(Number(m[1]))),
        relatedDocs: uniqueMatches(body, DOC_REF, (m) => `${m[1].toUpperCase()}-${m[2]}`).filter(
            (ref) => ref !== (idMatch ? idMatch[1].toUpperCase() : ""),
        ),
        summary: summaryParagraph(body),
    };
}

export function scanDocs(): DocRecord[] {
    const docs: DocRecord[] = [];
    for (const { kind, dir } of DOC_SOURCES) {
        let names: string[];
        try {
            names = readdirSync(join(REPO_ROOT, dir)).filter((n) => n.endsWith(".md")).sort();
        } catch {
            continue;
        }
        for (const name of names) docs.push(parseDocFile(kind, `${dir}/${name}`, name));
    }
    return docs;
}

/** Recursive listing of docs/ used by the file-tree sidebar. */
export function scanDocsTree(): TreeNode[] {
    return buildTree(join(REPO_ROOT, "docs"), "docs");
}

function buildTree(absDir: string, relDir: string): TreeNode[] {
    let names: string[];
    try {
        names = readdirSync(absDir).sort();
    } catch {
        return [];
    }

    const dirs: TreeNode[] = [];
    const files: TreeNode[] = [];
    for (const name of names) {
        if (name.startsWith(".") || TREE_HIDDEN.has(name)) continue;
        const abs = join(absDir, name);
        const rel = `${relDir}/${name}`;
        if (statSync(abs).isDirectory()) {
            const children = buildTree(abs, rel);
            if (children.length) dirs.push({ name, path: rel, type: "dir", children });
            continue;
        }
        const dot = name.lastIndexOf(".");
        if (dot !== -1 && TREE_EXTENSIONS.has(name.slice(dot))) {
            files.push({ name, path: rel, type: "file" });
        }
    }
    return [...dirs, ...files];
}
