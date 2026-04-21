/**
 * Syntax highlighting for Arukellt source code.
 *
 * Maps token kinds from the `ark-playground-wasm` tokenizer to CSS class
 * names and produces highlighted HTML from a token stream.
 *
 * @module
 */
import type { Token } from "./types.js";
/**
 * Highlight categories used as CSS class suffixes.
 *
 * Each category maps to the CSS class `ark-hl-<category>`. For example,
 * the `keyword` category produces the class `ark-hl-keyword`.
 */
export type HighlightCategory = "keyword" | "string" | "number" | "comment" | "operator" | "punctuation" | "identifier" | "boolean" | "type" | "plain";
/**
 * Classify a token kind string into a highlight category.
 *
 * Handles both unit variants (e.g. `"Fn"`) and tuple variants
 * (e.g. `"Ident(\"main\")"`, `"IntLit(42)"`).
 *
 * @param kind - The `Token.kind` string from the tokenizer.
 * @returns The highlight category for CSS class generation.
 */
export declare function classifyTokenKind(kind: string): HighlightCategory;
/**
 * Return the CSS class name for a highlight category.
 *
 * @param category - The highlight category.
 * @returns CSS class name (e.g. `"ark-hl-keyword"`).
 */
export declare function categoryClass(category: HighlightCategory): string;
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
export declare function highlightTokens(source: string, tokens: Token[]): string;
//# sourceMappingURL=highlight.d.ts.map