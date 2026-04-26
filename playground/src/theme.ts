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
export const DEFAULT_THEME_CSS = /* css */ `
/* ------------------------------------------------------------------ */
/* Arukellt Playground Editor — Default Dark Theme                     */
/* ------------------------------------------------------------------ */

.ark-editor {
  /* Customisable colour tokens */
  --ark-bg: #1e1e2e;
  --ark-fg: #cdd6f4;
  --ark-caret: #f5e0dc;
  --ark-selection: rgba(137, 180, 250, 0.25);
  --ark-line-height: 1.5;
  --ark-font-family: "JetBrains Mono", "Fira Code", "Cascadia Code",
    "Source Code Pro", ui-monospace, monospace;
  --ark-font-size: 14px;
  --ark-padding: 16px;
  --ark-border-radius: 8px;

  /* Highlight colour tokens */
  --ark-hl-keyword: #cba6f7;
  --ark-hl-string: #a6e3a1;
  --ark-hl-number: #fab387;
  --ark-hl-comment: #6c7086;
  --ark-hl-operator: #89dceb;
  --ark-hl-punctuation: #9399b2;
  --ark-hl-identifier: #cdd6f4;
  --ark-hl-boolean: #fab387;
  --ark-hl-type: #89b4fa;
}

/* ------------------------------------------------------------------ */
/* Editor layout                                                       */
/* ------------------------------------------------------------------ */

.ark-editor {
  position: relative;
  background: var(--ark-bg);
  border-radius: var(--ark-border-radius);
  overflow: hidden;
}

.ark-editor-backdrop,
.ark-editor-textarea {
  margin: 0;
  padding: var(--ark-padding);
  border: none;
  font-family: var(--ark-font-family);
  font-size: var(--ark-font-size);
  line-height: var(--ark-line-height);
  tab-size: 4;
  white-space: pre-wrap;
  word-wrap: break-word;
  overflow-wrap: break-word;
  box-sizing: border-box;
  width: 100%;
  min-height: 100%;
}

.ark-editor-backdrop {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  color: var(--ark-fg);
  pointer-events: none;
  overflow: auto;
}

.ark-editor-textarea {
  position: relative;
  z-index: 1;
  display: block;
  resize: vertical;
  background: transparent;
  color: transparent;
  caret-color: var(--ark-caret);
  outline: none;
  min-height: 200px;
}

.ark-editor-textarea::selection {
  background: var(--ark-selection);
  color: transparent;
}

/* ------------------------------------------------------------------ */
/* Highlight classes                                                    */
/* ------------------------------------------------------------------ */

.ark-hl-keyword {
  color: var(--ark-hl-keyword);
  font-weight: 600;
}

.ark-hl-string {
  color: var(--ark-hl-string);
}

.ark-hl-number {
  color: var(--ark-hl-number);
}

.ark-hl-comment {
  color: var(--ark-hl-comment);
  font-style: italic;
}

.ark-hl-operator {
  color: var(--ark-hl-operator);
}

.ark-hl-punctuation {
  color: var(--ark-hl-punctuation);
}

.ark-hl-identifier {
  color: var(--ark-hl-identifier);
}

.ark-hl-boolean {
  color: var(--ark-hl-boolean);
  font-weight: 600;
}

.ark-hl-type {
  color: var(--ark-hl-type);
}
`;

/** ID of the injected `<style>` element. */
const STYLE_ELEMENT_ID = "ark-editor-theme";

/**
 * Inject the default theme CSS into the document `<head>`.
 *
 * Safe to call multiple times — subsequent calls are no-ops.
 * Only works in browser environments with a `document` global.
 *
 * @param css - Optional custom CSS to inject instead of the default theme.
 * @returns The `<style>` element that was created (or the existing one).
 */
export function injectTheme(css?: string): HTMLStyleElement {
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
  if (typeof document === "undefined") {
    throw new Error("injectTheme() requires a browser environment");
  }

  const existing = document.getElementById(STYLE_ELEMENT_ID);
  if (existing) {
    return existing as HTMLStyleElement;
  }

  const style = document.createElement("style");
  style.id = STYLE_ELEMENT_ID;
  style.textContent = css ?? DEFAULT_THEME_CSS;
  document.head.appendChild(style);
  return style;
}
