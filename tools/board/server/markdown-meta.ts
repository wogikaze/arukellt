/**
 * Metadata extraction for the repository's markdown conventions.
 *
 * Three shapes coexist and all are read-only inputs to the board, so parsing
 * stays tolerant: a file that does not match a convention degrades to empty
 * metadata rather than failing the whole scan.
 *
 *   1. YAML frontmatter        `---\nStatus: open\n---`
 *   2. Bullet metadata         `- Track: compiler`
 *   3. Bold inline metadata    `**Status**: done`
 *   4. Plain header lines      `決定日: 2026-03-24` (ADR convention)
 */

export type Meta = Record<string, string>;

/** One metadata line in any of the bullet / bold / plain forms, with an ASCII or CJK key. */
const HEADER_META =
    /^\s*(?:[-*]\s+|>\s*)?(?:\*\*)?([\p{L}][\p{L}\w _/-]{0,28}?)(?:\*\*)?\s*[:：]\s+(.+?)\s*$/u;
const CHECKED_BOX = /^\s*[-*] \[[xX]\]/gm;
const UNCHECKED_BOX = /^\s*[-*] \[ \]/gm;
/** Lines that describe the document rather than its content, skipped by summaries. */
const METADATA_LINE = /^\s*(?:[-*]\s+|>\s*)?(?:\*\*)?(?:ステータス|Status|状態|決定日|決定者|日付|Date|Decided|Updated|Created|Track|ID|Priority|Depends|Supersed|関連|改訂日)/i;

function unquote(value: string): string {
    const trimmed = value.trim();
    if (trimmed.length >= 2 && trimmed[0] === trimmed[trimmed.length - 1]) {
        if (trimmed[0] === '"' || trimmed[0] === "'") return trimmed.slice(1, -1);
    }
    return trimmed;
}

/** Split YAML frontmatter from the body. Continuation lines fold into the previous key. */
export function splitFrontmatter(text: string): { meta: Meta; body: string } {
    if (!text.startsWith("---\n")) return { meta: {}, body: text };
    const end = text.indexOf("\n---", 4);
    if (end === -1) return { meta: {}, body: text };

    const block = text.slice(4, end);
    const body = text.slice(end + 4).replace(/^\r?\n/, "");
    const meta: Meta = {};
    let lastKey = "";
    for (const line of block.split("\n")) {
        if (!line.trim() || line.trimStart().startsWith("#")) continue;
        if (/^\s/.test(line) && lastKey) {
            meta[lastKey] = `${meta[lastKey]} ${line.trim()}`.trim();
            continue;
        }
        const colon = line.indexOf(":");
        if (colon === -1) continue;
        lastKey = line.slice(0, colon).trim();
        meta[lastKey] = unquote(line.slice(colon + 1));
    }
    return { meta, body };
}

/**
 * Backfill metadata from the header region of files without frontmatter.
 *
 * Scanning stops at the first section heading or horizontal rule: past that
 * point a `word: text` line is prose, not metadata. Values are only ever read
 * back through `pick()` with known keys, so an occasional stray entry is inert.
 */
export function inlineMeta(body: string, limit = 25): Meta {
    const meta: Meta = {};
    for (const line of body.split("\n").slice(0, limit)) {
        if (/^#{2,}\s/.test(line) || /^\s*(-{3,}|\*{3,}|={3,})\s*$/.test(line)) break;
        const match = HEADER_META.exec(line);
        if (!match) continue;
        const key = match[1].trim();
        if (!(key in meta)) meta[key] = unquote(match[2]);
    }
    return meta;
}

/** Case-insensitive lookup over several spellings of the same field. */
export function pick(meta: Meta, ...keys: string[]): string {
    const lowered = new Map(Object.entries(meta).map(([k, v]) => [k.toLowerCase(), v]));
    for (const key of keys) {
        const value = lowered.get(key.toLowerCase());
        if (value) return value.trim();
    }
    return "";
}

export function firstHeading(body: string): string {
    for (const line of body.split("\n", 60)) {
        if (line.startsWith("# ")) return stripMarkdown(line.slice(2));
    }
    return "";
}

/**
 * First prose paragraph after a `## Summary`-like heading, falling back to the
 * first paragraph of the document. Used verbatim as card subtitle text.
 */
export function summaryParagraph(body: string): string {
    const lines = body.split("\n");
    const headingAt = lines.findIndex((line) => /^#{2,3}\s+(Summary|概要|目的|文脈|背景|Context|決定)/i.test(line));
    const start = headingAt === -1 ? 0 : headingAt + 1;

    const collected: string[] = [];
    for (const line of lines.slice(start)) {
        const trimmed = line.trim();
        const isProse =
            trimmed !== "" &&
            !trimmed.startsWith("#") &&
            !trimmed.startsWith("|") &&
            !trimmed.startsWith("```") &&
            !/^(-{3,}|\*{3,}|={3,})$/.test(trimmed) &&
            !METADATA_LINE.test(trimmed);
        if (!isProse) {
            // Leading noise is skipped; noise after prose ends the paragraph.
            if (collected.length) break;
            continue;
        }
        collected.push(trimmed.replace(/^[-*]\s+/, ""));
        if (collected.join(" ").length > 240) break;
    }
    return stripMarkdown(collected.join(" ")).slice(0, 240);
}

export function stripMarkdown(text: string): string {
    return text
        .replace(/`([^`]*)`/g, "$1")
        .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
        .replace(/\*\*|__|\*|_/g, "")
        .replace(/\s+/g, " ")
        .trim();
}

export function countCheckboxes(text: string): { checked: number; total: number } {
    const checked = text.match(CHECKED_BOX)?.length ?? 0;
    const unchecked = text.match(UNCHECKED_BOX)?.length ?? 0;
    return { checked, total: checked + unchecked };
}

/**
 * Parse a `Depends on:` / `Track:` style list into normalized tokens.
 * Accepts commas, slashes, and full-width separators used in Japanese prose.
 */
export function splitList(raw: string): string[] {
    const text = raw.trim();
    if (!text || /^(none|なし|n\/a|—|-|\.)$/i.test(text)) return [];
    return text
        .split(/[,、\/]|\s+and\s+/)
        .map((token) => token.trim())
        .filter(Boolean);
}
