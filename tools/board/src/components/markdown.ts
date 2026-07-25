import { Marked } from "marked";

/**
 * Markdown pipeline for repository documents.
 *
 * Files come from the local checkout, but they are still untrusted input in the
 * sense that a document can contain raw HTML, so the generated tree is walked
 * and stripped of script-bearing constructs before it reaches the DOM.
 */

const marked = new Marked({ gfm: true, breaks: false });

const HTML_ENTITY = /&(?:#(?:x[\da-fA-F]+|\d+)|[a-zA-Z][a-zA-Z\d]+);/g;
const NAMED_ENTITIES: Record<string, string> = {
    amp: "&",
    lt: "<",
    gt: ">",
    quot: '"',
    apos: "'",
};

export function decodeHtmlEntities(text: string): string {
    return text.replace(HTML_ENTITY, (entity) => {
        if (entity[1] === "#") {
            const code = entity[2] === "x" || entity[2] === "X"
                ? Number.parseInt(entity.slice(3, -1), 16)
                : Number.parseInt(entity.slice(2, -1), 10);
            if (Number.isFinite(code) && code > 0) return String.fromCodePoint(code);
            return entity;
        }
        const name = entity.slice(1, -1);
        return name in NAMED_ENTITIES ? NAMED_ENTITIES[name] : entity;
    });
}

const ALLOWED_TAGS = new Set([
    "a", "blockquote", "br", "code", "del", "details", "div", "em", "h1", "h2", "h3", "h4", "h5", "h6",
    "hr", "img", "input", "li", "ol", "p", "pre", "s", "span", "strong", "summary", "sub", "sup",
    "table", "tbody", "td", "th", "thead", "tr", "ul",
]);

const ALLOWED_ATTRIBUTES = new Set(["href", "src", "alt", "title", "class", "id", "colspan", "rowspan", "type", "checked", "disabled", "start"]);

const SAFE_URL = /^(https?:|mailto:|#|\/|\.{0,2}\/)/i;

export interface Heading {
    id: string;
    text: string;
    level: number;
}

export interface RenderedMarkdown {
    html: string;
    headings: Heading[];
}

function slugify(text: string, used: Map<string, number>): string {
    const base =
        text
            .toLowerCase()
            .replace(/[`*_~[\]()]/g, "")
            .trim()
            .replace(/\s+/g, "-")
            .replace(/[^\p{L}\p{N}-]/gu, "") || "section";
    const seen = used.get(base) ?? 0;
    used.set(base, seen + 1);
    return seen === 0 ? base : `${base}-${seen}`;
}

function sanitize(root: Element): void {
    for (const element of [...root.querySelectorAll("*")]) {
        const tag = element.tagName.toLowerCase();
        if (!ALLOWED_TAGS.has(tag)) {
            element.remove();
            continue;
        }
        for (const attribute of [...element.attributes]) {
            const name = attribute.name.toLowerCase();
            if (!ALLOWED_ATTRIBUTES.has(name)) {
                element.removeAttribute(attribute.name);
                continue;
            }
            if ((name === "href" || name === "src") && !SAFE_URL.test(attribute.value.trim())) {
                element.removeAttribute(attribute.name);
            }
        }
    }
}

/**
 * Mermaid fences are left as `<div class="mermaid-source">` placeholders that
 * the React view replaces with rendered diagrams, so mermaid stays out of the
 * markdown path entirely.
 */
function extractMermaidBlocks(root: Element): void {
    for (const block of [...root.querySelectorAll("pre > code.language-mermaid")]) {
        const pre = block.parentElement;
        if (!pre) continue;
        const holder = root.ownerDocument.createElement("div");
        holder.className = "mermaid-source";
        holder.textContent = block.textContent ?? "";
        pre.replaceWith(holder);
    }
}

function annotateCodeBlocks(root: Element): void {
    for (const pre of [...root.querySelectorAll("pre")]) {
        const code = pre.querySelector("code");
        const language = [...(code?.classList ?? [])]
            .find((name) => name.startsWith("language-"))
            ?.slice("language-".length);
        if (language) pre.dataset.language = language;
    }
}

function addHeadingAnchors(root: Element, idPrefix: string): Heading[] {
    const used = new Map<string, number>();
    const headings: Heading[] = [];
    for (const heading of [...root.querySelectorAll("h1, h2, h3, h4")]) {
        const text = heading.textContent?.trim() ?? "";
        const id = `${idPrefix}${slugify(text, used)}`;
        heading.id = id;
        heading.classList.add("md-heading");
        const anchor = root.ownerDocument.createElement("a");
        anchor.className = "md-heading__anchor";
        anchor.href = `#${id}`;
        anchor.textContent = "#";
        anchor.setAttribute("aria-label", `${text} へのリンク`);
        heading.appendChild(anchor);
        headings.push({ id, text, level: Number(heading.tagName.slice(1)) });
    }
    return headings;
}

/**
 * Rewrite relative links so the view layer can resolve them against the source
 * document's directory without re-parsing hrefs.
 */
function markRepoLinks(root: Element, sourcePath: string): void {
    const baseDir = sourcePath.slice(0, sourcePath.lastIndexOf("/"));
    for (const anchor of [...root.querySelectorAll("a[href]")]) {
        const href = anchor.getAttribute("href") ?? "";
        if (/^(https?:|mailto:)/i.test(href)) {
            anchor.setAttribute("target", "_blank");
            anchor.setAttribute("rel", "noreferrer noopener");
            continue;
        }
        if (href.startsWith("#")) continue;

        const [rawPath, fragment] = href.split("#");
        const resolved = new URL(rawPath, `repo:///${baseDir}/`).pathname.replace(/^\/+/, "");
        anchor.setAttribute("data-repo-path", decodeURIComponent(resolved));
        if (fragment) anchor.setAttribute("data-repo-fragment", fragment);
    }
}

/**
 * `idPrefix` namespaces heading anchors per rendered document. Split panes can
 * show two files with a `## Summary` each, and duplicate element ids would make
 * the outline of one pane jump to the other.
 */
export function renderMarkdown(text: string, sourcePath: string, idPrefix: string): RenderedMarkdown {
    const parsed = marked.parse(decodeHtmlEntities(text), { async: false });
    const doc = new DOMParser().parseFromString(`<div id="root">${parsed}</div>`, "text/html");
    const root = doc.getElementById("root");
    if (!root) return { html: "", headings: [] };

    sanitize(root);
    extractMermaidBlocks(root);
    annotateCodeBlocks(root);
    markRepoLinks(root, sourcePath);
    const headings = addHeadingAnchors(root, idPrefix);
    return { html: root.innerHTML, headings };
}
