/**
 * Diagnostics panel and inline marker utilities for the Arukellt playground.
 *
 * Provides:
 * - A diagnostics list panel that displays parse errors, warnings, and help
 *   messages with line/column, severity, and code information.
 * - Helper functions for converting byte offsets to line/column positions.
 * - An inline diagnostic overlay builder that generates HTML with underline
 *   markers aligned to the editor's backdrop.
 *
 * @module
 */
import type { Diagnostic } from "./types.js";
/**
 * Convert a byte offset in source text to a 1-based line and column number.
 *
 * @param source - The source text.
 * @param offset - Byte offset (0-based). Clamped to `[0, source.length]`.
 * @returns An object with 1-based `line` and `col` numbers.
 *
 * @example
 * ```ts
 * const pos = offsetToLineCol("fn main() {\n  42\n}", 14);
 * // { line: 2, col: 3 }
 * ```
 */
export declare function offsetToLineCol(source: string, offset: number): {
    line: number;
    col: number;
};
/** Options for creating a diagnostics panel. */
export interface DiagnosticsPanelOptions {
    /**
     * Whether to auto-inject diagnostic CSS into the page.
     * Default: `true`.
     */
    injectStyles?: boolean;
}
/** A diagnostics panel instance. */
export interface DiagnosticsPanel {
    /** The root DOM element of the panel. */
    readonly element: HTMLElement;
    /** Update the panel with diagnostics from a parse result. */
    update(diagnostics: Diagnostic[], source: string): void;
    /** Clear all diagnostics from the panel. */
    clear(): void;
    /** Destroy the panel and remove it from the DOM. */
    destroy(): void;
}
/**
 * Create a diagnostics panel and mount it inside the given container.
 *
 * The panel displays a list of diagnostics with severity icons,
 * line/column positions, diagnostic codes, and messages. It includes
 * a sticky header with a summary count.
 *
 * @param container - The DOM element to mount the panel into.
 * @param options - Panel configuration.
 * @returns A {@link DiagnosticsPanel} instance.
 *
 * @example
 * ```ts
 * const panel = createDiagnosticsPanel(document.getElementById("diag")!);
 * const result = playground.parse(source);
 * panel.update(result.diagnostics, source);
 * ```
 */
export declare function createDiagnosticsPanel(container: HTMLElement, options?: DiagnosticsPanelOptions): DiagnosticsPanel;
/**
 * Build an HTML overlay string that shows diagnostic underlines aligned
 * with the editor's source text.
 *
 * The returned HTML contains the same text as the source (HTML-escaped),
 * but diagnostic label ranges are wrapped in `<span>` elements with
 * underline marker CSS classes. The text itself should be rendered
 * transparent — only the underlines are visible.
 *
 * When diagnostic ranges overlap, the highest-severity diagnostic wins
 * (error > warning > help).
 *
 * @param source - The source text.
 * @param diagnostics - Diagnostics from the parse API.
 * @returns HTML string for the diagnostic overlay. Empty string if there
 *   are no diagnostic labels to display.
 *
 * @example
 * ```ts
 * const result = playground.parse(source);
 * const html = buildDiagnosticOverlay(source, result.diagnostics);
 * overlayElement.innerHTML = html;
 * ```
 */
