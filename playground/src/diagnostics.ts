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

import type { Diagnostic, Severity } from "./types.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
export function offsetToLineCol(
  source: string,
  offset: number,
): { line: number; col: number } {
  const clamped = Math.max(0, Math.min(offset, source.length));
  let line = 1;
  let col = 1;
  for (let i = 0; i < clamped; i++) {
    if (source[i] === "\n") {
      line++;
      col = 1;
    } else {
      col++;
    }
  }
  return { line, col };
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Escape HTML special characters in text.
 * @internal
 */
function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/** Severity display metadata. @internal */
interface SeverityDisplay {
  icon: string;
  cssClass: string;
  label: string;
}

const SEVERITY_DISPLAY: Record<Severity, SeverityDisplay> = {
  error: { icon: "✕", cssClass: "ark-diag-error", label: "error" },
  warning: { icon: "⚠", cssClass: "ark-diag-warning", label: "warning" },
  help: { icon: "ℹ", cssClass: "ark-diag-help", label: "help" },
};

// ---------------------------------------------------------------------------
// Diagnostics Panel
// ---------------------------------------------------------------------------

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
export function createDiagnosticsPanel(
  container: HTMLElement,
  options: DiagnosticsPanelOptions = {},
): DiagnosticsPanel {
  const { injectStyles = true } = options;

  if (injectStyles) {
    injectDiagnosticStyles();
  }

  const root = document.createElement("div");
  root.className = "ark-diagnostics-panel";
  root.setAttribute("role", "log");
  root.setAttribute("aria-label", "Diagnostics");

  const header = document.createElement("div");
  header.className = "ark-diagnostics-header";
  header.textContent = "Diagnostics";
  root.appendChild(header);

  const list = document.createElement("ul");
  list.className = "ark-diagnostics-list";
  root.appendChild(list);

  container.appendChild(root);

  let destroyed = false;

  return {
    get element(): HTMLElement {
      return root;
    },

    update(diagnostics: Diagnostic[], source: string): void {
      if (destroyed) return;
      list.innerHTML = "";

      // --- Header summary ---
      if (diagnostics.length === 0) {
        header.textContent = "Diagnostics \u2014 no issues \u2713";
        header.className = "ark-diagnostics-header ark-diagnostics-ok";
        return;
      }

      const errorCount = diagnostics.filter(
        (d) => d.severity === "error",
      ).length;
      const warnCount = diagnostics.filter(
        (d) => d.severity === "warning",
      ).length;
      const helpCount = diagnostics.length - errorCount - warnCount;

      const summaryParts: string[] = [];
      if (errorCount > 0)
        summaryParts.push(
          `${errorCount} error${errorCount !== 1 ? "s" : ""}`,
        );
      if (warnCount > 0)
        summaryParts.push(
          `${warnCount} warning${warnCount !== 1 ? "s" : ""}`,
        );
      if (helpCount > 0)
        summaryParts.push(
          `${helpCount} note${helpCount !== 1 ? "s" : ""}`,
        );

      header.textContent = `Diagnostics \u2014 ${summaryParts.join(", ")}`;
      header.className =
        errorCount > 0
          ? "ark-diagnostics-header ark-diagnostics-has-errors"
          : "ark-diagnostics-header ark-diagnostics-has-warnings";

      // --- Diagnostic items ---
      for (const diag of diagnostics) {
        const display = SEVERITY_DISPLAY[diag.severity];
        const li = document.createElement("li");
        li.className = `ark-diagnostic-item ${display.cssClass}`;

        // Severity icon
        const icon = document.createElement("span");
        icon.className = "ark-diagnostic-icon";
        icon.textContent = display.icon;
        icon.setAttribute("aria-label", display.label);
        li.appendChild(icon);

        // Location (line:col from first label)
        if (diag.labels.length > 0) {
          const loc = offsetToLineCol(source, diag.labels[0].start);
          const locSpan = document.createElement("span");
          locSpan.className = "ark-diagnostic-loc";
          locSpan.textContent = `${loc.line}:${loc.col}`;
          li.appendChild(locSpan);
        }

        // Diagnostic code
        if (diag.code) {
          const codeSpan = document.createElement("span");
          codeSpan.className = "ark-diagnostic-code";
          codeSpan.textContent = `[${diag.code}]`;
          li.appendChild(codeSpan);
        }

        // Message
        const msg = document.createElement("span");
        msg.className = "ark-diagnostic-message";
        msg.textContent = diag.message;
        li.appendChild(msg);

        // Notes
        if (diag.notes.length > 0) {
          const notesEl = document.createElement("div");
          notesEl.className = "ark-diagnostic-notes";
          for (const note of diag.notes) {
            const noteEl = document.createElement("div");
            noteEl.className = "ark-diagnostic-note";
            noteEl.textContent = `  \u21B3 ${note}`;
            notesEl.appendChild(noteEl);
          }
          li.appendChild(notesEl);
        }

        // Suggestion
        if (diag.suggestion) {
          const sugEl = document.createElement("div");
          sugEl.className = "ark-diagnostic-suggestion";
          sugEl.textContent = `\uD83D\uDCA1 ${diag.suggestion}`;
          li.appendChild(sugEl);
        }

        list.appendChild(li);
      }
    },

    clear(): void {
      if (destroyed) return;
      list.innerHTML = "";
      header.textContent = "Diagnostics";
      header.className = "ark-diagnostics-header";
    },

    destroy(): void {
      if (destroyed) return;
      destroyed = true;
      if (root.parentNode) {
        root.parentNode.removeChild(root);
      }
    },
  };
}

// ---------------------------------------------------------------------------
// Inline Diagnostic Overlay
// ---------------------------------------------------------------------------

/** Severity priority for overlapping marker ranges. @internal */
const SEVERITY_PRIORITY: Record<Severity, number> = {
  error: 3,
  warning: 2,
  help: 1,
};

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
export function buildDiagnosticOverlay(
  source: string,
  diagnostics: Diagnostic[],
): string {
  if (source.length === 0 || diagnostics.length === 0) {
    return "";
  }

  // Collect all label ranges with their diagnostic severity.
  const ranges: Array<{ start: number; end: number; severity: Severity }> = [];
  for (const diag of diagnostics) {
    for (const label of diag.labels) {
      const start = Math.max(0, label.start);
      const end = Math.min(source.length, label.end);
      if (start < end) {
        ranges.push({ start, end, severity: diag.severity });
      }
    }
  }

  if (ranges.length === 0) {
    return "";
  }

  // Build a per-character severity map.
  // For overlapping ranges, the highest-priority severity wins.
  const charSev: Array<Severity | null> = new Array<Severity | null>(
    source.length,
  ).fill(null);

  for (const range of ranges) {
    for (let i = range.start; i < range.end; i++) {
      const current = charSev[i];
      if (
        current === null ||
        SEVERITY_PRIORITY[range.severity] > SEVERITY_PRIORITY[current]
      ) {
        charSev[i] = range.severity;
      }
    }
  }

  // Build HTML from segments of consecutive same-severity characters.
  const parts: string[] = [];
  let segStart = 0;
  let segSev = charSev[0];

  for (let i = 1; i <= source.length; i++) {
    const sev = i < source.length ? charSev[i] : undefined;
    if (sev !== segSev) {
      const text = escapeHtml(source.slice(segStart, i));
      if (segSev !== null) {
        parts.push(`<span class="ark-diag-marker-${segSev}">${text}</span>`);
      } else {
        parts.push(text);
      }
      segStart = i;
      segSev = sev ?? null;
    }
  }

  const html = parts.join("");
  return html.endsWith("\n") ? html : html + "\n";
}

// ---------------------------------------------------------------------------
// Diagnostic CSS
// ---------------------------------------------------------------------------

/** CSS for the diagnostics panel and inline marker overlay. */
export const DIAGNOSTICS_CSS = /* css */ `
/* ------------------------------------------------------------------ */
/* Arukellt Playground \u2014 Diagnostics Panel                            */
/* ------------------------------------------------------------------ */

.ark-diagnostics-panel {
  --ark-diag-bg: #1e1e2e;
  --ark-diag-border: #313244;
  --ark-diag-fg: #cdd6f4;
  --ark-diag-error-color: #f38ba8;
  --ark-diag-warning-color: #f9e2af;
  --ark-diag-help-color: #89b4fa;
  --ark-diag-loc-color: #6c7086;
  --ark-diag-code-color: #9399b2;

  background: var(--ark-diag-bg);
  border-top: 1px solid var(--ark-diag-border);
  color: var(--ark-diag-fg);
  font-family: "JetBrains Mono", "Fira Code", "Cascadia Code",
    "Source Code Pro", ui-monospace, monospace;
  font-size: 13px;
  max-height: 200px;
  overflow-y: auto;
}

.ark-diagnostics-header {
  padding: 6px 12px;
  font-weight: 600;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--ark-diag-loc-color);
  border-bottom: 1px solid var(--ark-diag-border);
  position: sticky;
  top: 0;
  background: var(--ark-diag-bg);
}

.ark-diagnostics-ok {
  color: #a6e3a1;
}

.ark-diagnostics-has-errors {
  color: var(--ark-diag-error-color);
}

.ark-diagnostics-has-warnings {
  color: var(--ark-diag-warning-color);
}

.ark-diagnostics-list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.ark-diagnostic-item {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 6px;
  padding: 4px 12px;
  border-bottom: 1px solid var(--ark-diag-border);
}

.ark-diagnostic-item:last-child {
  border-bottom: none;
}

.ark-diagnostic-icon {
  flex-shrink: 0;
  width: 16px;
  text-align: center;
}

.ark-diag-error .ark-diagnostic-icon {
  color: var(--ark-diag-error-color);
}

.ark-diag-warning .ark-diagnostic-icon {
  color: var(--ark-diag-warning-color);
}

.ark-diag-help .ark-diagnostic-icon {
  color: var(--ark-diag-help-color);
}

.ark-diagnostic-loc {
  color: var(--ark-diag-loc-color);
  font-size: 12px;
  min-width: 40px;
}

.ark-diagnostic-code {
  color: var(--ark-diag-code-color);
  font-size: 12px;
}

.ark-diagnostic-message {
  flex: 1;
  min-width: 0;
}

.ark-diagnostic-notes {
  width: 100%;
  padding-left: 22px;
  color: var(--ark-diag-loc-color);
  font-size: 12px;
}

.ark-diagnostic-note {
  padding: 1px 0;
}

.ark-diagnostic-suggestion {
  width: 100%;
  padding-left: 22px;
  color: #a6e3a1;
  font-size: 12px;
}

/* ------------------------------------------------------------------ */
/* Inline diagnostic markers (overlay in the editor)                   */
/* ------------------------------------------------------------------ */

.ark-editor-diagnostics {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  margin: 0;
  padding: var(--ark-padding, 16px);
  border: none;
  font-family: var(--ark-font-family, "JetBrains Mono", ui-monospace, monospace);
  font-size: var(--ark-font-size, 14px);
  line-height: var(--ark-line-height, 1.5);
  white-space: pre-wrap;
  word-wrap: break-word;
  overflow-wrap: break-word;
  box-sizing: border-box;
  width: 100%;
  min-height: 100%;
  pointer-events: none;
  color: transparent;
  overflow: auto;
}

.ark-diag-marker-error {
  text-decoration: wavy underline var(--ark-diag-error-color, #f38ba8);
  text-decoration-skip-ink: none;
  text-underline-offset: 2px;
}

.ark-diag-marker-warning {
  text-decoration: wavy underline var(--ark-diag-warning-color, #f9e2af);
  text-decoration-skip-ink: none;
  text-underline-offset: 2px;
}

.ark-diag-marker-help {
  text-decoration: wavy underline var(--ark-diag-help-color, #89b4fa);
  text-decoration-skip-ink: none;
  text-underline-offset: 2px;
}

/* ------------------------------------------------------------------ */
/* Playground app wrapper                                              */
/* ------------------------------------------------------------------ */

.ark-playground-app {
  display: flex;
  flex-direction: column;
  border-radius: var(--ark-border-radius, 8px);
  overflow: hidden;
}
`;

/** ID for the injected diagnostic `<style>` element. */
const DIAG_STYLE_ID = "ark-diagnostics-theme";

/**
 * Inject the diagnostics CSS into the document `<head>`.
 *
 * Safe to call multiple times — subsequent calls are no-ops.
 * Only works in browser environments with a `document` global.
 *
 * @param css - Optional custom CSS to inject instead of the default.
 * @returns The `<style>` element that was created (or the existing one).
 */
export function injectDiagnosticStyles(css?: string): HTMLStyleElement {
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
  if (typeof document === "undefined") {
    throw new Error("injectDiagnosticStyles() requires a browser environment");
  }

  const existing = document.getElementById(DIAG_STYLE_ID);
  if (existing) {
    return existing as HTMLStyleElement;
  }

  const style = document.createElement("style");
  style.id = DIAG_STYLE_ID;
  style.textContent = css ?? DIAGNOSTICS_CSS;
  document.head.appendChild(style);
  return style;
}
