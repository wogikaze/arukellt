/**
 * Default syntax-highlighting theme for the Arukellt playground editor.
 *
 * Provides CSS custom properties for theming and a default dark theme.
 * Consumers can override the custom properties or supply their own
 * stylesheet entirely.
 *
 * @module
 */
/**
 * Default dark theme CSS for the playground editor.
 *
 * Uses CSS custom properties on `.ark-editor` so consumers can override
 * individual colours without replacing the entire stylesheet.
 *
 * @example
 * ```ts
 * import { DEFAULT_THEME_CSS, injectTheme } from "@arukellt/playground";
 * injectTheme(); // injects into document.head
 * ```
 */
export declare const DEFAULT_THEME_CSS = "\n/* ------------------------------------------------------------------ */\n/* Arukellt Playground Editor \u2014 Default Dark Theme                     */\n/* ------------------------------------------------------------------ */\n\n.ark-editor {\n  /* Customisable colour tokens */\n  --ark-bg: #1e1e2e;\n  --ark-fg: #cdd6f4;\n  --ark-caret: #f5e0dc;\n  --ark-selection: rgba(137, 180, 250, 0.25);\n  --ark-line-height: 1.5;\n  --ark-font-family: \"JetBrains Mono\", \"Fira Code\", \"Cascadia Code\",\n    \"Source Code Pro\", ui-monospace, monospace;\n  --ark-font-size: 14px;\n  --ark-padding: 16px;\n  --ark-border-radius: 8px;\n\n  /* Highlight colour tokens */\n  --ark-hl-keyword: #cba6f7;\n  --ark-hl-string: #a6e3a1;\n  --ark-hl-number: #fab387;\n  --ark-hl-comment: #6c7086;\n  --ark-hl-operator: #89dceb;\n  --ark-hl-punctuation: #9399b2;\n  --ark-hl-identifier: #cdd6f4;\n  --ark-hl-boolean: #fab387;\n  --ark-hl-type: #89b4fa;\n}\n\n/* ------------------------------------------------------------------ */\n/* Editor layout                                                       */\n/* ------------------------------------------------------------------ */\n\n.ark-editor {\n  position: relative;\n  background: var(--ark-bg);\n  border-radius: var(--ark-border-radius);\n  overflow: hidden;\n}\n\n.ark-editor-backdrop,\n.ark-editor-textarea {\n  margin: 0;\n  padding: var(--ark-padding);\n  border: none;\n  font-family: var(--ark-font-family);\n  font-size: var(--ark-font-size);\n  line-height: var(--ark-line-height);\n  tab-size: 4;\n  white-space: pre-wrap;\n  word-wrap: break-word;\n  overflow-wrap: break-word;\n  box-sizing: border-box;\n  width: 100%;\n  min-height: 100%;\n}\n\n.ark-editor-backdrop {\n  position: absolute;\n  top: 0;\n  left: 0;\n  right: 0;\n  bottom: 0;\n  color: var(--ark-fg);\n  pointer-events: none;\n  overflow: auto;\n}\n\n.ark-editor-textarea {\n  position: relative;\n  z-index: 1;\n  display: block;\n  resize: vertical;\n  background: transparent;\n  color: transparent;\n  caret-color: var(--ark-caret);\n  outline: none;\n  min-height: 200px;\n}\n\n.ark-editor-textarea::selection {\n  background: var(--ark-selection);\n  color: transparent;\n}\n\n/* ------------------------------------------------------------------ */\n/* Highlight classes                                                    */\n/* ------------------------------------------------------------------ */\n\n.ark-hl-keyword {\n  color: var(--ark-hl-keyword);\n  font-weight: 600;\n}\n\n.ark-hl-string {\n  color: var(--ark-hl-string);\n}\n\n.ark-hl-number {\n  color: var(--ark-hl-number);\n}\n\n.ark-hl-comment {\n  color: var(--ark-hl-comment);\n  font-style: italic;\n}\n\n.ark-hl-operator {\n  color: var(--ark-hl-operator);\n}\n\n.ark-hl-punctuation {\n  color: var(--ark-hl-punctuation);\n}\n\n.ark-hl-identifier {\n  color: var(--ark-hl-identifier);\n}\n\n.ark-hl-boolean {\n  color: var(--ark-hl-boolean);\n  font-weight: 600;\n}\n\n.ark-hl-type {\n  color: var(--ark-hl-type);\n}\n";
/**
 * Inject the default theme CSS into the document `<head>`.
 *
 * Safe to call multiple times — subsequent calls are no-ops.
 * Only works in browser environments with a `document` global.
 *
 * @param css - Optional custom CSS to inject instead of the default theme.
 * @returns The `<style>` element that was created (or the existing one).
 */
export declare function injectTheme(css?: string): HTMLStyleElement;
//# sourceMappingURL=theme.d.ts.map