export declare function buildDiagnosticOverlay(source: string, diagnostics: Diagnostic[]): string;
/** CSS for the diagnostics panel and inline marker overlay. */
export declare const DIAGNOSTICS_CSS = "\n/* ------------------------------------------------------------------ */\n/* Arukellt Playground \u2014 Diagnostics Panel                            */\n/* ------------------------------------------------------------------ */\n\n.ark-diagnostics-panel {\n  --ark-diag-bg: #1e1e2e;\n  --ark-diag-border: #313244;\n  --ark-diag-fg: #cdd6f4;\n  --ark-diag-error-color: #f38ba8;\n  --ark-diag-warning-color: #f9e2af;\n  --ark-diag-help-color: #89b4fa;\n  --ark-diag-loc-color: #6c7086;\n  --ark-diag-code-color: #9399b2;\n\n  background: var(--ark-diag-bg);\n  border-top: 1px solid var(--ark-diag-border);\n  color: var(--ark-diag-fg);\n  font-family: \"JetBrains Mono\", \"Fira Code\", \"Cascadia Code\",\n    \"Source Code Pro\", ui-monospace, monospace;\n  font-size: 13px;\n  max-height: 200px;\n  overflow-y: auto;\n}\n\n.ark-diagnostics-header {\n  padding: 6px 12px;\n  font-weight: 600;\n  font-size: 12px;\n  text-transform: uppercase;\n  letter-spacing: 0.05em;\n  color: var(--ark-diag-loc-color);\n  border-bottom: 1px solid var(--ark-diag-border);\n  position: sticky;\n  top: 0;\n  background: var(--ark-diag-bg);\n}\n\n.ark-diagnostics-ok {\n  color: #a6e3a1;\n}\n\n.ark-diagnostics-has-errors {\n  color: var(--ark-diag-error-color);\n}\n\n.ark-diagnostics-has-warnings {\n  color: var(--ark-diag-warning-color);\n}\n\n.ark-diagnostics-list {\n  list-style: none;\n  margin: 0;\n  padding: 0;\n}\n\n.ark-diagnostic-item {\n  display: flex;\n  flex-wrap: wrap;\n  align-items: baseline;\n  gap: 6px;\n  padding: 4px 12px;\n  border-bottom: 1px solid var(--ark-diag-border);\n}\n\n.ark-diagnostic-item:last-child {\n  border-bottom: none;\n}\n\n.ark-diagnostic-icon {\n  flex-shrink: 0;\n  width: 16px;\n  text-align: center;\n}\n\n.ark-diag-error .ark-diagnostic-icon {\n  color: var(--ark-diag-error-color);\n}\n\n.ark-diag-warning .ark-diagnostic-icon {\n  color: var(--ark-diag-warning-color);\n}\n\n.ark-diag-help .ark-diagnostic-icon {\n  color: var(--ark-diag-help-color);\n}\n\n.ark-diagnostic-loc {\n  color: var(--ark-diag-loc-color);\n  font-size: 12px;\n  min-width: 40px;\n}\n\n.ark-diagnostic-code {\n  color: var(--ark-diag-code-color);\n  font-size: 12px;\n}\n\n.ark-diagnostic-message {\n  flex: 1;\n  min-width: 0;\n}\n\n.ark-diagnostic-notes {\n  width: 100%;\n  padding-left: 22px;\n  color: var(--ark-diag-loc-color);\n  font-size: 12px;\n}\n\n.ark-diagnostic-note {\n  padding: 1px 0;\n}\n\n.ark-diagnostic-suggestion {\n  width: 100%;\n  padding-left: 22px;\n  color: #a6e3a1;\n  font-size: 12px;\n}\n\n/* ------------------------------------------------------------------ */\n/* Inline diagnostic markers (overlay in the editor)                   */\n/* ------------------------------------------------------------------ */\n\n.ark-editor-diagnostics {\n  position: absolute;\n  top: 0;\n  left: 0;\n  right: 0;\n  bottom: 0;\n  margin: 0;\n  padding: var(--ark-padding, 16px);\n  border: none;\n  font-family: var(--ark-font-family, \"JetBrains Mono\", ui-monospace, monospace);\n  font-size: var(--ark-font-size, 14px);\n  line-height: var(--ark-line-height, 1.5);\n  white-space: pre-wrap;\n  word-wrap: break-word;\n  overflow-wrap: break-word;\n  box-sizing: border-box;\n  width: 100%;\n  min-height: 100%;\n  pointer-events: none;\n  color: transparent;\n  overflow: auto;\n}\n\n.ark-diag-marker-error {\n  text-decoration: wavy underline var(--ark-diag-error-color, #f38ba8);\n  text-decoration-skip-ink: none;\n  text-underline-offset: 2px;\n}\n\n.ark-diag-marker-warning {\n  text-decoration: wavy underline var(--ark-diag-warning-color, #f9e2af);\n  text-decoration-skip-ink: none;\n  text-underline-offset: 2px;\n}\n\n.ark-diag-marker-help {\n  text-decoration: wavy underline var(--ark-diag-help-color, #89b4fa);\n  text-decoration-skip-ink: none;\n  text-underline-offset: 2px;\n}\n\n/* ------------------------------------------------------------------ */\n/* Playground app wrapper                                              */\n/* ------------------------------------------------------------------ */\n\n.ark-playground-app {\n  display: flex;\n  flex-direction: column;\n  border-radius: var(--ark-border-radius, 8px);\n  overflow: hidden;\n}\n";
/**
 * Inject the diagnostics CSS into the document `<head>`.
 *
 * Safe to call multiple times — subsequent calls are no-ops.
 * Only works in browser environments with a `document` global.
 *
 * @param css - Optional custom CSS to inject instead of the default.
 * @returns The `<style>` element that was created (or the existing one).
 */
export declare function injectDiagnosticStyles(css?: string): HTMLStyleElement;
//# sourceMappingURL=diagnostics.d.ts.map