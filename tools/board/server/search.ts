import { readFileSync, statSync } from "node:fs";
import { join } from "node:path";

import type { SearchHit, SearchResponse } from "../shared/types";
import { REPO_ROOT } from "./repo";
import { getDataset } from "./dataset";

const MAX_HITS = 60;
const MAX_EXCERPTS_PER_FILE = 3;
const EXCERPT_LENGTH = 200;

/** Title matches outrank body matches so `#714` finds the issue itself first. */
const TITLE_WEIGHT = 40;
const ID_WEIGHT = 200;
const BODY_WEIGHT = 1;

interface CachedFile {
    mtimeMs: number;
    lowered: string;
    lines: string[];
}

const fileCache = new Map<string, CachedFile>();

function readIndexed(relPath: string): CachedFile | null {
    const abs = join(REPO_ROOT, relPath);
    let mtimeMs: number;
    try {
        mtimeMs = statSync(abs).mtimeMs;
    } catch {
        fileCache.delete(relPath);
        return null;
    }
    const hit = fileCache.get(relPath);
    if (hit && hit.mtimeMs === mtimeMs) return hit;

    const text = readFileSync(abs, "utf8");
    const entry: CachedFile = { mtimeMs, lowered: text.toLowerCase(), lines: text.split("\n") };
    fileCache.set(relPath, entry);
    return entry;
}

function countOccurrences(haystack: string, needle: string): number {
    let count = 0;
    let from = 0;
    while (true) {
        const at = haystack.indexOf(needle, from);
        if (at === -1) return count;
        count += 1;
        from = at + needle.length;
    }
}

function collectExcerpts(lines: string[], needle: string): { line: number; text: string }[] {
    const excerpts: { line: number; text: string }[] = [];
    for (let i = 0; i < lines.length && excerpts.length < MAX_EXCERPTS_PER_FILE; i += 1) {
        const at = lines[i].toLowerCase().indexOf(needle);
        if (at === -1) continue;
        const start = Math.max(0, at - 60);
        excerpts.push({
            line: i + 1,
            text: (start > 0 ? "…" : "") + lines[i].slice(start, start + EXCERPT_LENGTH).trim(),
        });
    }
    return excerpts;
}

/**
 * Case-insensitive substring search across issue and decision-document bodies.
 * Substring rather than token matching is deliberate: identifiers like
 * `wasm32-gc`, `ADR-013` and `#714` are the dominant queries here and tokenizers
 * split them apart.
 */
export function search(query: string): SearchResponse {
    const needle = query.trim().toLowerCase();
    if (needle.length < 2) return { query, total: 0, hits: [] };

    const data = getDataset();
    const bareId = /^#?(\d{1,4})$/.exec(needle)?.[1];
    const hits: SearchHit[] = [];

    const candidates = [
        ...data.issues.map((issue) => ({
            path: issue.path,
            title: issue.title,
            kind: "issue" as const,
            status: issue.status,
            identity: issue.id,
        })),
        ...data.docs.map((doc) => ({
            path: doc.path,
            title: doc.title,
            kind: "doc" as const,
            status: doc.status,
            identity: doc.id,
        })),
    ];

    for (const candidate of candidates) {
        const indexed = readIndexed(candidate.path);
        if (!indexed) continue;

        const bodyMatches = countOccurrences(indexed.lowered, needle);
        const titleMatches = countOccurrences(candidate.title.toLowerCase(), needle);
        const identityMatch = bareId ? candidate.identity.toLowerCase() === bareId : false;
        if (!bodyMatches && !titleMatches && !identityMatch) continue;

        hits.push({
            path: candidate.path,
            title: candidate.title,
            kind: candidate.kind,
            status: candidate.status,
            score:
                (identityMatch ? ID_WEIGHT : 0) +
                titleMatches * TITLE_WEIGHT +
                Math.min(bodyMatches, 20) * BODY_WEIGHT,
            excerpts: collectExcerpts(indexed.lines, needle),
        });
    }

    hits.sort((a, b) => b.score - a.score || a.path.localeCompare(b.path));
    return { query, total: hits.length, hits: hits.slice(0, MAX_HITS) };
}
