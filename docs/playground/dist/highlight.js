/**
 * Syntax highlighting for Arukellt source code.
 *
 * Maps token kinds from the `ark-playground-wasm` tokenizer to CSS class
 * names and produces highlighted HTML from a token stream.
 *
 * @module
 */
/** CSS class prefix for all highlight spans. */
const CLASS_PREFIX = "ark-hl-";
/**
 * Unit-variant token kinds that are keywords.
 *
 * These appear as exact strings in the `Token.kind` field (e.g. `"Fn"`).
 */
const KEYWORD_KINDS = new Set([
    "Fn",
    "Struct",
    "Enum",
    "Let",
    "Mut",
    "If",
    "Else",
    "Match",
    "While",
    "Loop",
    "For",
    "In",
    "Break",
    "Continue",
    "Return",
    "Pub",
    "Import",
    "As",
    "Trait",
    "Impl",
    "Use",
]);
/**
 * Unit-variant token kinds that are operators.
 */
const OPERATOR_KINDS = new Set([
    "Plus",
    "Minus",
    "Star",
    "Slash",
    "Percent",
    "EqEq",
    "BangEq",
    "Lt",
    "LtEq",
    "Gt",
    "GtEq",
    "AmpAmp",
    "PipePipe",
    "Bang",
    "Amp",
    "Pipe",
    "Caret",
    "Tilde",
    "Shl",
    "Shr",
    "Eq",
    "Arrow",
    "FatArrow",
]);
/**
 * Unit-variant token kinds that are punctuation/delimiters.
 */
const PUNCTUATION_KINDS = new Set([
    "LParen",
    "RParen",
    "LBrace",
    "RBrace",
    "LBracket",
    "RBracket",
    "Comma",
    "Semi",
    "Dot",
    "DotDot",
    "Question",
    "Colon",
    "ColonColon",
]);
/**
 * Whitespace/structural token kinds that receive no special styling.
 */
const PLAIN_KINDS = new Set(["Newline", "Eof", "Error"]);
/**
 * Prefixes for tuple-variant token kinds.
 *
 * The wasm tokenizer uses Rust's `Debug` format, so tuple variants
 * look like `IntLit(42)`, `Ident("main")`, etc. We match by prefix.
 */
const PREFIX_MAP = [
    // Numeric literals
    ["IntLit", "number"],
    ["FloatLit", "number"],
    ["TypedIntLit", "number"],
    ["TypedFloatLit", "number"],
    // String literals
    ["StringLit", "string"],
    ["FStringLit", "string"],
    ["CharLit", "string"],
    // Boolean literals
    ["BoolLit", "boolean"],
    // Comments
    ["OuterDocComment", "comment"],
    ["InnerDocComment", "comment"],
    // Identifiers
    ["Ident", "identifier"],
    // Reserved keywords
    ["Reserved", "keyword"],
];
/**
 * Classify a token kind string into a highlight category.
 *
 * Handles both unit variants (e.g. `"Fn"`) and tuple variants
 * (e.g. `"Ident(\"main\")"`, `"IntLit(42)"`).
 *
 * @param kind - The `Token.kind` string from the tokenizer.
 * @returns The highlight category for CSS class generation.
 */
export function classifyTokenKind(kind) {
    // Fast path: exact match on unit variants.
    if (KEYWORD_KINDS.has(kind))
        return "keyword";
    if (OPERATOR_KINDS.has(kind))
        return "operator";
    if (PUNCTUATION_KINDS.has(kind))
        return "punctuation";
    if (PLAIN_KINDS.has(kind))
        return "plain";
    // Tuple variant: match by prefix (e.g. "IntLit(42)" starts with "IntLit").
    for (const [prefix, category] of PREFIX_MAP) {
        if (kind === prefix || kind.startsWith(prefix + "(")) {
            return category;
        }
    }
    // Unknown token kind — treat as plain text.
    return "plain";
}
/**
 * Return the CSS class name for a highlight category.
 *
 * @param category - The highlight category.
 * @returns CSS class name (e.g. `"ark-hl-keyword"`).
 */
export function categoryClass(category) {
    return CLASS_PREFIX + category;
}
// ---------------------------------------------------------------------------
// HTML generation
// ---------------------------------------------------------------------------
/**
 * Escape HTML special characters in source text.
 * @internal
 */
function escapeHtml(text) {
    return text
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;");
}
/**
 * Produce highlighted HTML from source text and a token stream.
 *
 * Gaps between tokens (whitespace not captured by the tokenizer) are
 * emitted as unstyled text. Each token is wrapped in a `<span>` with
 * the appropriate `ark-hl-*` CSS class.
 *
 * @param source - The original source text.
 * @param tokens - Token array from `tokenize()`.
 * @returns HTML string with `<span>` elements for each token.
 *
 * @example
 * ```ts
 * const resp = playground.tokenize("fn main() {}");
 * const html = highlightTokens("fn main() {}", resp.tokens);
 * // '<span class="ark-hl-keyword">fn</span> <span class="ark-hl-identifier">main</span>...'
 * ```
 */
export function highlightTokens(source, tokens) {
    const parts = [];
    let pos = 0;
    for (const token of tokens) {
        // Skip EOF tokens.
        if (token.kind === "Eof")
            continue;
        // Emit any gap (whitespace) between previous position and this token.
        if (token.start > pos) {
            parts.push(escapeHtml(source.slice(pos, token.start)));
        }
        const category = classifyTokenKind(token.kind);
        if (category === "plain") {
            // Plain tokens (Newline, Error) — emit unclassed.
            parts.push(escapeHtml(token.text));
        }
        else {
            parts.push(`<span class="${categoryClass(category)}">${escapeHtml(token.text)}</span>`);
        }
        pos = token.end;
    }
    // Emit any trailing text after the last token.
    if (pos < source.length) {
        parts.push(escapeHtml(source.slice(pos)));
    }
    // Ensure trailing newline so the pre element doesn't collapse.
    const html = parts.join("");
    if (!html.endsWith("\n")) {
        return html + "\n";
    }
    return html;
}
//# sourceMappingURL=highlight.js.map